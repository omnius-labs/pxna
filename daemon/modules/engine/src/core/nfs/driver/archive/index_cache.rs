use std::{
    collections::{BTreeMap, HashMap},
    future::Future,
    sync::Arc,
};

use crate::prelude::*;

use super::identity::ArchiveIdentity;
use crate::base::util::SingleFlight;

/// cache が保持する値の概算 heap size。
pub(super) trait EstimatedSize {
    fn estimated_size(&self) -> u64;
}

/// negative cache の 1 件として数える概算 size。
const NEGATIVE_ENTRY_SIZE: u64 = 256;

/// archive の inspect 結果を `ArchiveIdentity` ごとに保持する process 内 LRU cache。
///
/// 容量超過では古い順に `Arc` を外すだけで、driver が保持している値は生き続ける。
/// archive として開けないことが確定した結果は negative cache として保持し、同じ identity を
/// 何度開いても inspect を繰り返さない。
pub(super) struct IndexCache<V> {
    capacity_bytes: u64,
    state: Mutex<IndexCacheState<V>>,
    flights: SingleFlight<ArchiveIdentity, V>,
}

impl<V: EstimatedSize + Send + Sync + 'static> IndexCache<V> {
    pub fn new(capacity_bytes: u64) -> Self {
        Self {
            capacity_bytes,
            state: Mutex::new(IndexCacheState::new()),
            flights: SingleFlight::new(),
        }
    }

    pub fn peek(&self, identity: &ArchiveIdentity) -> Option<Arc<V>> {
        self.state.lock().get(identity)?.into_result().ok()
    }

    /// cache に無ければ `inspect` を走らせる。同じ identity への同時 miss は 1 回の inspect に畳む。
    pub async fn get_or_inspect<F, Fut>(&self, identity: ArchiveIdentity, inspect: F) -> Result<Arc<V>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<V>> + Send + 'static,
    {
        if let Some(cached) = self.state.lock().get(&identity) {
            return cached.into_result();
        }

        match self.flights.run(identity, inspect()).await {
            Ok(value) => {
                let size = value.estimated_size();
                if size <= self.capacity_bytes {
                    self.state.lock().insert(identity, CachedIndex::Inspected(value.clone()), size, self.capacity_bytes);
                }
                Ok(value)
            }
            Err(error) => {
                if Self::is_deterministic(error.kind()) {
                    let cached = CachedIndex::Failed {
                        kind: error.kind().clone(),
                        message: error.message().map(str::to_string),
                    };
                    self.state.lock().insert(identity, cached, NEGATIVE_ENTRY_SIZE, self.capacity_bytes);
                }
                Err(error)
            }
        }
    }

    /// 同じ入力に対して同じ結果になる失敗だけを negative cache する。
    ///
    /// I/O error、timeout、cancellation、source change は再試行で結果が変わりうる。
    fn is_deterministic(kind: &ErrorKind) -> bool {
        matches!(
            kind,
            ErrorKind::UnsupportedType | ErrorKind::InvalidFormat | ErrorKind::PasswordRequired | ErrorKind::LimitExceeded
        )
    }
}

enum CachedIndex<V> {
    Inspected(Arc<V>),
    Failed { kind: ErrorKind, message: Option<String> },
}

impl<V> Clone for CachedIndex<V> {
    fn clone(&self) -> Self {
        match self {
            CachedIndex::Inspected(value) => CachedIndex::Inspected(value.clone()),
            CachedIndex::Failed { kind, message } => CachedIndex::Failed {
                kind: kind.clone(),
                message: message.clone(),
            },
        }
    }
}

impl<V> CachedIndex<V> {
    fn into_result(self) -> Result<Arc<V>> {
        match self {
            CachedIndex::Inspected(value) => Ok(value),
            CachedIndex::Failed { kind, message } => {
                let error = Error::new(kind);
                Err(match message {
                    Some(message) => error.with_message(message),
                    None => error,
                })
            }
        }
    }
}

struct IndexCacheEntry<V> {
    value: CachedIndex<V>,
    size: u64,
    tick: u64,
}

struct IndexCacheState<V> {
    entries: HashMap<ArchiveIdentity, IndexCacheEntry<V>>,
    order: BTreeMap<u64, ArchiveIdentity>,
    used_bytes: u64,
    next_tick: u64,
}

impl<V> IndexCacheState<V> {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            order: BTreeMap::new(),
            used_bytes: 0,
            next_tick: 0,
        }
    }

    fn get(&mut self, identity: &ArchiveIdentity) -> Option<CachedIndex<V>> {
        let tick = self.tick();
        let entry = self.entries.get_mut(identity)?;
        self.order.remove(&entry.tick);
        entry.tick = tick;
        self.order.insert(tick, *identity);
        Some(entry.value.clone())
    }

    fn insert(&mut self, identity: ArchiveIdentity, value: CachedIndex<V>, size: u64, capacity_bytes: u64) {
        if let Some(previous) = self.entries.remove(&identity) {
            self.order.remove(&previous.tick);
            self.used_bytes -= previous.size;
        }
        while self.used_bytes + size > capacity_bytes {
            let Some((_, oldest)) = self.order.pop_first() else {
                break;
            };
            if let Some(evicted) = self.entries.remove(&oldest) {
                self.used_bytes -= evicted.size;
            }
        }
        if self.used_bytes + size > capacity_bytes {
            return;
        }

        let tick = self.tick();
        self.order.insert(tick, identity);
        self.entries.insert(identity, IndexCacheEntry { value, size, tick });
        self.used_bytes += size;
    }

    fn tick(&mut self) -> u64 {
        self.next_tick += 1;
        self.next_tick
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };

    use super::*;
    use crate::core::nfs::SourceFingerprint;

    #[derive(Debug)]
    struct Sized(u64);

    impl EstimatedSize for Sized {
        fn estimated_size(&self) -> u64 {
            self.0
        }
    }

    fn identity(name: &str) -> ArchiveIdentity {
        ArchiveIdentity::from_local(name, &SourceFingerprint { size: 0, mtime_s: 0, mtime_ns: 0 })
    }

    async fn open(cache: &IndexCache<Sized>, name: &str, calls: &Arc<AtomicUsize>, result: fn() -> Result<Sized>) -> Result<Arc<Sized>> {
        let calls = calls.clone();
        cache
            .get_or_inspect(identity(name), move || async move {
                calls.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(20)).await;
                result()
            })
            .await
    }

    #[tokio::test]
    async fn hit_does_not_inspect_again_test() {
        let cache = IndexCache::new(100);
        let calls = Arc::new(AtomicUsize::new(0));
        open(&cache, "a", &calls, || Ok(Sized(10))).await.unwrap();
        open(&cache, "a", &calls, || Ok(Sized(10))).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn concurrent_miss_is_single_flight_test() {
        let cache = Arc::new(IndexCache::new(100));
        let calls = Arc::new(AtomicUsize::new(0));
        let (a, b) = tokio::join!(open(&cache, "a", &calls, || Ok(Sized(10))), open(&cache, "a", &calls, || Ok(Sized(10))));
        assert!(Arc::ptr_eq(&a.unwrap(), &b.unwrap()));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn deterministic_failure_is_negative_cached_test() {
        let cache = IndexCache::new(1024);
        let calls = Arc::new(AtomicUsize::new(0));
        let broken = || Err(Error::new(ErrorKind::InvalidFormat).with_message("broken"));
        assert_eq!(open(&cache, "a", &calls, broken).await.unwrap_err().kind(), &ErrorKind::InvalidFormat);
        let error = open(&cache, "a", &calls, broken).await.unwrap_err();
        assert_eq!(error.kind(), &ErrorKind::InvalidFormat);
        assert_eq!(error.message(), Some("broken"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn io_failure_is_not_cached_test() {
        let cache = IndexCache::new(1024);
        let calls = Arc::new(AtomicUsize::new(0));
        let io_error = || Err(Error::new(ErrorKind::IoError));
        open(&cache, "a", &calls, io_error).await.unwrap_err();
        open(&cache, "a", &calls, io_error).await.unwrap_err();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn evicts_least_recently_used_test() {
        let cache = IndexCache::new(20);
        let calls = Arc::new(AtomicUsize::new(0));
        open(&cache, "a", &calls, || Ok(Sized(10))).await.unwrap();
        open(&cache, "b", &calls, || Ok(Sized(10))).await.unwrap();
        open(&cache, "a", &calls, || Ok(Sized(10))).await.unwrap();
        open(&cache, "c", &calls, || Ok(Sized(10))).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 3);

        open(&cache, "a", &calls, || Ok(Sized(10))).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 3, "最近使った a は残る");
        open(&cache, "b", &calls, || Ok(Sized(10))).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 4, "最も古い b が外れている");
    }

    #[tokio::test]
    async fn oversized_value_is_not_cached_test() {
        let cache = IndexCache::new(5);
        let calls = Arc::new(AtomicUsize::new(0));
        open(&cache, "a", &calls, || Ok(Sized(10))).await.unwrap();
        open(&cache, "a", &calls, || Ok(Sized(10))).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
