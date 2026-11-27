use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_engine::model::{ArchiveOpen, DisplaySet, ListView, OriginalImageMode, Preferences, SortKey, SortOrder, SortPreference, ThumbnailShape};
use omnius_pxna_interface::{apis, models};

use crate::{
    error::{ApiError, ApiStatus, Error},
    interface::{ApiServer, session::Claims},
};

#[allow(unused_variables)]
#[async_trait]
impl apis::preferences::Preferences for ApiServer {
    type Claims = Claims;

    async fn get_preferences(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
    ) -> std::result::Result<apis::preferences::GetPreferencesResponse, ()> {
        Ok(match self.state.preferences.get().await {
            Ok(preferences) => apis::preferences::GetPreferencesResponse::Status200_DisplayPreferences(PreferencesModel::to_model(preferences)),
            Err(error) => ApiError::from(Error::from(error)).into(),
        })
    }

    async fn put_preferences(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        body: &models::Preferences,
    ) -> std::result::Result<apis::preferences::PutPreferencesResponse, ()> {
        let preferences = PreferencesModel::from_model(body);
        Ok(match self.state.preferences.put(&preferences).await {
            Ok(()) => apis::preferences::PutPreferencesResponse::Status200_TheSavedDisplayPreferences(PreferencesModel::to_model(preferences)),
            Err(error) => ApiError::from(Error::from(error)).into(),
        })
    }
}

impl From<ApiError> for apis::preferences::GetPreferencesResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            // 契約にある失敗は 401 と 500 だけで、保存済みの値を読むだけなので、ほかの失敗は起きない。
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::preferences::PutPreferencesResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

/// engine の表示設定と、API の応答と要求の型を相互に写す。
///
/// Shelves and folder settings hold values of the same shapes, so they map them through this too.
pub(super) struct PreferencesModel;

impl PreferencesModel {
    fn to_model(preferences: Preferences) -> models::Preferences {
        let mut model = models::Preferences::new(
            Self::display_set_to_model(preferences.wide),
            Self::display_set_to_model(preferences.narrow),
            Self::sort_to_model(preferences.sort),
        );
        model.archive_open = preferences.archive_open.map(Self::archive_open_to_model);
        model.timeline_libraries = preferences.timeline_libraries.map(|ids| ids.into_iter().map(models::LibraryId).collect());
        model
    }

    fn from_model(model: &models::Preferences) -> Preferences {
        Preferences {
            wide: Self::display_set_from_model(&model.wide),
            narrow: Self::display_set_from_model(&model.narrow),
            sort: Self::sort_from_model(&model.sort),
            archive_open: model.archive_open.map(Self::archive_open_from_model),
            timeline_libraries: model.timeline_libraries.as_ref().map(|ids| ids.iter().map(|id| id.0.clone()).collect()),
        }
    }

    pub(super) fn sort_to_model(sort: SortPreference) -> models::SortPreference {
        models::SortPreference::new(
            match sort.key {
                SortKey::Name => models::SortKey::Name,
                SortKey::Kind => models::SortKey::Kind,
                SortKey::Size => models::SortKey::Size,
                SortKey::ModifiedAt => models::SortKey::ModifiedAt,
            },
            match sort.order {
                SortOrder::Asc => models::SortOrder::Asc,
                SortOrder::Desc => models::SortOrder::Desc,
            },
        )
    }

    pub(super) fn sort_from_model(model: &models::SortPreference) -> SortPreference {
        SortPreference {
            key: match model.key {
                models::SortKey::Name => SortKey::Name,
                models::SortKey::Kind => SortKey::Kind,
                models::SortKey::Size => SortKey::Size,
                models::SortKey::ModifiedAt => SortKey::ModifiedAt,
            },
            order: match model.order {
                models::SortOrder::Asc => SortOrder::Asc,
                models::SortOrder::Desc => SortOrder::Desc,
            },
        }
    }

    pub(super) fn thumbnail_shape_to_model(value: ThumbnailShape) -> models::ThumbnailShape {
        match value {
            ThumbnailShape::Crop => models::ThumbnailShape::Crop,
            ThumbnailShape::Fit => models::ThumbnailShape::Fit,
        }
    }

    pub(super) fn thumbnail_shape_from_model(value: models::ThumbnailShape) -> ThumbnailShape {
        match value {
            models::ThumbnailShape::Crop => ThumbnailShape::Crop,
            models::ThumbnailShape::Fit => ThumbnailShape::Fit,
        }
    }

    pub(super) fn archive_open_to_model(value: ArchiveOpen) -> models::ArchiveOpen {
        match value {
            ArchiveOpen::Folder => models::ArchiveOpen::Folder,
            ArchiveOpen::Viewer => models::ArchiveOpen::Viewer,
        }
    }

    pub(super) fn archive_open_from_model(value: models::ArchiveOpen) -> ArchiveOpen {
        match value {
            models::ArchiveOpen::Folder => ArchiveOpen::Folder,
            models::ArchiveOpen::Viewer => ArchiveOpen::Viewer,
        }
    }

    fn display_set_to_model(set: DisplaySet) -> models::DisplaySet {
        models::DisplaySet::new(
            u32::from(set.tile_size),
            Self::thumbnail_shape_to_model(set.thumbnail_shape),
            match set.view {
                ListView::Grid => models::ListView::Grid,
                ListView::List => models::ListView::List,
            },
            match set.original_image {
                OriginalImageMode::Auto => models::OriginalImageMode::Auto,
                OriginalImageMode::OnZoom => models::OriginalImageMode::OnZoom,
            },
            u32::from(set.filmstrip_size),
        )
    }

    fn display_set_from_model(model: &models::DisplaySet) -> DisplaySet {
        DisplaySet {
            // u8 に収まらない値は範囲外として、engine の検査が拒否する。
            tile_size: u8::try_from(model.tile_size).unwrap_or(u8::MAX),
            thumbnail_shape: Self::thumbnail_shape_from_model(model.thumbnail_shape),
            view: match model.view {
                models::ListView::Grid => ListView::Grid,
                models::ListView::List => ListView::List,
            },
            original_image: match model.original_image {
                models::OriginalImageMode::Auto => OriginalImageMode::Auto,
                models::OriginalImageMode::OnZoom => OriginalImageMode::OnZoom,
            },
            filmstrip_size: u8::try_from(model.filmstrip_size).unwrap_or(u8::MAX),
        }
    }
}

#[cfg(test)]
mod tests {
    use http::StatusCode;
    use omnius_pxna_engine::service::ArchiveOptions;
    use serde_json::json;
    use testresult::TestResult;

    use super::*;
    use crate::interface::server::tests::*;

    fn preferences(tile_size: u32) -> serde_json::Value {
        json!({
            "wide": {"tileSize": tile_size, "thumbnailShape": "fit", "view": "list", "originalImage": "auto", "filmstripSize": 3},
            "narrow": {"tileSize": 2, "thumbnailShape": "crop", "view": "grid", "originalImage": "onZoom", "filmstripSize": 3},
            "sort": {"key": "modified_at", "order": "desc"},
        })
    }

    #[tokio::test]
    async fn preferences_default_save_and_reject_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        assert_eq!(send(&router, Method::GET, "/api/v1/preferences", &[], None).await?.status, StatusCode::UNAUTHORIZED);
        let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "bearer")).await?;
        let bearer = bearer(&setup)?;
        let headers = &[("authorization", bearer.as_str())];

        let defaults = send(&router, Method::GET, "/api/v1/preferences", headers, None).await?;
        assert_eq!(defaults.status, StatusCode::OK);
        assert_eq!(
            defaults.body["wide"],
            json!({"tileSize": 3, "thumbnailShape": "crop", "view": "grid", "originalImage": "auto", "filmstripSize": 3})
        );
        assert_eq!(defaults.body["narrow"]["originalImage"], "onZoom");
        assert_eq!(defaults.body["sort"], json!({"key": "name", "order": "asc"}));

        let saved = send(&router, Method::PUT, "/api/v1/preferences", headers, Some(preferences(5))).await?;
        assert_eq!(saved.status, StatusCode::OK);
        assert_eq!(saved.body, preferences(5));
        assert_eq!(send(&router, Method::GET, "/api/v1/preferences", headers, None).await?.body, preferences(5));

        for tile_size in [0, 6, 300] {
            let rejected = send(&router, Method::PUT, "/api/v1/preferences", headers, Some(preferences(tile_size))).await?;
            // 範囲は契約に書いてあり、生成されたルータが本文を読んだ時点で拒否する。その平文の 400 は、外側の middleware が `Problem` にする。
            assert_eq!(rejected.status, StatusCode::BAD_REQUEST, "{tile_size}");
        }
        for filmstrip_size in [0, 6] {
            let mut body = preferences(5);
            body["narrow"]["filmstripSize"] = json!(filmstrip_size);
            let rejected = send(&router, Method::PUT, "/api/v1/preferences", headers, Some(body)).await?;
            assert_eq!(rejected.status, StatusCode::BAD_REQUEST, "{filmstrip_size}");
        }
        let mut missing = preferences(5);
        missing["wide"].as_object_mut().unwrap().remove("filmstripSize");
        let rejected = send(&router, Method::PUT, "/api/v1/preferences", headers, Some(missing)).await?;
        assert!(rejected.status.is_client_error(), "filmstripSize is required: {}", rejected.status);
        assert_eq!(
            send(&router, Method::GET, "/api/v1/preferences", headers, None).await?.body,
            preferences(5),
            "拒否した保存は値を変えない"
        );

        let malformed = send(&router, Method::PUT, "/api/v1/preferences", headers, Some(json!({"wide": 1}))).await?;
        assert!(malformed.status.is_client_error(), "形の違う本文は拒否される: {}", malformed.status);
        Ok(())
    }

    #[tokio::test]
    async fn preferences_hold_how_an_archive_opens_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "bearer")).await?;
        let bearer = bearer(&setup)?;
        let headers = &[("authorization", bearer.as_str())];

        let defaults = send(&router, Method::GET, "/api/v1/preferences", headers, None).await?;
        assert!(defaults.body.get("archiveOpen").is_none(), "never saved: {}", defaults.body);

        for value in ["folder", "viewer"] {
            let mut body = preferences(3);
            body["archiveOpen"] = json!(value);
            let saved = send(&router, Method::PUT, "/api/v1/preferences", headers, Some(body.clone())).await?;
            assert_eq!(saved.status, StatusCode::OK, "{value}");
            assert_eq!(saved.body, body);
            assert_eq!(send(&router, Method::GET, "/api/v1/preferences", headers, None).await?.body, body);
        }

        let mut unknown = preferences(3);
        unknown["archiveOpen"] = json!("window");
        let rejected = send(&router, Method::PUT, "/api/v1/preferences", headers, Some(unknown)).await?;
        // The generated router rejects an unknown enum value while it reads the body, as for every enum of the contract.
        assert_eq!(rejected.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(send(&router, Method::GET, "/api/v1/preferences", headers, None).await?.body["archiveOpen"], "viewer");

        // A save without `archiveOpen` unsets it.
        send(&router, Method::PUT, "/api/v1/preferences", headers, Some(preferences(3))).await?;
        let replaced = send(&router, Method::GET, "/api/v1/preferences", headers, None).await?;
        assert!(replaced.body.get("archiveOpen").is_none(), "a save without it unsets it: {}", replaced.body);
        Ok(())
    }

    #[tokio::test]
    async fn preferences_survive_a_new_daemon_state_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "bearer")).await?;
        let bearer = bearer(&setup)?;
        let headers = &[("authorization", bearer.as_str())];
        send(&router, Method::PUT, "/api/v1/preferences", headers, Some(preferences(4))).await?;

        // 同じ state_dir で作り直した状態も、保存した値を返す。
        let reopened = crate::state::DaemonState::new(fixture.server.state.conf.clone()).await?;
        let stored = serde_json::to_value(PreferencesModel::to_model(reopened.preferences.get().await?))?;
        assert_eq!(stored, preferences(4));
        Ok(())
    }

    #[tokio::test]
    async fn preferences_put_from_a_cookie_session_requires_the_same_origin_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "cookie")).await?;
        let cookie = cookie(&setup)?;
        let foreign = send(
            &router,
            Method::PUT,
            "/api/v1/preferences",
            &[("cookie", cookie.as_str()), ("origin", "http://evil.example")],
            Some(preferences(3)),
        )
        .await?;
        assert_eq!(foreign.status, StatusCode::FORBIDDEN);
        let same = send(
            &router,
            Method::PUT,
            "/api/v1/preferences",
            &[("cookie", cookie.as_str()), ("origin", "http://localhost")],
            Some(preferences(3)),
        )
        .await?;
        assert_eq!(same.status, StatusCode::OK);
        Ok(())
    }
}
