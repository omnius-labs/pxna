use std::{collections::HashMap, sync::Arc};

use chrono::{Duration, Utc};
use omnius_core_base::tsid::TsidProvider;
use omnius_pxna_engine::service::{ConflictPolicy, StagingFile};
use omnius_pxna_interface::models;

use crate::{prelude::*, service::mutation::LibraryMutation};

struct UploadSession {
    library_id: String,
    model: models::Upload,
    conflict: ConflictPolicy,
    staging: Option<StagingFile>,
}

/// チャンク間で同じ staging を保持し、再開位置と公開のタイミングを管理する。
pub struct UploadService {
    mutation: Arc<LibraryMutation>,
    ids: Arc<Mutex<dyn TsidProvider + Send + Sync>>,
    sessions: Mutex<HashMap<String, Arc<TokioMutex<UploadSession>>>>,
}

impl UploadService {
    pub fn new(mutation: Arc<LibraryMutation>, ids: Arc<Mutex<dyn TsidProvider + Send + Sync>>) -> Arc<Self> {
        let service = Arc::new(Self {
            mutation,
            ids,
            sessions: Mutex::new(HashMap::new()),
        });
        let weak = Arc::downgrade(&service);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let Some(service) = weak.upgrade() else {
                    break;
                };
                service.reap().await;
            }
        });
        service
    }

    pub async fn create(&self, library_id: &str, body: &models::UploadCreateRequest) -> Result<models::Upload> {
        let conflict = LibraryMutation::conflict(body.conflict);
        let (dst, staging) = self.mutation.staging(library_id, &body.path, conflict).await?;
        let id = format!("upl_{}", self.ids.lock().create());
        let model = models::Upload::new(
            id.clone(),
            self.mutation.relative(library_id, &dst)?,
            body.size,
            0,
            models::UploadStatus::Active,
            Utc::now() + Duration::hours(24),
        );
        self.sessions.lock().insert(
            id,
            Arc::new(TokioMutex::new(UploadSession {
                library_id: library_id.to_string(),
                model: model.clone(),
                conflict,
                staging: Some(staging),
            })),
        );
        Ok(model)
    }

    pub async fn get(&self, library_id: &str, id: &str) -> Result<models::Upload> {
        let session = self.session(id)?;
        let mut session = session.lock().await;
        Self::validate(&mut session, library_id)?;
        Ok(session.model.clone())
    }

    pub async fn write(&self, library_id: &str, id: &str, offset: u64, bytes: &[u8]) -> Result<()> {
        let session = self.session(id)?;
        let mut session = session.lock().await;
        Self::validate(&mut session, library_id)?;
        Self::active(&session)?;
        if offset != session.model.committed_size {
            return Err(Error::new(ErrorKind::AlreadyExists).with_message("offset does not match committedSize"));
        }
        if bytes.len() as u64 > session.model.size.saturating_sub(offset) {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("chunk exceeds the declared size"));
        }
        let result = session.staging.as_mut().ok_or_else(|| Error::new(ErrorKind::AlreadyExists))?.write_all(bytes).await;
        if let Err(error) = result {
            // write_all が一部だけ書いた可能性があるため、この staging へ再追記させない。
            session.staging.take();
            session.model.status = models::UploadStatus::Aborted;
            return Err(error.into());
        }
        session.model.committed_size += bytes.len() as u64;
        Ok(())
    }

    pub async fn commit(&self, library_id: &str, id: &str) -> Result<models::FileSystemEntry> {
        let session = self.session(id)?;
        let mut session = session.lock().await;
        Self::validate(&mut session, library_id)?;
        Self::active(&session)?;
        if session.model.committed_size != session.model.size {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("committedSize does not match size"));
        }
        let staging = session.staging.take().ok_or_else(|| Error::new(ErrorKind::AlreadyExists))?;
        let result = self.mutation.publish(library_id, &session.model.path, staging, session.conflict).await;
        session.model.status = if result.is_ok() {
            models::UploadStatus::Committed
        } else {
            models::UploadStatus::Aborted
        };
        result
    }

    pub async fn abort(&self, library_id: &str, id: &str) -> Result<()> {
        let session = self.session(id)?;
        let mut session = session.lock().await;
        Self::validate(&mut session, library_id)?;
        if session.model.status == models::UploadStatus::Active {
            session.staging.take();
            session.model.status = models::UploadStatus::Aborted;
        }
        Ok(())
    }

    async fn reap(&self) {
        let sessions = self.sessions.lock().iter().map(|(id, session)| (id.clone(), session.clone())).collect::<Vec<_>>();
        for (id, session) in sessions {
            let mut session = session.lock().await;
            if session.model.expires_at <= Utc::now() {
                session.staging.take();
                session.model.status = models::UploadStatus::Expired;
                self.sessions.lock().remove(&id);
            }
        }
    }

    fn session(&self, id: &str) -> Result<Arc<TokioMutex<UploadSession>>> {
        self.sessions
            .lock()
            .get(id)
            .cloned()
            .ok_or_else(|| Error::new(ErrorKind::NotFound).with_message("upload not found"))
    }

    fn validate(session: &mut UploadSession, library_id: &str) -> Result<()> {
        if session.library_id != library_id {
            return Err(Error::new(ErrorKind::NotFound).with_message("upload not found"));
        }
        if session.model.expires_at <= Utc::now() {
            session.staging.take();
            session.model.status = models::UploadStatus::Expired;
            return Err(Error::new(ErrorKind::NotFound).with_message("upload expired"));
        }
        Ok(())
    }

    fn active(session: &UploadSession) -> Result<()> {
        if session.model.status != models::UploadStatus::Active {
            return Err(Error::new(ErrorKind::AlreadyExists).with_message("upload is not active"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testresult::TestResult;

    #[tokio::test]
    async fn offset_race_and_expiry_discard_staging_test() -> TestResult {
        let (_dir, state) = crate::service::mutation::tests::state().await?;
        let request = models::UploadCreateRequest::new("/file.bin".to_string(), 4);
        let model = state.uploads.create("test", &request).await?;
        let (a, b) = tokio::join!(state.uploads.write("test", &model.id, 0, b"ab"), state.uploads.write("test", &model.id, 0, b"cd"));
        assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
        assert_eq!(state.uploads.get("test", &model.id).await?.committed_size, 2);
        let session = state.uploads.session(&model.id)?;
        let staging_path = {
            let mut session = session.lock().await;
            session.model.expires_at = Utc::now() - Duration::seconds(1);
            session.staging.as_ref().ok_or("no staging")?.path().to_path_buf()
        };
        assert!(staging_path.exists());
        assert_eq!(state.uploads.get("test", &model.id).await.unwrap_err().kind(), &ErrorKind::NotFound);
        assert!(!staging_path.exists());
        state.uploads.reap().await;
        assert!(state.uploads.session(&model.id).is_err());
        Ok(())
    }
}
