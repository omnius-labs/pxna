use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_interface::{apis, models};

use crate::{
    error::{ApiError, ApiStatus},
    interface::{ApiServer, session::Claims},
};

#[allow(unused_variables)]
#[async_trait]
impl apis::operations::Operations for ApiServer {
    type Claims = Claims;

    async fn cancel_operation(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CancelOperationPathParams,
    ) -> std::result::Result<apis::operations::CancelOperationResponse, ()> {
        Ok(match self.state.operations.cancel(&path_params.operation_id) {
            Ok(operation) => apis::operations::CancelOperationResponse::Status202_CancellationRequested(models::OperationResponse::new(operation)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn get_operation(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetOperationPathParams,
    ) -> std::result::Result<apis::operations::GetOperationResponse, ()> {
        Ok(match self.state.operations.get(&path_params.operation_id) {
            Ok(operation) => apis::operations::GetOperationResponse::Status200_OperationState(models::OperationResponse::new(operation)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn list_operations(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> std::result::Result<apis::operations::ListOperationsResponse, ()> {
        Ok(apis::operations::ListOperationsResponse::Status200_KnownOperations(models::OperationList::new(
            self.state.operations.list(),
        )))
    }
}

impl From<ApiError> for apis::operations::CancelOperationResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::operations::GetOperationResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::operations::ListOperationsResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

#[cfg(test)]
mod tests {
    use omnius_pxna_engine::service::ArchiveOptions;
    use serde_json::json;
    use testresult::TestResult;

    use super::*;
    use crate::{
        interface::server::tests::*,
        prelude::*,
        service::mutation::{DeleteResult, LibraryMutation, MoveResult},
    };

    #[tokio::test]
    async fn operation_overwrite_merge_skip_and_recursive_delete_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let state = &fixture.server.state;
        let root = &state.conf.libraries[0].path;
        std::fs::create_dir_all(root.join("source/sub"))?;
        std::fs::create_dir_all(root.join("target/sub"))?;
        std::fs::write(root.join("source/sub/replaced.txt"), b"new")?;
        std::fs::write(root.join("target/sub/replaced.txt"), b"old")?;
        std::fs::write(root.join("target/retained.txt"), b"retain")?;
        std::fs::write(root.join("source/type.txt"), b"file")?;
        std::fs::create_dir_all(root.join("target/type.txt"))?;
        std::fs::write(root.join("target/type.txt/old"), b"old")?;
        let request: models::EntryTransferRequest = serde_json::from_value(json!({"srcPaths":["/source"],"dstPath":"/target","conflict":"overwrite"}))?;
        let operation = state.operations.copy("photos", &request, true).await?;
        let operation = wait_operation(state, &operation.id).await?;
        assert_eq!(operation.status, models::OperationStatus::Succeeded);
        assert_eq!(operation.processed_count, 4);
        assert_eq!(std::fs::read(root.join("target/sub/replaced.txt"))?, b"new");
        assert_eq!(std::fs::read(root.join("target/retained.txt"))?, b"retain");
        assert_eq!(std::fs::read(root.join("target/type.txt"))?, b"file");
        let skip: models::EntryTransferRequest = serde_json::from_value(json!({"srcPaths":["/source/type.txt","/photos/notes.txt"],"dstFolderPath":"/target","conflict":"skip"}))?;
        let started = state.operations.copy("photos", &skip, false).await?;
        let operation = wait_operation(state, &started.id).await?;
        assert_eq!(operation.status, models::OperationStatus::Succeeded);
        assert_eq!((operation.processed_count, operation.skipped_count), (1, 1));
        let fail: models::EntryTransferRequest = serde_json::from_value(json!({"srcPaths":["/source/type.txt","/photos/notes.txt"],"dstFolderPath":"/target"}))?;
        let operation = state.operations.copy("photos", &fail, false).await?;
        let operation = wait_operation(state, &operation.id).await?;
        assert_eq!(operation.status, models::OperationStatus::Failed);
        assert_eq!(operation.failed_entries.len(), 2);
        let result = state
            .operations
            .delete_entries("photos", &[models::RelativePath("/source".to_string())], true, true)
            .await?;
        let DeleteResult::Started(operation) = result else {
            panic!("recursive delete must be asynchronous")
        };
        assert_eq!(wait_operation(state, &operation.id).await?.status, models::OperationStatus::Succeeded);
        assert!(!root.join("source").exists());
        Ok(())
    }

    #[tokio::test]
    async fn operation_copy_readonly_archive_and_synchronous_folder_move_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let state = &fixture.server.state;
        std::fs::write(state.conf.libraries[1].path.join("readonly.txt"), b"readable")?;
        let request: models::EntryTransferRequest = serde_json::from_value(json!({"srcPaths":["/readonly.txt"],"dstLibraryId":"photos","dstPath":"/copied.txt"}))?;
        let operation = state.operations.copy("videos", &request, false).await?;
        assert_eq!(wait_operation(state, &operation.id).await?.status, models::OperationStatus::Succeeded);
        assert!(state.operations.move_entries("videos", &request, false).await.is_err());
        let archive: models::EntryTransferRequest = serde_json::from_value(json!({"srcPaths":["/photos/a.7z"],"dstPath":"/expanded"}))?;
        let operation = state.operations.copy("photos", &archive, true).await?;
        assert_eq!(wait_operation(state, &operation.id).await?.status, models::OperationStatus::Succeeded);
        assert_eq!(std::fs::read(state.conf.libraries[0].path.join("expanded/dir/x.png"))?, b"pixel");
        let moved: models::EntryTransferRequest = serde_json::from_value(json!({"srcPaths":["/expanded"],"dstPath":"/renamed"}))?;
        let MoveResult::Completed(entries) = state.operations.move_entries("photos", &moved, true).await? else {
            panic!("single move must be synchronous")
        };
        assert_eq!(entries[0].path, "/renamed");
        assert!(!state.conf.libraries[0].path.join("expanded").exists());
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn mutation_symlink_boundary_and_partial_tree_failure_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let state = &fixture.server.state;
        let root = &state.conf.libraries[0].path;
        std::fs::create_dir(root.join("links"))?;
        std::fs::write(root.join("links/good.txt"), b"good")?;
        std::os::unix::fs::symlink(&state.conf.libraries[1].path, root.join("outside"))?;
        assert_eq!(
            state.mutation.destination("photos", "/../changed.txt", false).await.err().map(|error| error.kind().clone()),
            Some(ErrorKind::InvalidInput)
        );
        state
            .mutation
            .write("photos", "/outside/changed.txt", b"owner link", LibraryMutation::conflict(None))
            .await?;
        assert_eq!(std::fs::read(state.conf.libraries[1].path.join("changed.txt"))?, b"owner link");
        let direct: models::EntryTransferRequest = serde_json::from_value(json!({"srcPaths":["/outside/changed.txt"],"dstPath":"/direct-link-copy.txt"}))?;
        let operation = state.operations.copy("photos", &direct, false).await?;
        assert_eq!(wait_operation(state, &operation.id).await?.status, models::OperationStatus::Succeeded);
        std::os::unix::fs::symlink("good.txt", root.join("links/link.txt"))?;
        let direct: models::EntryTransferRequest = serde_json::from_value(json!({"srcPaths":["/links/link.txt"],"dstPath":"/direct-leaf-link.txt"}))?;
        let operation = state.operations.copy("photos", &direct, false).await?;
        assert_eq!(wait_operation(state, &operation.id).await?.status, models::OperationStatus::Succeeded);
        assert_eq!(std::fs::read(root.join("direct-leaf-link.txt"))?, b"good");

        let request: models::EntryTransferRequest = serde_json::from_value(json!({"srcPaths":["/links"],"dstPath":"/copied-links"}))?;
        let operation = state.operations.copy("photos", &request, true).await?;
        let operation = wait_operation(state, &operation.id).await?;
        assert_eq!(operation.status, models::OperationStatus::Failed);
        assert_eq!(operation.failed_entries[0].path, "/links/link.txt");
        assert_eq!(std::fs::read(root.join("copied-links/good.txt"))?, b"good");
        assert!(!root.join("copied-links/link.txt").exists());
        Ok(())
    }
}
