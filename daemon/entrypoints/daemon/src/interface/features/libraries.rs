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
impl apis::libraries::Libraries for ApiServer {
    type Claims = Claims;

    async fn get_capabilities(
        &self,
        _method: &Method,
        _host: &Host,
        _cookies: &CookieJar,
        _claims: &Self::Claims,
    ) -> std::result::Result<apis::libraries::GetCapabilitiesResponse, ()> {
        Ok(apis::libraries::GetCapabilitiesResponse::Status200_OptionalBrowsingFeaturesImplementedByThisDaemon(
            models::Capabilities::new(true, true, true),
        ))
    }

    async fn get_library(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetLibraryPathParams,
    ) -> std::result::Result<apis::libraries::GetLibraryResponse, ()> {
        Ok(match self.state.library.get_library(&path_params.library_id) {
            Ok(library) => apis::libraries::GetLibraryResponse::Status200_LibraryMetadata(models::LibraryResponse::new(library)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn list_libraries(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> std::result::Result<apis::libraries::ListLibrariesResponse, ()> {
        let libraries = self.state.library.list_libraries();
        Ok(apis::libraries::ListLibrariesResponse::Status200_AvailableLibraries(models::LibraryList::new(libraries)))
    }
}

impl From<ApiError> for apis::libraries::GetLibraryResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            // 契約に 400、409、504 はなく、id の照合だけで起きることもない。
            ApiStatus::BadRequest
            | ApiStatus::Conflict
            | ApiStatus::RangeNotSatisfiable
            | ApiStatus::GatewayTimeout
            | ApiStatus::PayloadTooLarge
            | ApiStatus::InsufficientStorage
            | ApiStatus::InternalServerError => Self::Status500_InternalServerError(error.body),
        }
    }
}

#[cfg(test)]
mod tests {
    use omnius_pxna_engine::service::ArchiveOptions;
    use testresult::TestResult;

    use super::*;
    use crate::interface::server::tests::*;

    async fn get_library(fixture: &Fixture, id: &str) -> apis::libraries::GetLibraryResponse {
        let params = models::GetLibraryPathParams { library_id: id.to_string() };
        apis::libraries::Libraries::get_library(&fixture.server, &Method::GET, &host(), &CookieJar::default(), &claims(), &params)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn libraries_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let libraries = apis::libraries::Libraries::list_libraries(&fixture.server, &Method::GET, &host(), &CookieJar::default(), &claims())
            .await
            .unwrap();
        let apis::libraries::ListLibrariesResponse::Status200_AvailableLibraries(list) = libraries else {
            panic!("library list must succeed");
        };
        let summary = list
            .libraries
            .iter()
            .map(|library| (library.id.as_str(), library.name.as_str(), library.readonly))
            .collect::<Vec<_>>();
        assert_eq!(summary, [("photos", "Photos", false), ("videos", "Videos", true)], "設定に書いた順で返す");

        let apis::libraries::GetLibraryResponse::Status200_LibraryMetadata(library) = get_library(&fixture, "videos").await else {
            panic!("configured library must be found");
        };
        assert_eq!((library.library.id.as_str(), library.library.readonly), ("videos", true));

        let apis::libraries::GetLibraryResponse::Status404_ResourceNotFound(body) = get_library(&fixture, "missing").await else {
            panic!("unknown library must be 404");
        };
        assert_eq!(body.kind, models::ErrorKind::NotFound);
        Ok(())
    }
}
