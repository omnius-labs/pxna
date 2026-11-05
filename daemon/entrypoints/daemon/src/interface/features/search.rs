use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_interface::{apis, models};

use omnius_core_base::error::OmniError as _;

use crate::{
    error::{ApiError, ApiStatus, Error, ErrorKind},
    interface::{ApiServer, session::Claims},
    service::{library::LibraryRelativePath, listing::ListOptions},
};

#[allow(unused_variables)]
#[async_trait]
impl apis::search::Search for ApiServer {
    type Claims = Claims;

    async fn list_search_entries(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        query_params: &models::ListSearchEntriesQueryParams,
    ) -> std::result::Result<apis::search::ListSearchEntriesResponse, ()> {
        let listing = async {
            let options = ListOptions::from_search(query_params)?;
            let path = query_params.path.as_deref().unwrap_or("/");
            let located = match (&query_params.locate, &query_params.locate_library_id) {
                (Some(locate), Some(library_id)) => Some((locate, library_id)),
                (None, None) => None,
                _ => return Err(Error::new(ErrorKind::InvalidInput).with_message("locate and locateLibraryId must be given together")),
            };
            if let Some((_, library_id)) = located {
                if query_params.offset.is_some() {
                    return Err(Error::new(ErrorKind::InvalidInput).with_message("locate and offset cannot be given together"));
                }
                if !query_params.library_ids.iter().any(|id| &id.0 == library_id) {
                    return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("locateLibraryId is not in libraryIds: {library_id}")));
                }
            }
            let mut libraries = Vec::new();
            for library_id in &query_params.library_ids {
                let selection = options.marks().select(&self.state.rating_tags, &claims.user.id, &library_id.0).await?;
                libraries.push((library_id.0.clone(), options.clone().with_selection(selection)));
            }
            match located {
                Some((locate, library_id)) => {
                    let entry = LibraryRelativePath::join(path, locate)?.absolute();
                    self.state.library.search_entries_located(libraries, path, &options, library_id, &entry).await
                }
                None => self.state.library.search_entries(libraries, path, &options).await,
            }
        }
        .await;
        Ok(match listing {
            Ok(listing) => apis::search::ListSearchEntriesResponse::Status200_MatchingEntriesOfTheLibraries(listing),
            Err(error) => ApiError::from(error).into(),
        })
    }
}

impl From<ApiError> for apis::search::ListSearchEntriesResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            ApiStatus::Conflict | ApiStatus::RangeNotSatisfiable | ApiStatus::PayloadTooLarge | ApiStatus::InsufficientStorage | ApiStatus::InternalServerError => {
                Self::Status500_InternalServerError(error.body)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use http::StatusCode;
    use omnius_pxna_engine::service::ArchiveOptions;
    use testresult::TestResult;

    use crate::interface::server::tests::*;

    const SEARCH: &str = "/api/v1/search/entries";

    async fn login(router: &axum::Router) -> TestResult<String> {
        let setup = send(router, http::Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "bearer")).await?;
        bearer(&setup)
    }

    #[tokio::test]
    async fn searches_several_libraries_as_one_sequence_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let dir = fixture._dir.path();
        std::fs::write(dir.join("library/photos/b-sun.png"), b"x")?;
        std::fs::write(dir.join("videos/a-sun.mp4"), b"x")?;
        std::fs::write(dir.join("videos/c-sun.mp4"), b"x")?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];

        let uri = format!("{SEARCH}?libraryIds=photos&libraryIds=videos&query=sun&recursive=true&sort=name");
        let response = send(&router, http::Method::GET, &uri, headers, None).await?;
        assert_eq!(response.status, StatusCode::OK, "{}", response.body);
        let found = response.body["entries"]
            .as_array()
            .ok_or("entries is missing")?
            .iter()
            .map(|entry| {
                format!(
                    "{}:{}",
                    entry["libraryId"].as_str().unwrap_or_default(),
                    entry["entry"]["path"].as_str().unwrap_or_default()
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(found, ["videos:/a-sun.mp4", "photos:/photos/b-sun.png", "videos:/c-sun.mp4"]);
        assert_eq!(response.body["totalCount"], 3);
        assert_eq!(response.body["truncatedLibraryIds"], serde_json::json!([]));

        let second = send(&router, http::Method::GET, &format!("{uri}&offset=1&limit=1"), headers, None).await?;
        assert_eq!(second.body["entries"][0]["entry"]["path"], "/photos/b-sun.png");
        assert_eq!(second.body["snapshot"], response.body["snapshot"]);
        Ok(())
    }

    #[tokio::test]
    async fn a_start_needs_one_library_and_every_library_must_exist_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];

        let started = send(&router, http::Method::GET, &format!("{SEARCH}?libraryIds=photos&path=/photos&query=a"), headers, None).await?;
        assert_eq!(started.status, StatusCode::OK, "{}", started.body);
        let mixed = send(
            &router,
            http::Method::GET,
            &format!("{SEARCH}?libraryIds=photos&libraryIds=videos&path=/photos&query=a"),
            headers,
            None,
        )
        .await?;
        assert_eq!(mixed.status, StatusCode::BAD_REQUEST);
        let unknown = send(&router, http::Method::GET, &format!("{SEARCH}?libraryIds=photos&libraryIds=missing&query=a"), headers, None).await?;
        assert_eq!(unknown.status, StatusCode::NOT_FOUND);
        Ok(())
    }

    #[tokio::test]
    async fn locate_search_returns_the_page_of_the_entry_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let dir = fixture._dir.path();
        std::fs::write(dir.join("library/photos/b-sun.png"), b"x")?;
        std::fs::write(dir.join("videos/a-sun.mp4"), b"x")?;
        std::fs::write(dir.join("videos/c-sun.mp4"), b"x")?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];

        let uri = format!("{SEARCH}?libraryIds=photos&libraryIds=videos&query=sun&recursive=true&sort=name&limit=1");
        let located = send(&router, http::Method::GET, &format!("{uri}&locate=photos/b-sun.png&locateLibraryId=photos"), headers, None).await?;
        assert_eq!(located.status, StatusCode::OK, "{}", located.body);
        assert_eq!(located.body["offset"], 1);
        assert_eq!(located.body["entries"][0]["libraryId"], "photos");
        assert_eq!(located.body["entries"][0]["entry"]["path"], "/photos/b-sun.png");

        let same = send(&router, http::Method::GET, &format!("{uri}&offset=1"), headers, None).await?;
        assert_eq!(located.body["entries"], same.body["entries"]);
        assert_eq!(located.body["totalCount"], same.body["totalCount"]);

        let started = send(
            &router,
            http::Method::GET,
            &format!("{SEARCH}?libraryIds=photos&path=/photos&query=sun&recursive=true&locate=b-sun.png&locateLibraryId=photos"),
            headers,
            None,
        )
        .await?;
        assert_eq!(started.status, StatusCode::OK, "{}", started.body);
        assert_eq!(started.body["entries"][0]["entry"]["path"], "/photos/b-sun.png");
        Ok(())
    }

    #[tokio::test]
    async fn locate_errors_of_the_search_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        std::fs::write(fixture._dir.path().join("library/photos/b-sun.png"), b"x")?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];

        for uri in [
            format!("{SEARCH}?libraryIds=photos&query=sun&locate=b-sun.png"),
            format!("{SEARCH}?libraryIds=photos&query=sun&locateLibraryId=photos"),
            format!("{SEARCH}?libraryIds=photos&query=sun&locate=b-sun.png&locateLibraryId=videos"),
            format!("{SEARCH}?libraryIds=photos&query=sun&locate=b-sun.png&locateLibraryId=photos&offset=0"),
        ] {
            let response = send(&router, http::Method::GET, &uri, headers, None).await?;
            assert_eq!(response.status, StatusCode::BAD_REQUEST, "{uri}");
        }
        let missing = send(
            &router,
            http::Method::GET,
            &format!("{SEARCH}?libraryIds=photos&query=sun&recursive=true&locate=missing.png&locateLibraryId=photos"),
            headers,
            None,
        )
        .await?;
        assert_eq!(missing.status, StatusCode::NOT_FOUND);
        Ok(())
    }
}
