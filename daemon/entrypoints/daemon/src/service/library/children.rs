use super::{LibraryBrowser, LibraryRelativePath};
use crate::{
    prelude::*,
    service::listing::{ListingInvalidation, SnapshotVersion, SummaryKey},
};
use omnius_pxna_engine::service::ChildProbeVersion;
use omnius_pxna_interface::{models, types::Nullable};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub struct ChildProbeCache {
    entries: Mutex<HashMap<SummaryKey, CachedProbe>>,
    epoch: AtomicU64,
}
struct CachedProbe {
    version: String,
    has_children: bool,
    expires: Instant,
}
impl ChildProbeCache {
    pub fn invalidate(&self, invalidation: &ListingInvalidation) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
        let libraries = invalidation.entries();
        self.entries.lock().retain(|key, _| !libraries.contains_key(key.library()));
    }
    fn get(&self, key: &SummaryKey, version: &str) -> Option<models::FolderChildrenProbe> {
        let now = Instant::now();
        let mut entries = self.entries.lock();
        entries.retain(|_, value| value.expires > now);
        let entry = entries.get(key)?;
        if entry.version != version {
            return None;
        }
        let mut response = models::FolderChildrenProbe::new(
            entry.expires.saturating_duration_since(now).as_millis().min(30000) as u32,
            Nullable::Present(entry.has_children),
        );
        response.source_version = Some(version.to_owned());
        Some(response)
    }
    fn insert(&self, key: SummaryKey, version: String, value: bool, epoch: u64) {
        let mut entries = self.entries.lock();
        if self.epoch.load(Ordering::SeqCst) != epoch {
            return;
        }
        if entries.len() >= 4096
            && !entries.contains_key(&key)
            && let Some(oldest) = entries.iter().min_by_key(|(_, value)| value.expires).map(|(key, _)| key.clone())
        {
            entries.remove(&oldest);
        }
        entries.insert(
            key,
            CachedProbe {
                version,
                has_children: value,
                expires: Instant::now() + Duration::from_secs(30),
            },
        );
    }
}
impl LibraryBrowser {
    pub async fn probe_folder_children(&self, library_id: &str, path: &str, include_hidden: bool) -> Result<models::FolderChildrenProbe> {
        let library = self.library(library_id)?;
        let relative = LibraryRelativePath::parse(path)?;
        let key = SummaryKey::for_probe(library_id, &library.path, path, include_hidden)?;
        let storage = self.storage.clone();
        let root = library.path.clone();
        let summaries = self.snapshots.summaries().clone();
        let cache = self.snapshots.child_probes().clone();
        let epoch = cache.epoch.load(Ordering::SeqCst);
        Ok(self
            .run_child_probe(move |deadline, cancellation| {
                let target = storage.child_probe_target(&root, relative.segments(), deadline, &cancellation)?;
                if cancellation.is_cancelled() || Instant::now() >= deadline {
                    return None;
                }
                let version = target.source_version();
                if let Some(hit) = cache.get(&key, &version) {
                    return Some(hit);
                }
                let snapshot_version = match target.version() {
                    ChildProbeVersion::Local(modified) => SnapshotVersion::Local(Some(*modified)),
                    ChildProbeVersion::Archive(identity) => SnapshotVersion::Archive(*identity),
                };
                let result = summaries
                    .cached_folder_children(&key, Some(snapshot_version))
                    .or_else(|| target.has_children(include_hidden, deadline, &cancellation))?;
                if cancellation.is_cancelled() || Instant::now() >= deadline || !target.unchanged() || cache.epoch.load(Ordering::SeqCst) != epoch {
                    return None;
                }
                cache.insert(key.clone(), version.clone(), result, epoch);
                cache.get(&key, &version)
            })
            .await)
    }
    async fn run_child_probe<F>(&self, task: F) -> models::FolderChildrenProbe
    where
        F: FnOnce(Instant, CancellationToken) -> Option<models::FolderChildrenProbe> + Send + 'static,
    {
        let unknown = || models::FolderChildrenProbe::new(0, Nullable::Null);
        let deadline = Instant::now() + Duration::from_millis(50);
        let Some((worker, stream)) = self.storage.work_budget().try_probe() else {
            return unknown();
        };
        let cancellation = CancellationToken::new();
        let _cancel_on_drop = cancellation.clone().drop_guard();
        let job = worker.run_blocking_with(stream, move || task(deadline, cancellation));
        let remaining = deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(remaining, job).await {
            Ok(Ok(Some(response))) => response,
            _ => unknown(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::LibraryConfig, service::listing::ListingSnapshots};
    use omnius_core_base::{clock::ClockUtc, tsid::TsidProviderImpl};
    use omnius_pxna_engine::service::{ArchiveRuntime, LocalStorageDriver, StorageService, WorkBudget, WorkClass, WorkKind};
    use rand::{SeedableRng as _, rngs::ChaCha20Rng};
    use std::{path::Path, sync::Arc, time::SystemTime};
    use testresult::TestResult;

    pub(super) fn browser(root: &Path) -> LibraryBrowser {
        let budget = Arc::new(WorkBudget::default());
        let tsid = Arc::new(Mutex::new(TsidProviderImpl::new(ClockUtc, ChaCha20Rng::seed_from_u64(0), 16)));
        let storage = Arc::new(StorageService::new(
            Arc::new(LocalStorageDriver::with_budget(tsid, budget.clone())),
            Arc::new(ArchiveRuntime::default().with_budget(budget)),
        ));
        LibraryBrowser::new(
            &[LibraryConfig {
                id: "test".into(),
                name: "Test".into(),
                path: root.into(),
                readonly: false,
            }],
            storage,
            4,
            ListingSnapshots::new(Default::default(), Arc::new(ClockUtc)),
        )
    }
    pub(super) fn aged(path: &Path) {
        let modified = SystemTime::now() - Duration::from_secs(10);
        std::fs::File::open(path).unwrap().set_times(std::fs::FileTimes::new().set_modified(modified)).unwrap();
    }
    async fn value(browser: &LibraryBrowser, path: &str, hidden: bool) -> Nullable<bool> {
        browser.probe_folder_children("test", path, hidden).await.unwrap().has_children
    }
    #[tokio::test]
    async fn shallow_probe_files_empty_hidden_archive_link_and_limit_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let browser = browser(dir.path());
        for name in ["empty", "files", "folder", "archive", "hidden", "many", "link"] {
            std::fs::create_dir(dir.path().join(name))?;
        }
        std::fs::write(dir.path().join("files/a.txt"), b"x")?;
        std::fs::create_dir(dir.path().join("folder/child"))?;
        std::fs::write(dir.path().join("archive/a.zip"), b"not opened")?;
        std::fs::create_dir(dir.path().join("hidden/.child"))?;
        for i in 0..129 {
            std::fs::write(dir.path().join(format!("many/{i}.txt")), b"")?;
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(dir.path().join("folder"), dir.path().join("link/child"))?;
        for name in ["empty", "files", "folder", "archive", "hidden", "many", "link"] {
            aged(&dir.path().join(name));
        }
        assert_eq!(value(&browser, "/empty", false).await, Nullable::Present(false));
        assert_eq!(value(&browser, "/files", false).await, Nullable::Present(false));
        assert_eq!(value(&browser, "/folder", false).await, Nullable::Present(true));
        assert_eq!(value(&browser, "/archive", false).await, Nullable::Present(true));
        assert_eq!(value(&browser, "/hidden", false).await, Nullable::Present(false));
        assert_eq!(value(&browser, "/hidden", true).await, Nullable::Present(true));
        assert_eq!(value(&browser, "/many", false).await, Nullable::Null);
        assert_eq!(value(&browser, "/link", false).await, Nullable::Present(false));
        assert_eq!(value(&browser, "/absent", false).await, Nullable::Null);
        assert!(browser.probe_folder_children("test", "/../escape", false).await.is_err());
        assert!(browser.probe_folder_children("missing", "/", false).await.is_err());
        Ok(())
    }
    #[tokio::test]
    async fn version_ttl_mutation_and_recent_updates_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let browser = browser(dir.path());
        assert_eq!(value(&browser, "/", false).await, Nullable::Null);
        aged(dir.path());
        let result = browser.probe_folder_children("test", "/", false).await?;
        assert_eq!(result.has_children, Nullable::Present(false));
        let key = SummaryKey::for_probe("test", dir.path(), "/", false)?;
        let cache = browser.snapshots.child_probes();
        cache.entries.lock().get_mut(&key).unwrap().expires = Instant::now() + Duration::from_secs(5);
        let reused = browser.probe_folder_children("test", "/", false).await?;
        assert!(reused.max_age_ms <= 5000);
        assert_eq!(reused.source_version, result.source_version);
        std::fs::create_dir(dir.path().join("new"))?;
        assert_eq!(value(&browser, "/", false).await, Nullable::Null);
        aged(dir.path());
        assert_eq!(value(&browser, "/", false).await, Nullable::Present(true));
        let epoch = cache.epoch.load(Ordering::SeqCst);
        browser.snapshots.invalidate(&ListingInvalidation::for_entry("test", "/new")?).await;
        assert!(cache.entries.lock().is_empty());
        cache.insert(key.clone(), "old".into(), false, epoch);
        assert!(cache.entries.lock().is_empty());
        cache.insert(key.clone(), "current".into(), true, cache.epoch.load(Ordering::SeqCst));
        cache.entries.lock().get_mut(&key).unwrap().expires = Instant::now();
        assert!(cache.get(&key, "current").is_none());
        Ok(())
    }
    #[tokio::test]
    async fn probe_timeout_keeps_worker_until_real_exit_and_cancels_next_work_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let browser = browser(dir.path());
        let budget = browser.storage.work_budget().clone();
        let (release, wait) = std::sync::mpsc::channel();
        let (started, ready) = tokio::sync::oneshot::channel();
        let (stopped, ended) = tokio::sync::oneshot::channel();
        let probe = browser.run_child_probe(move |_, cancellation| {
            started.send(()).unwrap();
            wait.recv().unwrap();
            assert!(cancellation.is_cancelled());
            stopped.send(()).unwrap();
            None
        });
        tokio::pin!(probe);
        tokio::select! { _ = ready => {}, _ = &mut probe => panic!("worker must start") }
        assert_eq!(probe.await.has_children, Nullable::Null);
        assert!(budget.try_probe().is_none());
        let foreground = budget.acquire(WorkClass::Foreground, WorkKind::Listing).await?;
        release.send(())?;
        ended.await?;
        drop(foreground);
        let _finished = budget.acquire(WorkClass::Background, WorkKind::Exif).await?;
        assert_eq!(budget.counts(WorkKind::ChildProbe), (1, 1));
        Ok(())
    }
}

#[cfg(test)]
mod archive_tests {
    use super::tests::{aged, browser};
    use super::*;
    use testresult::TestResult;
    #[tokio::test]
    async fn probe_uses_cached_archive_and_fresh_summary_without_new_inspection_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let archive = dir.path().join("pack.7z");
        let mut writer = sevenz_rust2::ArchiveWriter::create(&archive)?;
        writer.push_archive_entry(sevenz_rust2::ArchiveEntry::new_file("dir/file.txt"), Some(std::io::Cursor::new(b"x".to_vec())))?;
        writer.finish()?;
        let browser = browser(dir.path());
        assert_eq!(browser.probe_folder_children("test", "/pack.7z", false).await?.has_children, Nullable::Null);
        // An ordinary user open populates the archive cache; probes never open it themselves.
        browser.get_folder("test", "/pack.7z").await?;
        assert_eq!(browser.probe_folder_children("test", "/pack.7z", false).await?.has_children, Nullable::Present(true));
        assert_eq!(browser.probe_folder_children("test", "/pack.7z/dir", false).await?.has_children, Nullable::Present(false));
        std::fs::create_dir(dir.path().join("large"))?;
        for i in 0..129 {
            std::fs::write(dir.path().join(format!("large/{i}.txt")), b"")?;
        }
        aged(&dir.path().join("large"));
        assert_eq!(browser.probe_folder_children("test", "/large", false).await?.has_children, Nullable::Null);
        let options = crate::service::listing::SummaryOptions::from_query(&serde_json::from_value(serde_json::json!({"path":"/large"}))?)?;
        browser.list_folder_summaries("test", "/large", &options).await?;
        assert_eq!(browser.probe_folder_children("test", "/large", false).await?.has_children, Nullable::Present(false));
        Ok(())
    }
}
