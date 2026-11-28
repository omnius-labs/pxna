//! The copy, move, and delete commands, which answer `202` with an Operation.

use omnius_pxna_interface::apis;

use crate::error::{ApiError, ApiStatus};

impl From<ApiError> for apis::entries::CopyEntriesResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::entries::MoveEntriesResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::entries::DeleteEntriesResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

#[cfg(test)]
mod tests {
    use http::{Method, StatusCode};
    use omnius_pxna_engine::service::ArchiveOptions;
    use serde_json::{Value, json};
    use testresult::TestResult;

    use crate::interface::server::tests::*;

    struct Client {
        router: axum::Router,
        bearer: String,
    }

    impl Client {
        async fn new(fixture: &Fixture) -> TestResult<Self> {
            let router = router(fixture);
            let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "bearer")).await?;
            let bearer = bearer(&setup)?;
            Ok(Self { router, bearer })
        }

        async fn post(&self, command: &str, body: Value) -> TestResult<HttpResponse> {
            let headers = [("authorization", self.bearer.as_str())];
            send(&self.router, Method::POST, &format!("/api/v1/entries/{command}"), &headers, Some(body)).await
        }

        /// Sends a command that must be accepted, and answers its Operation once it is terminal.
        async fn run(&self, fixture: &Fixture, command: &str, body: Value) -> TestResult<omnius_pxna_interface::models::Operation> {
            let response = self.post(command, body).await?;
            assert_eq!(response.status, StatusCode::ACCEPTED, "{command}: {}", response.body);
            let id = response.body["operation"]["id"].as_str().ok_or("operation id")?;
            wait_operation(&fixture.server.state, id).await
        }
    }

    fn entries(items: &[(&str, bool)]) -> Value {
        entry_refs("photos", items)
    }

    #[tokio::test]
    async fn a_mixed_selection_is_copied_moved_and_deleted_in_one_request_each_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let root = fixture.server.state.conf.libraries[0].path.clone();
        std::fs::create_dir_all(root.join("album/sub"))?;
        std::fs::write(root.join("album/sub/a.txt"), b"a")?;
        std::fs::write(root.join("one.txt"), b"one")?;
        std::fs::create_dir_all(root.join("copies"))?;
        std::fs::create_dir_all(root.join("moved"))?;
        let client = Client::new(&fixture).await?;
        let selection = [("/one.txt", false), ("/album", true)];

        let copied = client
            .run(
                &fixture,
                "copy",
                json!({"sources": entries(&selection), "destination": {"libraryId": "photos", "folderPath": "/copies"}}),
            )
            .await?;
        assert_eq!(copied.kind, omnius_pxna_interface::models::OperationKind::Copy);
        assert_eq!(copied.status, omnius_pxna_interface::models::OperationStatus::Succeeded, "{copied:?}");
        assert_eq!(std::fs::read(root.join("copies/one.txt"))?, b"one");
        assert_eq!(std::fs::read(root.join("copies/album/sub/a.txt"))?, b"a");

        let moved = client
            .run(
                &fixture,
                "move",
                json!({"sources": entries(&selection), "destination": {"libraryId": "photos", "folderPath": "/moved"}}),
            )
            .await?;
        assert_eq!(moved.kind, omnius_pxna_interface::models::OperationKind::Move);
        assert_eq!(moved.status, omnius_pxna_interface::models::OperationStatus::Succeeded, "{moved:?}");
        assert!(root.join("moved/one.txt").is_file() && root.join("moved/album/sub/a.txt").is_file());
        assert!(!root.join("one.txt").exists() && !root.join("album").exists());

        let deleted = client
            .run(
                &fixture,
                "delete",
                json!({"targets": entries(&[("/moved/one.txt", false), ("/moved/album", true)]), "recursive": true}),
            )
            .await?;
        assert_eq!(deleted.kind, omnius_pxna_interface::models::OperationKind::Delete);
        assert_eq!(deleted.status, omnius_pxna_interface::models::OperationStatus::Succeeded, "{deleted:?}");
        assert!(!root.join("moved/one.txt").exists() && !root.join("moved/album").exists());
        Ok(())
    }

    #[tokio::test]
    async fn one_move_within_a_library_answers_a_terminal_operation_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let client = Client::new(&fixture).await?;
        let moved = client
            .post(
                "move",
                json!({"sources": entries(&[("/photos/notes.txt", false)]), "destination": {"libraryId": "photos", "path": "/notes.txt"}}),
            )
            .await?;
        assert_eq!(moved.status, StatusCode::ACCEPTED);
        assert_eq!(moved.body["operation"]["status"], "succeeded");
        assert_eq!(moved.body["operation"]["resultEntries"][0]["path"], "/notes.txt");

        // A conflict found while renaming is a failed entry of the terminal Operation, not an error status.
        std::fs::write(fixture.server.state.conf.libraries[0].path.join("taken.txt"), b"taken")?;
        let conflict = client
            .post(
                "move",
                json!({"sources": entries(&[("/notes.txt", false)]), "destination": {"libraryId": "photos", "path": "/taken.txt"}}),
            )
            .await?;
        assert_eq!(conflict.status, StatusCode::ACCEPTED);
        assert_eq!(conflict.body["operation"]["status"], "failed");
        assert_eq!(conflict.body["operation"]["failedEntries"][0]["kind"], "Conflict");
        Ok(())
    }

    #[tokio::test]
    async fn the_checks_before_a_command_starts_keep_their_statuses_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let client = Client::new(&fixture).await?;
        let cases = [
            (
                "copy",
                json!({"sources": [{"libraryId": "photos", "path": "/photos/notes.txt"}, {"libraryId": "videos", "path": "/v.txt"}], "destination": {"libraryId": "photos", "folderPath": "/"}}),
                StatusCode::BAD_REQUEST,
            ),
            (
                "copy",
                json!({"sources": entries(&[("/photos/notes.txt", false)]), "destination": {"libraryId": "photos"}}),
                StatusCode::BAD_REQUEST,
            ),
            (
                "move",
                json!({"sources": entries(&[("/photos/notes.txt", false), ("/photos/a.7z", false)]), "destination": {"libraryId": "photos", "path": "/x"}}),
                StatusCode::BAD_REQUEST,
            ),
            (
                "move",
                json!({"sources": entries(&[("/photos/missing.txt", false)]), "destination": {"libraryId": "photos", "folderPath": "/"}}),
                StatusCode::NOT_FOUND,
            ),
            (
                "move",
                json!({"sources": entries(&[("/photos/notes.txt", false)]), "destination": {"libraryId": "videos", "folderPath": "/"}}),
                StatusCode::FORBIDDEN,
            ),
            ("delete", json!({"targets": entries(&[("/photos/a.7z", true)]), "recursive": true}), StatusCode::FORBIDDEN),
            ("delete", json!({"targets": entries(&[("/photos/a.7z/y.txt", false)])}), StatusCode::FORBIDDEN),
            ("delete", json!({"targets": entries(&[("/photos", true)])}), StatusCode::BAD_REQUEST),
            ("delete", json!({"targets": []}), StatusCode::BAD_REQUEST),
        ];
        for (command, body, expected) in cases {
            let response = client.post(command, body.clone()).await?;
            assert_eq!(response.status, expected, "{command} {body}: {}", response.body);
            assert!(response.body["code"].is_string(), "{}", response.body);
        }
        assert!(
            fixture.server.state.conf.libraries[0].path.join("photos/a.7z").is_file(),
            "an archive's root is not deleted"
        );

        let archive = client.run(&fixture, "delete", json!({"targets": entries(&[("/photos/a.7z", false)])})).await?;
        assert_eq!(
            archive.status,
            omnius_pxna_interface::models::OperationStatus::Succeeded,
            "the archive file is deleted as a file"
        );
        Ok(())
    }
}
