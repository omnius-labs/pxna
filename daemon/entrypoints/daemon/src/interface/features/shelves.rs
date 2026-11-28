use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_engine::model::{DisplayLayout, DisplayValueSet, DisplayValues, FolderSetting, Shelf, ShelfFilter, ShelfInput, ShelfScope, ShelfStart};
use omnius_pxna_interface::{apis, models};

use crate::{
    config::LibraryConfig,
    error::{ApiError, ApiStatus},
    interface::{ApiServer, features::preferences::PreferencesModel, session::Claims},
    prelude::*,
    service::{
        library::LibraryRelativePath,
        listing::{ListOptions, MAX_STARTS, Scope},
    },
};

#[allow(unused_variables)]
#[async_trait]
impl apis::shelves::Shelves for ApiServer {
    type Claims = Claims;

    async fn list_shelves(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> std::result::Result<apis::shelves::ListShelvesResponse, ()> {
        let starts = self.default_shelf_starts().await;
        Ok(match self.state.shelves.list(&claims.user.id, &starts).await {
            Ok(shelves) => apis::shelves::ListShelvesResponse::Status200_TheShelves(ShelfModel::list_to_model(shelves)),
            Err(error) => ApiError::from(Error::from(error)).into(),
        })
    }

    async fn create_shelf(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        body: &models::ShelfInput,
    ) -> std::result::Result<apis::shelves::CreateShelfResponse, ()> {
        let created = async {
            let input = ShelfModel::input_from_model(body)?;
            Ok::<_, Error>(self.state.shelves.create(&claims.user.id, &input).await?)
        }
        .await;
        Ok(match created {
            Ok(shelf) => apis::shelves::CreateShelfResponse::Status201_TheCreatedShelf(ShelfModel::to_model(shelf)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn replace_shelf(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::ReplaceShelfPathParams,
        body: &models::ShelfInput,
    ) -> std::result::Result<apis::shelves::ReplaceShelfResponse, ()> {
        let replaced = async {
            let input = ShelfModel::input_from_model(body)?;
            Ok::<_, Error>(self.state.shelves.replace(&claims.user.id, &path_params.shelf_id, &input).await?)
        }
        .await;
        Ok(match replaced {
            Ok(shelf) => apis::shelves::ReplaceShelfResponse::Status200_TheSavedShelf(ShelfModel::to_model(shelf)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn delete_shelf(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::DeleteShelfPathParams,
    ) -> std::result::Result<apis::shelves::DeleteShelfResponse, ()> {
        Ok(match self.state.shelves.delete(&claims.user.id, &path_params.shelf_id).await {
            Ok(()) => apis::shelves::DeleteShelfResponse::Status204_TheShelfWasDeleted,
            Err(error) => ApiError::from(Error::from(error)).into(),
        })
    }

    async fn reorder_shelves(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        body: &models::ShelfOrder,
    ) -> std::result::Result<apis::shelves::ReorderShelvesResponse, ()> {
        Ok(match self.state.shelves.reorder(&claims.user.id, &body.ids).await {
            Ok(shelves) => apis::shelves::ReorderShelvesResponse::Status200_TheShelvesInTheNewOrder(ShelfModel::list_to_model(shelves)),
            Err(error) => ApiError::from(Error::from(error)).into(),
        })
    }

    async fn list_folder_settings(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
    ) -> std::result::Result<apis::shelves::ListFolderSettingsResponse, ()> {
        Ok(match self.state.shelves.folder_settings(&claims.user.id).await {
            Ok(settings) => {
                let settings = settings.into_iter().map(ShelfModel::folder_setting_to_model).collect();
                apis::shelves::ListFolderSettingsResponse::Status200_TheFolderSettings(models::FolderSettingList::new(settings))
            }
            Err(error) => ApiError::from(Error::from(error)).into(),
        })
    }

    async fn put_folder_setting(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::PutFolderSettingPathParams,
        query_params: &models::PutFolderSettingQueryParams,
        body: &models::FolderSettingRequest,
    ) -> std::result::Result<apis::shelves::PutFolderSettingResponse, ()> {
        let saved = async {
            let setting = FolderSetting {
                library_id: path_params.library_id.clone(),
                path: ShelfModel::normal_path(&path_params.library_id, &query_params.path)?,
                display: ShelfModel::display_from_model(&body.display),
            };
            self.state.shelves.put_folder_setting(&claims.user.id, &setting).await?;
            Ok::<_, Error>(setting)
        }
        .await;
        Ok(match saved {
            Ok(setting) => apis::shelves::PutFolderSettingResponse::Status200_TheSavedSettings(ShelfModel::folder_setting_to_model(setting)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn delete_folder_setting(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::DeleteFolderSettingPathParams,
        query_params: &models::DeleteFolderSettingQueryParams,
    ) -> std::result::Result<apis::shelves::DeleteFolderSettingResponse, ()> {
        let deleted = async {
            let path = ShelfModel::normal_path(&path_params.library_id, &query_params.path)?;
            Ok::<_, Error>(self.state.shelves.delete_folder_setting(&claims.user.id, &path_params.library_id, &path).await?)
        }
        .await;
        Ok(match deleted {
            Ok(()) => apis::shelves::DeleteFolderSettingResponse::Status204_TheSettingsWereRemoved,
            Err(error) => ApiError::from(error).into(),
        })
    }
}

impl ApiServer {
    /// Where the default Shelf starts: the roots of the configured Libraries that `timelineLibraries` of the display preferences names, or of every one
    /// when none is saved or none of the saved ones is configured, in the order of the configuration. Preferences that cannot be read count as none saved.
    async fn default_shelf_starts(&self) -> Vec<ShelfStart> {
        let named = match self.state.preferences.get().await {
            Ok(preferences) => preferences.timeline_libraries.unwrap_or_default(),
            Err(error) => {
                warn!(?error, "shelves: cannot read the display preferences");
                Vec::new()
            }
        };
        let libraries = &self.state.conf.libraries;
        let any_named = libraries.iter().any(|library| named.contains(&library.id));
        libraries
            .iter()
            .filter(|library| !any_named || named.contains(&library.id))
            .take(MAX_STARTS)
            .map(|library| ShelfStart {
                library_id: library.id.clone(),
                path: "/".to_string(),
            })
            .collect()
    }
}

impl From<ApiError> for apis::shelves::ListShelvesResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::shelves::CreateShelfResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::shelves::ReplaceShelfResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::shelves::DeleteShelfResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::shelves::ReorderShelvesResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::shelves::ListFolderSettingsResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::shelves::PutFolderSettingResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::shelves::DeleteFolderSettingResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

/// Maps the Shelves and folder settings of the engine to and from the API's types, and checks what the engine cannot.
struct ShelfModel;

impl ShelfModel {
    /// The Shelf a request saves. Its Scope and Filter are checked as the body of `listEntryMonths` is, so that a saved Shelf can be sent as it is held,
    /// and each start's path is turned into normal form. Whether a start names anything is not checked.
    fn input_from_model(model: &models::ShelfInput) -> Result<ShelfInput> {
        let mut request = models::EntryMonthsRequest::new(model.scope.clone());
        request.filter = model.filter.clone();
        ListOptions::from_entry_months(&request)?;
        Scope::from_request(&model.scope)?;

        let starts = model
            .scope
            .starts
            .iter()
            .map(|start| {
                Ok(ShelfStart {
                    library_id: start.library_id.clone(),
                    path: LibraryRelativePath::parse(&start.path)?.absolute(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(ShelfInput {
            name: model.name.clone(),
            scope: ShelfScope {
                starts,
                recursive: model.scope.recursive.unwrap_or(false),
                inside_archives: model.scope.inside_archives.unwrap_or(false),
            },
            filter: model.filter.as_ref().map(Self::filter_from_model).unwrap_or_default(),
            display: model.display.as_ref().map(Self::display_from_model).unwrap_or_default(),
        })
    }

    /// The normal form of the path of a folder of the folder settings.
    fn normal_path(library_id: &str, path: &str) -> Result<String> {
        if !LibraryConfig::is_valid_id(library_id) {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("not a valid library id: {library_id:?}")));
        }
        Ok(LibraryRelativePath::parse(path)?.absolute())
    }

    fn list_to_model(shelves: Vec<Shelf>) -> models::ShelfList {
        models::ShelfList::new(shelves.into_iter().map(Self::to_model).collect())
    }

    fn to_model(shelf: Shelf) -> models::Shelf {
        let starts = shelf.scope.starts.into_iter().map(|start| models::LibraryPath::new(start.library_id, start.path)).collect();
        let mut scope = models::EntryScope::new(starts);
        scope.recursive = Some(shelf.scope.recursive);
        scope.inside_archives = Some(shelf.scope.inside_archives);

        let mut filter = models::EntryFilter::new();
        filter.query = shelf.filter.query;
        filter.rating = shelf.filter.rating;
        filter.tags = shelf.filter.tags.map(|tags| tags.into_iter().map(models::TagName).collect());
        filter.kinds = shelf.filter.kinds;

        models::Shelf::new(shelf.id, shelf.name, scope, filter, Self::display_to_model(shelf.display))
    }

    fn filter_from_model(model: &models::EntryFilter) -> ShelfFilter {
        ShelfFilter {
            query: model.query.clone(),
            rating: model.rating.clone(),
            tags: model.tags.as_ref().map(|tags| tags.iter().map(|tag| tag.0.clone()).collect()),
            kinds: model.kinds.clone(),
        }
    }

    fn folder_setting_to_model(setting: FolderSetting) -> models::FolderSetting {
        models::FolderSetting::new(setting.library_id, setting.path, Self::display_to_model(setting.display))
    }

    fn display_to_model(display: DisplayValues) -> models::DisplayValues {
        let mut model = models::DisplayValues::new();
        model.wide = display.wide.map(Self::set_to_model);
        model.narrow = display.narrow.map(Self::set_to_model);
        model.sort = display.sort.map(PreferencesModel::sort_to_model);
        model.archive_open = display.archive_open.map(PreferencesModel::archive_open_to_model);
        model
    }

    fn display_from_model(model: &models::DisplayValues) -> DisplayValues {
        DisplayValues {
            wide: model.wide.as_ref().map(Self::set_from_model),
            narrow: model.narrow.as_ref().map(Self::set_from_model),
            sort: model.sort.as_ref().map(PreferencesModel::sort_from_model),
            archive_open: model.archive_open.map(PreferencesModel::archive_open_from_model),
        }
    }

    fn set_to_model(set: DisplayValueSet) -> models::DisplayValueSet {
        let mut model = models::DisplayValueSet::new();
        model.view = set.view.map(|view| match view {
            DisplayLayout::Grid => models::DisplayLayout::Grid,
            DisplayLayout::List => models::DisplayLayout::List,
            DisplayLayout::Timeline => models::DisplayLayout::Timeline,
        });
        model.tile_size = set.tile_size.map(u32::from);
        model.thumbnail_shape = set.thumbnail_shape.map(PreferencesModel::thumbnail_shape_to_model);
        model
    }

    fn set_from_model(model: &models::DisplayValueSet) -> DisplayValueSet {
        DisplayValueSet {
            view: model.view.map(|view| match view {
                models::DisplayLayout::Grid => DisplayLayout::Grid,
                models::DisplayLayout::List => DisplayLayout::List,
                models::DisplayLayout::Timeline => DisplayLayout::Timeline,
            }),
            // A value that does not fit in u8 is out of range, which the engine rejects.
            tile_size: model.tile_size.map(|size| u8::try_from(size).unwrap_or(u8::MAX)),
            thumbnail_shape: model.thumbnail_shape.map(PreferencesModel::thumbnail_shape_from_model),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use http::StatusCode;
    use omnius_pxna_engine::service::ArchiveOptions;
    use serde_json::{Value, json};
    use testresult::TestResult;

    use super::*;
    use crate::{config::DaemonConfig, db::DatabaseBootstrap, interface::server::tests::*, state::DaemonState};

    /// A router over its own state, with an owner signed in by a Bearer token.
    struct Client {
        fixture: Fixture,
        router: axum::Router,
        bearer: String,
    }

    impl Client {
        async fn new() -> TestResult<Self> {
            let fixture = fixture(ArchiveOptions::default()).await?;
            let router = router(&fixture);
            let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "bearer")).await?;
            let bearer = bearer(&setup)?;
            Ok(Self { fixture, router, bearer })
        }

        /// Another daemon over the same state, with the configuration changed.
        async fn restart(&mut self, change: impl FnOnce(&mut DaemonConfig)) -> TestResult {
            let mut conf = self.fixture.server.state.conf.clone();
            change(&mut conf);
            self.fixture.server = ApiServer {
                state: Arc::new(DaemonState::new(conf).await?),
            };
            self.router = router(&self.fixture);
            Ok(())
        }

        async fn send(&self, method: Method, path: &str, body: Option<Value>) -> TestResult<HttpResponse> {
            let headers = &[("authorization", self.bearer.as_str())];
            send(&self.router, method, &format!("/api/v1{path}"), headers, body).await
        }

        async fn shelves(&self) -> TestResult<Vec<Value>> {
            let response = self.send(Method::GET, "/shelves", None).await?;
            assert_eq!(response.status, StatusCode::OK, "{}", response.body);
            Ok(response.body["shelves"].as_array().ok_or("shelves is missing")?.clone())
        }

        async fn create(&self, body: Value) -> TestResult<Value> {
            let response = self.send(Method::POST, "/shelves", Some(body)).await?;
            assert_eq!(response.status, StatusCode::CREATED, "{}", response.body);
            Ok(response.body)
        }

        async fn folder_settings(&self) -> TestResult<Value> {
            let response = self.send(Method::GET, "/folder-settings", None).await?;
            assert_eq!(response.status, StatusCode::OK, "{}", response.body);
            Ok(response.body["settings"].clone())
        }
    }

    fn shelf(name: &str) -> Value {
        json!({"name": name, "scope": {"starts": [{"libraryId": "photos", "path": "/photos"}]}})
    }

    /// The path of the folder settings of `path`, relative to `/api/v1`.
    fn setting_uri(library_id: &str, path: &str) -> String {
        format!("/libraries/{library_id}/entries/folder-settings?path={path}")
    }

    fn place(library_id: &str, path: &str) -> Value {
        json!({"libraryId": library_id, "path": path})
    }

    fn names(shelves: &[Value]) -> Vec<&str> {
        shelves.iter().filter_map(|shelf| shelf["name"].as_str()).collect()
    }

    fn start_libraries(shelf: &Value) -> Vec<&str> {
        shelf["scope"]["starts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|start| start["libraryId"].as_str())
            .collect()
    }

    async fn save_timeline_libraries(client: &Client, ids: &[&str]) -> TestResult {
        let mut preferences = client.send(Method::GET, "/preferences", None).await?.body;
        preferences["timelineLibraries"] = json!(ids);
        let response = client.send(Method::PUT, "/preferences", Some(preferences)).await?;
        assert_eq!(response.status, StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn the_first_read_creates_the_default_shelf_once_test() -> TestResult {
        let client = Client::new().await?;
        let shelves = client.shelves().await?;
        assert_eq!(names(&shelves), ["Timeline"]);
        let default = &shelves[0];
        assert_eq!(
            default["scope"],
            json!({"starts": [place("photos", "/"), place("videos", "/")], "recursive": true, "insideArchives": true})
        );
        assert_eq!(default["filter"], json!({"kinds": ["image", "video"]}));
        assert_eq!(default["display"], json!({"wide": {"view": "timeline"}, "narrow": {"view": "timeline"}}));

        let deleted = client.send(Method::DELETE, &format!("/shelves/{}", default["id"].as_str().ok_or("id")?), None).await?;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        assert!(client.shelves().await?.is_empty(), "it is not created again");
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn two_first_reads_at_once_create_one_default_shelf_test() -> TestResult {
        let client = Client::new().await?;
        let (first, second) = tokio::join!(client.shelves(), client.shelves());
        let (first, second) = (first?, second?);
        assert_eq!(names(&first), ["Timeline"]);
        assert_eq!(first, second);
        assert_eq!(client.shelves().await?.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn the_default_shelf_starts_follow_the_preferences_and_the_configuration_test() -> TestResult {
        for (saved, expected) in [
            (vec!["videos"], vec!["videos"]),
            (vec!["videos", "gone", "photos"], vec!["photos", "videos"]),
            (vec!["gone"], vec!["photos", "videos"]),
        ] {
            let client = Client::new().await?;
            save_timeline_libraries(&client, &saved).await?;
            assert_eq!(start_libraries(&client.shelves().await?[0]), expected, "{saved:?}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn preferences_that_cannot_be_read_count_as_none_saved_test() -> TestResult {
        let client = Client::new().await?;
        let db = DatabaseBootstrap::connect(&client.fixture.server.state.conf.core.state_dir).await?;
        sqlx::query("INSERT INTO preferences (id, value) VALUES (1, 'broken')").execute(&db).await?;
        assert_eq!(client.send(Method::GET, "/preferences", None).await?.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(start_libraries(&client.shelves().await?[0]), ["photos", "videos"]);
        Ok(())
    }

    #[tokio::test]
    async fn the_default_shelf_waits_for_a_library_and_starts_at_the_first_of_them_test() -> TestResult {
        let mut client = Client::new().await?;
        client.restart(|conf| conf.libraries.clear()).await?;
        assert!(client.shelves().await?.is_empty());

        let root = PathBuf::from(&client.fixture._dir.path()).join("library");
        let ids = (0..MAX_STARTS + 6).map(|index| format!("lib{index:02}")).collect::<Vec<_>>();
        let libraries = ids
            .iter()
            .map(|id| LibraryConfig {
                id: id.clone(),
                name: id.clone(),
                path: root.clone(),
                readonly: true,
            })
            .collect::<Vec<_>>();
        client.restart(|conf| conf.libraries = libraries).await?;
        let shelves = client.shelves().await?;
        assert_eq!(names(&shelves), ["Timeline"]);
        assert_eq!(start_libraries(&shelves[0]), ids[..MAX_STARTS].iter().map(String::as_str).collect::<Vec<_>>());
        Ok(())
    }

    #[tokio::test]
    async fn shelves_are_created_replaced_reordered_and_deleted_test() -> TestResult {
        let client = Client::new().await?;
        let default = client.shelves().await?[0].clone();
        let full = json!({
            "name": "  Trips  ",
            "scope": {
                "starts": [place("photos", "/photos/../a/./b/"), place("photos", "/a/b"), place("gone", "/missing")],
                "recursive": true,
            },
            "filter": {"query": "sea", "rating": "none", "tags": [" trip "], "kinds": ["folder", "other"]},
            "display": {
                "wide": {"view": "timeline", "tileSize": 5, "thumbnailShape": "fit"},
                "narrow": {"view": "list"},
                "sort": {"key": "modified_at", "order": "desc"},
                "archiveOpen": "viewer",
            },
        });
        let created = client.create(full.clone()).await?;
        assert_eq!(created["name"], "Trips");
        assert_eq!(
            created["scope"],
            json!({
                "starts": [place("photos", "/a/b"), place("photos", "/a/b"), place("gone", "/missing")],
                "recursive": true,
                "insideArchives": false,
            }),
            "a start that names nothing and a start given twice are saved, in normal form"
        );
        assert_eq!(created["filter"], full["filter"]);
        assert_eq!(created["display"], full["display"]);
        let plain = client.create(shelf("Plain")).await?;
        assert_eq!((&plain["filter"], &plain["display"]), (&json!({}), &json!({})));
        assert_eq!(client.shelves().await?, vec![default.clone(), created.clone(), plain.clone()]);

        let id = created["id"].as_str().ok_or("id")?;
        let replaced = client.send(Method::PUT, &format!("/shelves/{id}"), Some(shelf("Replaced"))).await?;
        assert_eq!(replaced.status, StatusCode::OK, "{}", replaced.body);
        assert_eq!((&replaced.body["id"], &replaced.body["filter"]), (&created["id"], &json!({})));
        assert_eq!(client.shelves().await?[1], replaced.body, "the position is kept");

        let ids = [&plain["id"], &replaced.body["id"], &default["id"]];
        let reordered = client.send(Method::PUT, "/shelves/order", Some(json!({"ids": ids}))).await?;
        assert_eq!(reordered.status, StatusCode::OK, "{}", reordered.body);
        assert_eq!(names(reordered.body["shelves"].as_array().ok_or("shelves")?), ["Plain", "Replaced", "Timeline"]);
        assert_eq!(client.shelves().await?, *reordered.body["shelves"].as_array().ok_or("shelves")?);
        for refused in [json!([ids[0], ids[1]]), json!([ids[0], ids[0], ids[1], ids[2]]), json!([ids[0], ids[1], "shf_unknown"])] {
            let response = client.send(Method::PUT, "/shelves/order", Some(json!({ "ids": refused }))).await?;
            assert_eq!(response.status, StatusCode::BAD_REQUEST, "{refused}");
            assert_eq!(response.body["code"], "InvalidRequest", "{refused}");
        }

        let path = format!("/shelves/{id}");
        assert_eq!(client.send(Method::DELETE, &path, None).await?.status, StatusCode::NO_CONTENT);
        assert_eq!(names(&client.shelves().await?), ["Plain", "Timeline"]);
        for response in [client.send(Method::DELETE, &path, None).await?, client.send(Method::PUT, &path, Some(shelf("x"))).await?] {
            assert_eq!((response.status, response.body["code"].as_str()), (StatusCode::NOT_FOUND, Some("NotFound")));
        }
        Ok(())
    }

    #[tokio::test]
    async fn another_users_shelves_are_not_served_test() -> TestResult {
        let client = Client::new().await?;
        let start = ShelfStart {
            library_id: "photos".to_string(),
            path: "/".to_string(),
        };
        let theirs = client.fixture.server.state.shelves.create("usr_other", &ShelfInput::default_shelf(&[start])).await?;
        let mine = client.shelves().await?;
        assert_eq!(mine.len(), 1);
        assert_ne!(mine[0]["id"], json!(theirs.id));
        let path = format!("/shelves/{}", theirs.id);
        assert_eq!(client.send(Method::PUT, &path, Some(shelf("x"))).await?.status, StatusCode::NOT_FOUND);
        assert_eq!(client.send(Method::DELETE, &path, None).await?.status, StatusCode::NOT_FOUND);
        let order = json!({"ids": [mine[0]["id"], theirs.id]});
        assert_eq!(client.send(Method::PUT, "/shelves/order", Some(order)).await?.status, StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[tokio::test]
    async fn a_shelf_that_list_entry_months_would_refuse_is_not_saved_test() -> TestResult {
        let client = Client::new().await?;
        let saved = client.create(shelf("saved")).await?;
        let path = format!("/shelves/{}", saved["id"].as_str().ok_or("id")?);
        let start = place("photos", "/");
        let refused = [
            json!({"scope": {"starts": []}}),
            json!({"scope": {"starts": vec![start.clone(); MAX_STARTS + 1]}}),
            json!({"scope": {"starts": [place("..", "/")]}}),
            json!({"scope": {"starts": [place("a/b", "/")]}}),
            json!({"scope": {"starts": [place("photos", "/..")]}}),
            json!({"scope": {"starts": [place("photos", "a\\b")]}}),
            json!({"scope": {"starts": [start]}, "filter": {"rating": "great"}}),
            json!({"scope": {"starts": [start]}, "filter": {"tags": ["   "]}}),
            json!({"scope": {"starts": [start]}, "filter": {"kinds": ["movie"]}}),
        ];
        for body in refused {
            let months = client.send(Method::POST, "/entries/months", Some(body.clone())).await?;
            assert_eq!(months.status, StatusCode::BAD_REQUEST, "{body}");
            let mut body = body;
            body["name"] = json!("refused");
            for response in [
                client.send(Method::POST, "/shelves", Some(body.clone())).await?,
                client.send(Method::PUT, &path, Some(body.clone())).await?,
            ] {
                assert_eq!(response.status, StatusCode::BAD_REQUEST, "{body}");
            }
        }

        for name in ["", "   ", &"x".repeat(101)] {
            let response = client.send(Method::POST, "/shelves", Some(shelf(name))).await?;
            assert_eq!(
                (response.status, response.body["code"].as_str()),
                (StatusCode::BAD_REQUEST, Some("InvalidRequest")),
                "{name:?}"
            );
        }
        let mut tile = shelf("tile");
        tile["display"] = json!({"wide": {"tileSize": 6}});
        assert_eq!(client.send(Method::POST, "/shelves", Some(tile)).await?.status, StatusCode::BAD_REQUEST);
        assert_eq!(names(&client.shelves().await?), ["saved", "Timeline"], "nothing refused is saved");
        Ok(())
    }

    #[tokio::test]
    async fn the_201st_shelf_is_refused_test() -> TestResult {
        let client = Client::new().await?;
        for index in client.shelves().await?.len()..200 {
            client.create(shelf(&format!("shelf {index}"))).await?;
        }
        let response = client.send(Method::POST, "/shelves", Some(shelf("one too many"))).await?;
        assert_eq!((response.status, response.body["code"].as_str()), (StatusCode::BAD_REQUEST, Some("InvalidRequest")));
        assert_eq!(client.shelves().await?.len(), 200);
        Ok(())
    }

    #[tokio::test]
    async fn folder_settings_are_saved_by_the_normal_form_and_removed_test() -> TestResult {
        let client = Client::new().await?;
        assert_eq!(client.folder_settings().await?, json!([]));
        let saved = client
            .send(
                Method::PUT,
                &setting_uri("photos", "x/../photos/a.7z/./dir/"),
                Some(json!({"display": {"wide": {"view": "list", "tileSize": 1}}})),
            )
            .await?;
        assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
        assert_eq!(
            saved.body,
            json!({"libraryId": "photos", "path": "/photos/a.7z/dir", "display": {"wide": {"view": "list", "tileSize": 1}}})
        );

        let replacement = json!({"libraryId": "photos", "path": "/photos/a.7z/dir", "display": {"archiveOpen": "folder"}});
        assert_eq!(
            client
                .send(Method::PUT, &setting_uri("photos", "/photos/a.7z/dir"), Some(json!({"display": {"archiveOpen": "folder"}})))
                .await?
                .status,
            StatusCode::OK
        );
        assert_eq!(client.folder_settings().await?, json!([replacement]));

        for (library_id, path, display) in [
            ("photos", "/photos/a.7z/dir", json!({"wide": {"view": "timeline"}})),
            ("photos", "/photos/a.7z/dir", json!({"narrow": {"view": "timeline"}})),
            ("photos", "/..", json!({})),
            ("..", "/", json!({})),
        ] {
            let response = client.send(Method::PUT, &setting_uri(library_id, path), Some(json!({"display": display}))).await?;
            assert_eq!(
                (response.status, response.body["code"].as_str()),
                (StatusCode::BAD_REQUEST, Some("InvalidRequest")),
                "{library_id} {path} {display}"
            );
        }
        assert_eq!(client.folder_settings().await?, json!([replacement]));

        let target = setting_uri("photos", "/photos/./a.7z/dir");
        assert_eq!(client.send(Method::DELETE, &target, None).await?.status, StatusCode::NO_CONTENT);
        assert_eq!(client.folder_settings().await?, json!([]));
        let missing = client.send(Method::DELETE, &target, None).await?;
        assert_eq!((missing.status, missing.body["code"].as_str()), (StatusCode::NOT_FOUND, Some("NotFound")));
        let invalid = client.send(Method::DELETE, &setting_uri("photos", "/.."), None).await?;
        assert_eq!(invalid.status, StatusCode::BAD_REQUEST);
        Ok(())
    }

    /// The Libraries `photos` and `videos`, both writable, with a Shelf and folder settings that name folders and archives of them.
    struct Tree {
        client: Client,
        photos: PathBuf,
        videos: PathBuf,
    }

    impl Tree {
        /// Makes the files and folders under `photos`, then saves settings of tile size 1, 2, ... for `settings` and a Shelf that starts at `starts`.
        async fn new(files: &[&str], settings: &[(&str, &str)], starts: &[(&str, &str)]) -> TestResult<Self> {
            let mut client = Client::new().await?;
            client.restart(|conf| conf.libraries[1].readonly = false).await?;
            let libraries = &client.fixture.server.state.conf.libraries;
            let (photos, videos) = (libraries[0].path.clone(), libraries[1].path.clone());
            for file in files {
                let path = photos.join(file);
                std::fs::create_dir_all(path.parent().ok_or("parent")?)?;
                std::fs::write(path, file.as_bytes())?;
            }
            let tree = Self { client, photos, videos };
            for (index, (library_id, path)) in settings.iter().enumerate() {
                tree.save_setting(library_id, path, index + 1).await?;
            }
            let starts = starts.iter().map(|(library_id, path)| place(library_id, path)).collect::<Vec<_>>();
            // The default Shelf is left uncreated, so the only Shelf is this one.
            tree.client.create(json!({"name": "s", "scope": {"starts": starts}})).await?;
            Ok(tree)
        }

        async fn save_setting(&self, library_id: &str, path: &str, tile_size: usize) -> TestResult {
            let body = json!({"display": {"wide": {"tileSize": tile_size}}});
            let response = self.client.send(Method::PUT, &setting_uri(library_id, path), Some(body)).await?;
            assert_eq!(response.status, StatusCode::OK, "{}", response.body);
            Ok(())
        }

        /// Sends `POST /entries/{command}`, and waits for its Operation to finish.
        async fn mutate(&self, command: &str, body: Value) -> TestResult {
            let response = self.client.send(Method::POST, &format!("/entries/{command}"), Some(body)).await?;
            assert_eq!(response.status, StatusCode::ACCEPTED, "{command}: {}", response.body);
            let id = response.body["operation"]["id"].as_str().ok_or("operation id")?;
            wait_operation(&self.client.fixture.server.state, id).await?;
            Ok(())
        }

        /// The folder settings as their Library, path, and tile size.
        async fn settings(&self) -> TestResult<Vec<(String, String, u64)>> {
            Ok(self
                .client
                .folder_settings()
                .await?
                .as_array()
                .ok_or("settings")?
                .iter()
                .map(|setting| {
                    (
                        setting["libraryId"].as_str().unwrap_or_default().to_string(),
                        setting["path"].as_str().unwrap_or_default().to_string(),
                        setting["display"]["wide"]["tileSize"].as_u64().unwrap_or_default(),
                    )
                })
                .collect())
        }

        async fn starts(&self) -> TestResult<Vec<(String, String)>> {
            let response = self.client.send(Method::GET, "/shelves", None).await?;
            let shelf = response.body["shelves"][0].clone();
            assert_eq!(shelf["name"], "s", "{}", response.body);
            Ok(shelf["scope"]["starts"]
                .as_array()
                .ok_or("starts")?
                .iter()
                .map(|start| {
                    (
                        start["libraryId"].as_str().unwrap_or_default().to_string(),
                        start["path"].as_str().unwrap_or_default().to_string(),
                    )
                })
                .collect())
        }
    }

    fn rows(values: &[(&str, &str, u64)]) -> Vec<(String, String, u64)> {
        values
            .iter()
            .map(|(library_id, path, tile_size)| (library_id.to_string(), path.to_string(), *tile_size))
            .collect()
    }

    fn places(values: &[(&str, &str)]) -> Vec<(String, String)> {
        values.iter().map(|(library_id, path)| (library_id.to_string(), path.to_string())).collect()
    }

    #[tokio::test]
    async fn settings_and_starts_follow_a_rename_and_a_move_within_a_library_test() -> TestResult {
        let at = [("photos", "/a"), ("photos", "/a/sub"), ("photos", "/ab")];
        let tree = Tree::new(&["a/sub/x.png", "ab/y.png", "into/z.png"], &at, &at).await?;

        tree.mutate(
            "move",
            json!({"sources": [{"libraryId": "photos", "path": "/a", "view": "folder"}], "destination": {"libraryId": "photos", "path": "/b"}}),
        )
        .await?;
        assert_eq!(tree.settings().await?, rows(&[("photos", "/ab", 3), ("photos", "/b", 1), ("photos", "/b/sub", 2)]));
        assert_eq!(tree.starts().await?, places(&[("photos", "/b"), ("photos", "/b/sub"), ("photos", "/ab")]));

        tree.mutate(
            "move",
            json!({"sources": [{"libraryId": "photos", "path": "/b", "view": "folder"}], "destination": {"libraryId": "photos", "folderPath": "/into"}}),
        )
        .await?;
        assert_eq!(
            tree.settings().await?,
            rows(&[("photos", "/ab", 3), ("photos", "/into/b", 1), ("photos", "/into/b/sub", 2)])
        );
        assert_eq!(tree.starts().await?, places(&[("photos", "/into/b"), ("photos", "/into/b/sub"), ("photos", "/ab")]));
        Ok(())
    }

    #[tokio::test]
    async fn settings_and_starts_follow_a_move_across_libraries_test() -> TestResult {
        let at = [("photos", "/a"), ("photos", "/a/sub"), ("photos", "/a/empty"), ("photos", "/a/pack.7z/dir")];
        let tree = Tree::new(&["a/sub/x.png"], &at, &at).await?;
        std::fs::create_dir_all(tree.photos.join("a/empty"))?;
        std::fs::copy(&tree.client.fixture.archive, tree.photos.join("a/pack.7z"))?;
        // Settings left at the destination by a folder removed outside Pxna do not apply to the folder moved there.
        tree.save_setting("videos", "/a/gone", 5).await?;

        tree.mutate(
            "move",
            json!({"sources": [{"libraryId": "photos", "path": "/a", "view": "folder"}], "destination": {"libraryId": "videos", "path": "/a"}}),
        )
        .await?;
        assert!(tree.videos.join("a/sub/x.png").is_file());
        assert_eq!(
            tree.settings().await?,
            rows(&[("videos", "/a", 1), ("videos", "/a/empty", 3), ("videos", "/a/pack.7z/dir", 4), ("videos", "/a/sub", 2)])
        );
        assert_eq!(
            tree.starts().await?,
            places(&[("videos", "/a"), ("videos", "/a/sub"), ("videos", "/a/empty"), ("videos", "/a/pack.7z/dir")])
        );
        Ok(())
    }

    #[tokio::test]
    async fn settings_and_starts_follow_a_move_of_several_folders_test() -> TestResult {
        let at = [("photos", "/a"), ("photos", "/b")];
        let tree = Tree::new(&["a/x.png", "b/y.png", "into/z.png"], &at, &at).await?;

        tree.mutate("move", json!({"sources": [{"libraryId": "photos", "path": "/a", "view": "folder"}, {"libraryId": "photos", "path": "/b", "view": "folder"}], "destination": {"libraryId": "photos", "folderPath": "/into"}})).await?;
        assert_eq!(tree.settings().await?, rows(&[("photos", "/into/a", 1), ("photos", "/into/b", 2)]));
        assert_eq!(tree.starts().await?, places(&[("photos", "/into/a"), ("photos", "/into/b")]));
        Ok(())
    }

    #[tokio::test]
    async fn a_folder_moved_into_an_existing_folder_leaves_its_settings_test() -> TestResult {
        for conflict_target_has_settings in [true, false] {
            let at = [("photos", "/src"), ("photos", "/src/sub")];
            let tree = Tree::new(&["src/sub/x.png", "src/y.png", "dst/z.png"], &at, &at).await?;
            if conflict_target_has_settings {
                tree.save_setting("photos", "/dst", 5).await?;
            }

            tree.mutate(
                "move",
                json!({"sources": [{"libraryId": "photos", "path": "/src", "view": "folder"}], "destination": {"libraryId": "photos", "path": "/dst"}, "conflict": "overwrite"}),
            )
            .await?;
            assert!(tree.photos.join("dst/sub/x.png").is_file() && !tree.photos.join("src").exists());
            let kept = if conflict_target_has_settings { 5 } else { 1 };
            assert_eq!(tree.settings().await?, rows(&[("photos", "/dst", kept), ("photos", "/dst/sub", 2)]));
            assert_eq!(tree.starts().await?, places(&[("photos", "/dst"), ("photos", "/dst/sub")]));
        }
        Ok(())
    }

    #[tokio::test]
    async fn a_skipped_folder_keeps_its_settings_test() -> TestResult {
        let at = [("photos", "/a"), ("photos", "/b")];
        let tree = Tree::new(&["a/x.png", "b/y.png", "into/a/z.png"], &at, &at).await?;

        tree.mutate("move", json!({"sources": [{"libraryId": "photos", "path": "/a", "view": "folder"}, {"libraryId": "photos", "path": "/b", "view": "folder"}], "destination": {"libraryId": "photos", "folderPath": "/into"}, "conflict": "skip"}))
            .await?;
        assert!(tree.photos.join("a/x.png").is_file() && tree.photos.join("into/b/y.png").is_file());
        assert_eq!(tree.settings().await?, rows(&[("photos", "/a", 1), ("photos", "/into/b", 2)]));
        assert_eq!(tree.starts().await?, places(&[("photos", "/a"), ("photos", "/into/b")]));
        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_folder_left_by_a_failed_entry_keeps_its_settings_while_what_left_it_follows_test() -> TestResult {
        let at = [("photos", "/a"), ("photos", "/a/done"), ("photos", "/a/kept")];
        let tree = Tree::new(&["a/done/x.png", "a/kept/y.png"], &at, &at).await?;
        // A symbolic link cannot be traversed, so it fails and holds `kept` and `a` at the source.
        std::os::unix::fs::symlink("y.png", tree.photos.join("a/kept/link"))?;

        tree.mutate(
            "move",
            json!({"sources": [{"libraryId": "photos", "path": "/a", "view": "folder"}], "destination": {"libraryId": "videos", "path": "/a"}}),
        )
        .await?;
        assert!(tree.photos.join("a/kept").is_dir() && !tree.photos.join("a/done").exists());
        assert!(tree.videos.join("a/kept/y.png").is_file());
        assert_eq!(tree.settings().await?, rows(&[("photos", "/a", 1), ("photos", "/a/kept", 3), ("videos", "/a/done", 2)]));
        assert_eq!(tree.starts().await?, places(&[("photos", "/a"), ("videos", "/a/done"), ("photos", "/a/kept")]));
        Ok(())
    }

    #[tokio::test]
    async fn a_move_that_replaces_an_archive_drops_its_settings_and_keeps_its_starts_test() -> TestResult {
        // A file takes the place of one archive, and a folder moved entry by entry takes the place of another.
        let tree = Tree::new(
            &["new.7z", "pack.7z/x.png", "into/pack.7z"],
            &[
                ("photos", "/photos/a.7z/dir"),
                ("photos", "/into/pack.7z/inside"),
                ("photos", "/new.7z"),
                ("photos", "/pack.7z"),
            ],
            &[("photos", "/photos/a.7z"), ("photos", "/into/pack.7z"), ("photos", "/new.7z")],
        )
        .await?;
        std::fs::create_dir_all(tree.photos.join("other"))?;

        tree.mutate("move", json!({"sources": [{"libraryId": "photos", "path": "/new.7z", "view": "file"}], "destination": {"libraryId": "photos", "path": "/photos/a.7z"}, "conflict": "overwrite"}))
            .await?;
        tree.mutate(
            "move",
            json!({"sources": [{"libraryId": "photos", "path": "/pack.7z", "view": "folder"}, {"libraryId": "photos", "path": "/other", "view": "folder"}], "destination": {"libraryId": "photos", "folderPath": "/into"}, "conflict": "overwrite"}),
        )
        .await?;
        assert!(tree.photos.join("into/pack.7z/x.png").is_file());
        assert_eq!(tree.settings().await?, rows(&[("photos", "/into/pack.7z", 4), ("photos", "/photos/a.7z", 3)]));
        assert_eq!(
            tree.starts().await?,
            places(&[("photos", "/photos/a.7z"), ("photos", "/into/pack.7z"), ("photos", "/photos/a.7z")])
        );
        Ok(())
    }

    #[tokio::test]
    async fn a_delete_removes_settings_and_keeps_starts_test() -> TestResult {
        let at = [("photos", "/a"), ("photos", "/a/sub"), ("photos", "/b"), ("photos", "/ab")];
        let tree = Tree::new(&["a/sub/x.png", "ab/y.png"], &at, &at).await?;
        std::fs::create_dir_all(tree.photos.join("b"))?;

        tree.mutate("delete", json!({"targets": [{"libraryId": "photos", "path": "/a", "view": "folder"}], "recursive": true}))
            .await?;
        tree.mutate("delete", json!({"targets": [{"libraryId": "photos", "path": "/b", "view": "folder"}]})).await?;
        assert_eq!(tree.settings().await?, rows(&[("photos", "/ab", 4)]));
        assert_eq!(tree.starts().await?, places(&at));
        Ok(())
    }

    #[tokio::test]
    async fn requests_from_a_cookie_session_require_the_same_origin_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        assert_eq!(send(&router, Method::GET, "/api/v1/shelves", &[], None).await?.status, StatusCode::UNAUTHORIZED);
        assert_eq!(send(&router, Method::GET, "/api/v1/folder-settings", &[], None).await?.status, StatusCode::UNAUTHORIZED);
        let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "cookie")).await?;
        let cookie = cookie(&setup)?;
        let same = [("cookie", cookie.as_str()), ("origin", "http://localhost")];
        let missing = [("cookie", cookie.as_str())];
        let foreign = [("cookie", cookie.as_str()), ("origin", "http://evil.example")];

        let shelves = send(&router, Method::GET, "/api/v1/shelves", &same, None).await?;
        assert_eq!(shelves.status, StatusCode::OK);
        let id = shelves.body["shelves"][0]["id"].as_str().ok_or("id")?.to_string();
        assert_eq!(send(&router, Method::GET, "/api/v1/folder-settings", &same, None).await?.status, StatusCode::OK);

        let setting = json!({"display": {}});
        let setting_path = format!("/api/v1{}", setting_uri("photos", "/photos"));
        let operations = [
            (Method::POST, "/api/v1/shelves".to_string(), Some(shelf("new")), StatusCode::CREATED),
            (Method::PUT, format!("/api/v1/shelves/{id}"), Some(shelf("replaced")), StatusCode::OK),
            (Method::PUT, "/api/v1/shelves/order".to_string(), Some(json!({"ids": []})), StatusCode::BAD_REQUEST),
            (Method::DELETE, format!("/api/v1/shelves/{id}"), None, StatusCode::NO_CONTENT),
            (Method::PUT, setting_path.clone(), Some(setting), StatusCode::OK),
            (Method::DELETE, setting_path, None, StatusCode::NO_CONTENT),
        ];
        for (method, path, body, answer) in operations {
            for headers in [&missing[..], &foreign[..]] {
                let response = send(&router, method.clone(), &path, headers, body.clone()).await?;
                assert_eq!(response.status, StatusCode::FORBIDDEN, "{method} {path}");
            }
            let response = send(&router, method.clone(), &path, &same, body).await?;
            assert_eq!(response.status, answer, "{method} {path}: {}", response.body);
        }
        Ok(())
    }
}
