use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_interface::{apis, models};

use omnius_core_base::error::OmniError as _;

use crate::{
    error::{ApiError, ApiStatus, Error, ErrorKind},
    interface::{ApiServer, session::Claims},
    service::{
        library::LibraryRelativePath,
        listing::{ListOptions, SummaryOptions},
        mutation::{DeleteResult, MoveResult},
    },
};

#[allow(unused_variables)]
#[async_trait]
impl apis::folders::Folders for ApiServer {
    type Claims = Claims;

    async fn copy_folders(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CopyFoldersPathParams,
        body: &models::EntryTransferRequest,
    ) -> std::result::Result<apis::folders::CopyFoldersResponse, ()> {
        Ok(match self.state.operations.copy(&path_params.library_id, body, true).await {
            Ok(operation) => apis::folders::CopyFoldersResponse::Status202_CopyOperationStarted(models::OperationResponse::new(operation)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn create_folder(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CreateFolderPathParams,
        body: &models::CreateFolderRequest,
    ) -> std::result::Result<apis::folders::CreateFolderResponse, ()> {
        Ok(match self.state.mutation.create_folder(&path_params.library_id, body).await {
            Ok(file) => apis::folders::CreateFolderResponse::Status201_FolderCreated(models::FolderResponse::new(file)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn delete_folders(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::DeleteFoldersPathParams,
        body: &models::DeleteFoldersRequest,
    ) -> std::result::Result<apis::folders::DeleteFoldersResponse, ()> {
        Ok(
            match self
                .state
                .operations
                .delete_entries(&path_params.library_id, &body.src_paths, true, body.recursive.unwrap_or(false))
                .await
            {
                Ok(DeleteResult::Completed) => apis::folders::DeleteFoldersResponse::Status204_FoldersDeleted,
                Ok(DeleteResult::Started(operation)) => apis::folders::DeleteFoldersResponse::Status202_DeleteOperationStarted(models::OperationResponse::new(operation)),
                Err(error) => ApiError::from(error).into(),
            },
        )
    }

    async fn get_folder(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetFolderPathParams,
        query_params: &models::GetFolderQueryParams,
    ) -> std::result::Result<apis::folders::GetFolderResponse, ()> {
        Ok(match self.state.library.get_folder(&path_params.library_id, &query_params.path).await {
            Ok(folder) => apis::folders::GetFolderResponse::Status200_FolderMetadata(models::FolderResponse::new(folder)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn list_folder_entries(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::ListFolderEntriesPathParams,
        query_params: &models::ListFolderEntriesQueryParams,
    ) -> std::result::Result<apis::folders::ListFolderEntriesResponse, ()> {
        let listing = async {
            let options = ListOptions::from_query(query_params)?;
            match &query_params.locate {
                Some(locate) => {
                    if query_params.offset.is_some() {
                        return Err(Error::new(ErrorKind::InvalidInput).with_message("locate and offset cannot be given together"));
                    }
                    let entry = LibraryRelativePath::join(&query_params.path, locate)?.absolute();
                    self.state
                        .library
                        .list_folder_entries_located(&path_params.library_id, &query_params.path, &options, &entry)
                        .await
                }
                None => self.state.library.list_folder_entries(&path_params.library_id, &query_params.path, &options).await,
            }
        }
        .await;
        Ok(match listing {
            Ok(listing) => apis::folders::ListFolderEntriesResponse::Status200_FolderEntries(listing),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn list_folder_summaries(
        &self,
        _method: &Method,
        _host: &Host,
        _cookies: &CookieJar,
        _claims: &Self::Claims,
        path: &models::ListFolderSummariesPathParams,
        query: &models::ListFolderSummariesQueryParams,
    ) -> std::result::Result<apis::folders::ListFolderSummariesResponse, ()> {
        let result = async {
            let options = SummaryOptions::from_query(query)?;
            match &query.locate {
                Some(locate) => {
                    if query.offset.is_some() {
                        return Err(Error::new(ErrorKind::InvalidInput).with_message("locate and offset cannot be given together"));
                    }
                    let entry = LibraryRelativePath::join(&query.path, locate)?.absolute();
                    self.state.library.list_folder_summaries_located(&path.library_id, &query.path, &options, &entry).await
                }
                None => self.state.library.list_folder_summaries(&path.library_id, &query.path, &options).await,
            }
        }
        .await;
        Ok(match result {
            Ok(listing) => apis::folders::ListFolderSummariesResponse::Status200_NamesAndKindsFromOneSummarySnapshot(listing),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn get_entry_details(
        &self,
        _method: &Method,
        _host: &Host,
        _cookies: &CookieJar,
        _claims: &Self::Claims,
        path: &models::GetEntryDetailsPathParams,
        body: &models::EntryDetailsRequest,
    ) -> std::result::Result<apis::folders::GetEntryDetailsResponse, ()> {
        let paths = body.paths.iter().map(|path| path.0.clone()).collect::<Vec<_>>();
        Ok(match self.state.library.entry_details(&path.library_id, &paths).await {
            Ok(details) => apis::folders::GetEntryDetailsResponse::Status200_ExactlyOneDetailOrErrorKindPerRequestedPath(details),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn probe_folder_children(
        &self,
        _method: &Method,
        _host: &Host,
        _cookies: &CookieJar,
        _claims: &Self::Claims,
        path: &models::ProbeFolderChildrenPathParams,
        query: &models::ProbeFolderChildrenQueryParams,
    ) -> std::result::Result<apis::folders::ProbeFolderChildrenResponse, ()> {
        Ok(
            match self
                .state
                .library
                .probe_folder_children(&path.library_id, &query.path, query.include_hidden.unwrap_or(false))
                .await
            {
                Ok(result) => apis::folders::ProbeFolderChildrenResponse::Status200_True(result),
                Err(error) => ApiError::from(error).into(),
            },
        )
    }

    async fn move_folders(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::MoveFoldersPathParams,
        body: &models::EntryTransferRequest,
    ) -> std::result::Result<apis::folders::MoveFoldersResponse, ()> {
        Ok(match self.state.operations.move_entries(&path_params.library_id, body, true).await {
            Ok(MoveResult::Completed(entries)) => apis::folders::MoveFoldersResponse::Status200_MovedSynchronously(models::EntryList::new(entries)),
            Ok(MoveResult::Started(operation)) => apis::folders::MoveFoldersResponse::Status202_MoveOperationStarted(models::OperationResponse::new(operation)),
            Err(error) => ApiError::from(error).into(),
        })
    }
}

impl From<ApiError> for apis::folders::GetFolderResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            ApiStatus::RangeNotSatisfiable | ApiStatus::PayloadTooLarge | ApiStatus::InsufficientStorage | ApiStatus::InternalServerError => {
                Self::Status500_InternalServerError(error.body)
            }
        }
    }
}

impl From<ApiError> for apis::folders::ListFolderEntriesResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            ApiStatus::RangeNotSatisfiable | ApiStatus::PayloadTooLarge | ApiStatus::InsufficientStorage | ApiStatus::InternalServerError => {
                Self::Status500_InternalServerError(error.body)
            }
        }
    }
}

impl From<ApiError> for apis::folders::CopyFoldersResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::folders::CreateFolderResponse {
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

impl From<ApiError> for apis::folders::DeleteFoldersResponse {
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

impl From<ApiError> for apis::folders::MoveFoldersResponse {
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
    use omnius_pxna_engine::service::ArchiveOptions;
    use omnius_pxna_interface::types::Nullable;
    use testresult::TestResult;

    use super::*;
    use crate::interface::server::tests::*;

    async fn list(fixture: &Fixture, path: &str) -> apis::folders::ListFolderEntriesResponse {
        let query = models::ListFolderEntriesQueryParams {
            path: path.to_string(),
            recursive: None,
            offset: None,
            locate: None,
            query: None,
            limit: None,
            sort: None,
            order: None,
            include_hidden: None,
            folders_first: None,
            folders_only: None,
        };
        let params = models::ListFolderEntriesPathParams { library_id: "photos".to_string() };
        apis::folders::Folders::list_folder_entries(&fixture.server, &Method::GET, &host(), &CookieJar::default(), &claims(), &params, &query)
            .await
            .unwrap()
    }

    async fn list_query(fixture: &Fixture, path: &str, locate: Option<&str>, limit: Option<u32>, offset: Option<u64>) -> apis::folders::ListFolderEntriesResponse {
        let query = models::ListFolderEntriesQueryParams {
            path: path.to_string(),
            locate: locate.map(str::to_string),
            limit,
            offset,
            ..crate::service::listing::tests::query(None, None)
        };
        let params = models::ListFolderEntriesPathParams { library_id: "photos".to_string() };
        apis::folders::Folders::list_folder_entries(&fixture.server, &Method::GET, &host(), &CookieJar::default(), &claims(), &params, &query)
            .await
            .unwrap()
    }

    async fn summary_query(fixture: &Fixture, path: &str, locate: Option<&str>, limit: Option<u32>, offset: Option<u64>) -> apis::folders::ListFolderSummariesResponse {
        let query = models::ListFolderSummariesQueryParams {
            path: path.to_string(),
            offset,
            locate: locate.map(str::to_string),
            query: None,
            limit,
            sort: None,
            order: None,
            include_hidden: None,
            folders_first: None,
            folders_only: None,
        };
        let params = models::ListFolderSummariesPathParams { library_id: "photos".to_string() };
        apis::folders::Folders::list_folder_summaries(&fixture.server, &Method::GET, &host(), &CookieJar::default(), &claims(), &params, &query)
            .await
            .unwrap()
    }

    /// A folder written just now is rescanned after the first listing; age it so the two requests share a snapshot.
    fn age(path: &std::path::Path) -> std::io::Result<()> {
        let modified = std::time::SystemTime::now() - std::time::Duration::from_secs(10);
        std::fs::File::open(path)?.set_modified(modified)
    }

    #[tokio::test]
    async fn locate_folder_returns_the_page_of_the_entry_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let shelf = fixture._dir.path().join("library/shelf");
        std::fs::create_dir_all(shelf.join("sub"))?;
        for name in ["a.png", "b.png", "c.png"] {
            std::fs::write(shelf.join(name), b"png")?;
        }
        std::fs::write(shelf.join("sub/d.png"), b"png")?;
        age(&shelf)?;

        let located = list_query(&fixture, "/shelf", Some("b.png"), Some(1), None).await;
        let apis::folders::ListFolderEntriesResponse::Status200_FolderEntries(located) = located else {
            panic!("locate must succeed");
        };
        assert_eq!(located.offset, 1);
        assert_eq!(located.entries.iter().map(|entry| entry.name.as_str()).collect::<Vec<_>>(), ["b.png"]);
        assert_eq!(located.total_count, 4);

        let same = list_query(&fixture, "/shelf", None, Some(1), Some(1)).await;
        let apis::folders::ListFolderEntriesResponse::Status200_FolderEntries(same) = same else {
            panic!("the page of the same offset must succeed");
        };
        assert_eq!(located.entries, same.entries);
        assert_eq!(located.snapshot, same.snapshot);
        assert_eq!(located.total_count, same.total_count);

        let recursive = {
            let query = models::ListFolderEntriesQueryParams {
                path: "/shelf".to_string(),
                recursive: Some(true),
                locate: Some("sub/d.png".to_string()),
                ..crate::service::listing::tests::query(None, None)
            };
            let params = models::ListFolderEntriesPathParams { library_id: "photos".to_string() };
            apis::folders::Folders::list_folder_entries(&fixture.server, &Method::GET, &host(), &CookieJar::default(), &claims(), &params, &query)
                .await
                .unwrap()
        };
        let apis::folders::ListFolderEntriesResponse::Status200_FolderEntries(recursive) = recursive else {
            panic!("a descendant must be found in a recursive listing");
        };
        assert!(recursive.entries.iter().any(|entry| entry.path == "/shelf/sub/d.png"));
        Ok(())
    }

    #[tokio::test]
    async fn locate_summary_returns_the_page_of_the_entry_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let shelf = fixture._dir.path().join("library/shelf");
        std::fs::create_dir_all(&shelf)?;
        for name in ["a.png", "b.png", "c.png"] {
            std::fs::write(shelf.join(name), b"png")?;
        }
        age(&shelf)?;

        let located = summary_query(&fixture, "/shelf", Some("b.png"), Some(1), None).await;
        let apis::folders::ListFolderSummariesResponse::Status200_NamesAndKindsFromOneSummarySnapshot(located) = located else {
            panic!("locate must succeed");
        };
        assert_eq!(located.offset, 1);
        assert_eq!(located.entries.iter().map(|entry| entry.name.as_str()).collect::<Vec<_>>(), ["b.png"]);

        let same = summary_query(&fixture, "/shelf", None, Some(1), Some(1)).await;
        let apis::folders::ListFolderSummariesResponse::Status200_NamesAndKindsFromOneSummarySnapshot(same) = same else {
            panic!("the page of the same offset must succeed");
        };
        assert_eq!(located.entries, same.entries);
        assert_eq!(located.snapshot, same.snapshot);
        Ok(())
    }

    #[tokio::test]
    async fn locate_errors_of_the_folder_listings_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let shelf = fixture._dir.path().join("library/shelf");
        std::fs::create_dir_all(&shelf)?;
        std::fs::write(shelf.join("a.png"), b"png")?;
        age(&shelf)?;

        assert!(matches!(
            list_query(&fixture, "/shelf", Some("a.png"), None, Some(0)).await,
            apis::folders::ListFolderEntriesResponse::Status400_InvalidRequest(_)
        ));
        assert!(matches!(
            list_query(&fixture, "/shelf", Some("a.png"), Some(1), Some(1)).await,
            apis::folders::ListFolderEntriesResponse::Status400_InvalidRequest(_)
        ));
        assert!(matches!(
            list_query(&fixture, "/shelf", Some("missing.png"), None, None).await,
            apis::folders::ListFolderEntriesResponse::Status404_ResourceNotFound(_)
        ));
        assert!(matches!(
            summary_query(&fixture, "/shelf", Some("a.png"), None, Some(0)).await,
            apis::folders::ListFolderSummariesResponse::Status400_InvalidRequest(_)
        ));
        assert!(matches!(
            summary_query(&fixture, "/shelf", Some("missing.png"), None, None).await,
            apis::folders::ListFolderSummariesResponse::Status404_ResourceNotFound(_)
        ));
        Ok(())
    }

    #[tokio::test]
    async fn listing_query_recurses_without_entering_archives_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let photos = fixture._dir.path().join("library/photos");
        std::fs::create_dir_all(photos.join("sub"))?;
        std::fs::create_dir_all(photos.join(".hidden"))?;
        std::fs::write(photos.join("sub/Sunset.PNG"), b"png")?;
        std::fs::write(photos.join(".hidden/sunset.png"), b"hidden")?;
        write_7z(&photos.join("sunset.7z"), &[("sunset-inner.png", b"inner")])?;
        let mut query = models::ListFolderEntriesQueryParams {
            path: "/photos".to_string(),
            recursive: Some(true),
            query: Some("SUNSET".to_string()),
            ..crate::service::listing::tests::query(None, None)
        };
        let options = ListOptions::from_query(&query)?;
        let listing = fixture.server.state.library.list_folder_entries("photos", &query.path, &options).await?;
        assert_eq!(listing.total_count, 2);
        assert_eq!(
            listing.entries.iter().map(|entry| entry.path.as_str()).collect::<Vec<_>>(),
            ["/photos/sunset.7z", "/photos/sub/Sunset.PNG"]
        );
        query.path = "/photos/sunset.7z".to_string();
        query.query = Some("inner".to_string());
        let options = ListOptions::from_query(&query)?;
        let listing = fixture.server.state.library.list_folder_entries("photos", &query.path, &options).await?;
        assert_eq!(
            listing.entries.iter().map(|entry| entry.path.as_str()).collect::<Vec<_>>(),
            ["/photos/sunset.7z/sunset-inner.png"]
        );
        Ok(())
    }

    #[tokio::test]
    async fn listing_returns_content_type_for_images_and_videos_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let photos = fixture._dir.path().join("library/photos");
        std::fs::write(photos.join("Pic.PNG"), b"png")?;
        std::fs::write(photos.join("clip.mp4"), b"mp4")?;
        std::fs::create_dir_all(photos.join("sub"))?;

        let apis::folders::ListFolderEntriesResponse::Status200_FolderEntries(listing) = list(&fixture, "/photos").await else {
            panic!("listing must succeed");
        };
        let content_type = |name: &str| {
            let entry = listing.entries.iter().find(|entry| entry.name == name).unwrap_or_else(|| panic!("{name} is missing"));
            match &entry.metadata.content_type {
                Some(Nullable::Present(value)) => Some(value.clone()),
                _ => None,
            }
        };
        assert_eq!(content_type("Pic.PNG").as_deref(), Some("image/png"));
        assert_eq!(content_type("clip.mp4").as_deref(), Some("video/mp4"));
        // 画像でも動画でもない file、フォルダ、archive には載せない。
        assert_eq!(content_type("notes.txt"), None);
        assert_eq!(content_type("sub"), None);
        assert_eq!(content_type("a.7z"), None);
        Ok(())
    }

    #[tokio::test]
    async fn archive_root_listing_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let apis::folders::ListFolderEntriesResponse::Status200_FolderEntries(listing) = list(&fixture, "/photos/a.7z").await else {
            panic!("archive root listing must succeed");
        };
        let paths = listing.entries.iter().map(|entry| entry.path.as_str()).collect::<Vec<_>>();
        assert_eq!(paths, ["/photos/a.7z/dir", "/photos/a.7z/y.txt"]);
        assert_eq!(listing.entries[0].kind, models::FileSystemEntryKind::Folder);

        let folder = apis::folders::Folders::get_folder(
            &fixture.server,
            &Method::GET,
            &host(),
            &CookieJar::default(),
            &claims(),
            &models::GetFolderPathParams { library_id: "photos".to_string() },
            &models::GetFolderQueryParams { path: "/photos/a.7z".to_string() },
        )
        .await
        .unwrap();
        let apis::folders::GetFolderResponse::Status200_FolderMetadata(folder) = folder else {
            panic!("archive root must be a folder");
        };
        assert_eq!(folder.folder.path, "/photos/a.7z");
        assert_eq!(folder.folder.kind, models::FileSystemEntryKind::Folder);
        Ok(())
    }
}

impl From<ApiError> for apis::folders::ListFolderSummariesResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            ApiStatus::RangeNotSatisfiable | ApiStatus::PayloadTooLarge | ApiStatus::InsufficientStorage | ApiStatus::InternalServerError => {
                Self::Status500_InternalServerError(error.body)
            }
        }
    }
}

impl From<ApiError> for apis::folders::GetEntryDetailsResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::PayloadTooLarge => Self::Status413_TheRequestBodyExceedsTheOne(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::folders::ProbeFolderChildrenResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}
