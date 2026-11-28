use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_engine::{model::Rating, service::MediaKind};
use omnius_pxna_interface::{apis, models};

use crate::{
    error::{ApiError, ApiStatus},
    interface::{ApiServer, session::Claims},
    prelude::*,
};

#[allow(unused_variables)]
#[async_trait]
impl apis::ratings_tags::RatingsTags for ApiServer {
    type Claims = Claims;

    async fn get_rating(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetRatingPathParams,
        query_params: &models::GetRatingQueryParams,
    ) -> std::result::Result<apis::ratings_tags::GetRatingResponse, ()> {
        let rating = async {
            let path = self.rating_target(&path_params.library_id, &query_params.path).await?;
            Ok::<_, Error>(self.state.rating_tags.rating(&claims.user.id, &path_params.library_id, &path).await?)
        }
        .await;
        Ok(match rating {
            Ok(rating) => apis::ratings_tags::GetRatingResponse::Status200_TheRating(models::RatingResponse::new(Self::rating_to_model(rating))),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn put_rating(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::PutRatingPathParams,
        query_params: &models::PutRatingQueryParams,
        body: &models::PutRatingRequest,
    ) -> std::result::Result<apis::ratings_tags::PutRatingResponse, ()> {
        let saved = async {
            let path = self.rating_target(&path_params.library_id, &query_params.path).await?;
            let rating = Self::rating_from_model(&body.rating);
            self.state.rating_tags.set_rating(&claims.user.id, &path_params.library_id, &path, rating).await?;
            Ok::<_, Error>(rating)
        }
        .await;
        Ok(match saved {
            Ok(rating) => apis::ratings_tags::PutRatingResponse::Status200_TheSavedRating(models::RatingResponse::new(Self::rating_to_model(rating))),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn get_entry_tags(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetEntryTagsPathParams,
        query_params: &models::GetEntryTagsQueryParams,
    ) -> std::result::Result<apis::ratings_tags::GetEntryTagsResponse, ()> {
        let tags = async {
            let path = self.rating_target(&path_params.library_id, &query_params.path).await?;
            Ok::<_, Error>(self.state.rating_tags.entry_tags(&claims.user.id, &path_params.library_id, &path).await?)
        }
        .await;
        Ok(match tags {
            Ok(tags) => apis::ratings_tags::GetEntryTagsResponse::Status200_TheTags(models::EntryTagsResponse::new(Self::tags_to_model(tags))),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn put_entry_tags(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::PutEntryTagsPathParams,
        query_params: &models::PutEntryTagsQueryParams,
        body: &models::PutEntryTagsRequest,
    ) -> std::result::Result<apis::ratings_tags::PutEntryTagsResponse, ()> {
        let saved = async {
            let path = self.rating_target(&path_params.library_id, &query_params.path).await?;
            let names = body.tags.iter().map(|tag| tag.0.clone()).collect::<Vec<_>>();
            Ok::<_, Error>(self.state.rating_tags.set_entry_tags(&claims.user.id, &path_params.library_id, &path, &names).await?)
        }
        .await;
        Ok(match saved {
            Ok(tags) => apis::ratings_tags::PutEntryTagsResponse::Status200_TheSavedTags(models::EntryTagsResponse::new(Self::tags_to_model(tags))),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn list_tag_words(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> std::result::Result<apis::ratings_tags::ListTagWordsResponse, ()> {
        Ok(match self.state.rating_tags.tags(&claims.user.id).await {
            Ok(tags) => apis::ratings_tags::ListTagWordsResponse::Status200_TheWords(models::TagsResponse::new(Self::tags_to_model(tags))),
            Err(error) => ApiError::from(Error::from(error)).into(),
        })
    }
}

impl ApiServer {
    /// 評価とタグの対象になる Entry を確かめ、保存の key にする Library 相対パスを返す。
    ///
    /// 存在する file で、サムネイルと同じ表で画像か動画と判定できるものに限る。`readonly` の Library と archive の中の path も対象にできる。
    async fn rating_target(&self, library_id: &str, path: &str) -> Result<String> {
        let entry = self.state.library.get_file(library_id, path).await?;
        if MediaKind::of_name(&entry.name).is_none() {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("only an image or a video can be rated or tagged: {}", entry.name)));
        }
        Ok(entry.path)
    }

    fn rating_to_model(rating: Option<Rating>) -> models::RatingValue {
        match rating {
            Some(Rating::Good) => models::RatingValue::Good,
            Some(Rating::Bad) => models::RatingValue::Bad,
            None => models::RatingValue::None,
        }
    }

    fn rating_from_model(value: &models::RatingValue) -> Option<Rating> {
        match value {
            models::RatingValue::Good => Some(Rating::Good),
            models::RatingValue::Bad => Some(Rating::Bad),
            models::RatingValue::None => None,
        }
    }

    fn tags_to_model(tags: Vec<String>) -> Vec<models::TagName> {
        tags.into_iter().map(models::TagName).collect()
    }
}

impl From<ApiError> for apis::ratings_tags::GetRatingResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::ratings_tags::PutRatingResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::ratings_tags::GetEntryTagsResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::ratings_tags::PutEntryTagsResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::ratings_tags::ListTagWordsResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            _ => Self::Status500_InternalServerError(error.body),
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

    const RATING: &str = "/api/v1/libraries/photos/entries/rating";
    const TAGS: &str = "/api/v1/libraries/photos/entries/tags";

    async fn login(router: &axum::Router) -> TestResult<String> {
        let setup = send(router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "bearer")).await?;
        bearer(&setup)
    }

    #[tokio::test]
    async fn rating_is_saved_replaced_and_cleared_for_an_image_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        std::fs::write(fixture._dir.path().join("library/p.png"), b"pixel")?;
        let router = router(&fixture);
        assert_eq!(
            send(&router, Method::GET, &format!("{RATING}?path=/p.png"), &[], None).await?.status,
            StatusCode::UNAUTHORIZED
        );
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];

        let first = send(&router, Method::GET, &format!("{RATING}?path=/p.png"), headers, None).await?;
        assert_eq!((first.status, first.body), (StatusCode::OK, json!({"rating": "none"})));
        for value in ["good", "bad", "none"] {
            let saved = send(&router, Method::PUT, &format!("{RATING}?path=/p.png"), headers, Some(json!({"rating": value}))).await?;
            assert_eq!((saved.status, saved.body), (StatusCode::OK, json!({"rating": value})));
            let read = send(&router, Method::GET, &format!("{RATING}?path=/p.png"), headers, None).await?;
            assert_eq!(read.body, json!({"rating": value}));
        }

        let unknown = send(&router, Method::PUT, &format!("{RATING}?path=/p.png"), headers, Some(json!({"rating": "great"}))).await?;
        assert!(unknown.status.is_client_error(), "{}", unknown.status);
        Ok(())
    }

    #[tokio::test]
    async fn only_an_existing_image_or_video_can_be_rated_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];

        let text = send(&router, Method::PUT, &format!("{RATING}?path=/photos/notes.txt"), headers, Some(json!({"rating": "good"}))).await?;
        assert_eq!(text.status, StatusCode::BAD_REQUEST);
        let missing = send(&router, Method::PUT, &format!("{RATING}?path=/missing.png"), headers, Some(json!({"rating": "good"}))).await?;
        assert_eq!(missing.status, StatusCode::NOT_FOUND);
        let folder = send(&router, Method::PUT, &format!("{RATING}?path=/"), headers, Some(json!({"rating": "good"}))).await?;
        assert_eq!(folder.status, StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[tokio::test]
    async fn a_readonly_library_and_an_archive_entry_accept_ratings_and_tags_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let video = fixture._dir.path().join("videos/v.mp4");
        std::fs::write(&video, b"video")?;
        let before = std::fs::metadata(&video)?.modified()?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];

        let rated = send(
            &router,
            Method::PUT,
            "/api/v1/libraries/videos/entries/rating?path=/v.mp4",
            headers,
            Some(json!({"rating": "bad"})),
        )
        .await?;
        assert_eq!((rated.status, rated.body), (StatusCode::OK, json!({"rating": "bad"})));
        let tagged = send(
            &router,
            Method::PUT,
            "/api/v1/libraries/videos/entries/tags?path=/v.mp4",
            headers,
            Some(json!({"tags": ["clip"]})),
        )
        .await?;
        assert_eq!((tagged.status, tagged.body), (StatusCode::OK, json!({"tags": ["clip"]})));
        assert_eq!(
            (std::fs::read(&video)?, std::fs::metadata(&video)?.modified()?),
            (b"video".to_vec(), before),
            "Entry は変わらない"
        );
        // 変更系の API は readonly の Library を拒否したままである。
        let upload = send(&router, Method::POST, "/api/v1/libraries/videos/entries/folders", headers, Some(json!({"path": "/new"}))).await?;
        assert_eq!(upload.status, StatusCode::FORBIDDEN);

        let inner = send(
            &router,
            Method::PUT,
            &format!("{RATING}?path=/photos/a.7z/dir/x.png"),
            headers,
            Some(json!({"rating": "good"})),
        )
        .await?;
        assert_eq!((inner.status, inner.body), (StatusCode::OK, json!({"rating": "good"})));
        let read = send(&router, Method::GET, &format!("{RATING}?path=/photos/a.7z/dir/x.png"), headers, None).await?;
        assert_eq!(read.body, json!({"rating": "good"}));
        Ok(())
    }

    #[tokio::test]
    async fn tags_are_replaced_completed_and_visible_from_another_session_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        std::fs::write(fixture._dir.path().join("library/p.png"), b"pixel")?;
        std::fs::write(fixture._dir.path().join("library/q.png"), b"pixel")?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];

        let saved = send(
            &router,
            Method::PUT,
            &format!("{TAGS}?path=/p.png"),
            headers,
            Some(json!({"tags": [" trip ", "cat", "trip"]})),
        )
        .await?;
        assert_eq!((saved.status, saved.body), (StatusCode::OK, json!({"tags": ["cat", "trip"]})));
        send(&router, Method::PUT, &format!("{TAGS}?path=/q.png"), headers, Some(json!({"tags": ["trip", "dog"]}))).await?;
        send(&router, Method::PUT, &format!("{TAGS}?path=/p.png"), headers, Some(json!({"tags": ["cat"]}))).await?;

        let words = send(&router, Method::GET, "/api/v1/tags/words", headers, None).await?;
        assert_eq!(words.body, json!({"tags": ["cat", "dog", "trip"]}));
        let tags = send(&router, Method::GET, &format!("{TAGS}?path=/p.png"), headers, None).await?;
        assert_eq!(tags.body, json!({"tags": ["cat"]}));

        let second = send(&router, Method::POST, "/api/v1/auth/login", &[], credentials("owner", "password1", "bearer")).await?;
        let other = bearer(&second)?;
        assert_ne!(other, token);
        let seen = send(&router, Method::GET, &format!("{TAGS}?path=/q.png"), &[("authorization", other.as_str())], None).await?;
        assert_eq!(seen.body, json!({"tags": ["dog", "trip"]}), "別のセッションから同じ値が見える");
        Ok(())
    }

    #[tokio::test]
    async fn invalid_tags_are_rejected_and_change_nothing_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        std::fs::write(fixture._dir.path().join("library/p.png"), b"pixel")?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];
        send(&router, Method::PUT, &format!("{TAGS}?path=/p.png"), headers, Some(json!({"tags": ["keep"]}))).await?;

        let too_many = (0..33).map(|index| format!("t{index}")).collect::<Vec<_>>();
        for tags in [json!(["  "]), json!(["a\nb"]), json!(["あ".repeat(65)]), json!(too_many)] {
            let rejected = send(&router, Method::PUT, &format!("{TAGS}?path=/p.png"), headers, Some(json!({"tags": tags}))).await?;
            assert_eq!(rejected.status, StatusCode::BAD_REQUEST, "{tags}");
        }
        let read = send(&router, Method::GET, &format!("{TAGS}?path=/p.png"), headers, None).await?;
        assert_eq!(read.body, json!({"tags": ["keep"]}));
        let words = send(&router, Method::GET, "/api/v1/tags/words", headers, None).await?;
        assert_eq!(words.body, json!({"tags": ["keep"]}));
        Ok(())
    }

    /// 評価とタグを付けた画像を `path` に置く。
    async fn marked(fixture: &Fixture, router: &axum::Router, headers: &[(&str, &str)], path: &str) -> TestResult {
        let local = fixture._dir.path().join("library").join(path.trim_start_matches('/'));
        std::fs::create_dir_all(local.parent().ok_or("no parent")?)?;
        std::fs::write(local, b"pixel")?;
        send(router, Method::PUT, &format!("{RATING}?path={path}"), headers, Some(json!({"rating": "good"}))).await?;
        send(router, Method::PUT, &format!("{TAGS}?path={path}"), headers, Some(json!({"tags": ["trip"]}))).await?;
        Ok(())
    }

    async fn rating_of(router: &axum::Router, headers: &[(&str, &str)], path: &str) -> TestResult<(StatusCode, serde_json::Value)> {
        let response = send(router, Method::GET, &format!("{RATING}?path={path}"), headers, None).await?;
        Ok((response.status, response.body))
    }

    #[tokio::test]
    async fn renaming_and_moving_a_file_carries_its_rating_and_tags_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];
        marked(&fixture, &router, headers, "/p.png").await?;

        let renamed = send(
            &router,
            Method::POST,
            "/api/v1/entries/move",
            headers,
            Some(json!({"sources": [{"libraryId": "photos", "path": "/p.png", "view": "file"}], "destination": {"libraryId": "photos", "path": "/q.png"}})),
        )
        .await?;
        assert_eq!(renamed.status, StatusCode::ACCEPTED);
        assert_eq!(renamed.body["operation"]["status"], "succeeded");
        assert_eq!(rating_of(&router, headers, "/q.png").await?.1, json!({"rating": "good"}));
        let tags = send(&router, Method::GET, &format!("{TAGS}?path=/q.png"), headers, None).await?;
        assert_eq!(tags.body, json!({"tags": ["trip"]}));
        assert_eq!(rating_of(&router, headers, "/p.png").await?.0, StatusCode::NOT_FOUND);
        assert!(
            fixture
                .server
                .state
                .rating_tags
                .rated_paths(&user_id(&router, headers).await?, "photos", omnius_pxna_engine::model::Rating::Good)
                .await?
                .contains("/q.png")
        );

        send(&router, Method::POST, "/api/v1/libraries/photos/entries/folders", headers, Some(json!({"path": "/moved"}))).await?;
        let moved = send(
            &router,
            Method::POST,
            "/api/v1/entries/move",
            headers,
            Some(json!({"sources": [{"libraryId": "photos", "path": "/q.png", "view": "file"}], "destination": {"libraryId": "photos", "path": "/moved/q.png"}})),
        )
        .await?;
        assert_eq!(moved.status, StatusCode::ACCEPTED);
        assert_eq!(moved.body["operation"]["status"], "succeeded");
        assert_eq!(rating_of(&router, headers, "/moved/q.png").await?.1, json!({"rating": "good"}));
        Ok(())
    }

    #[tokio::test]
    async fn renaming_a_folder_carries_the_marks_of_what_is_inside_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];
        marked(&fixture, &router, headers, "/album/p.png").await?;
        marked(&fixture, &router, headers, "/album2/p.png").await?;

        let renamed = send(
            &router,
            Method::POST,
            "/api/v1/entries/move",
            headers,
            Some(json!({"sources": [{"libraryId": "photos", "path": "/album", "view": "folder"}], "destination": {"libraryId": "photos", "path": "/trip"}})),
        )
        .await?;
        assert_eq!(renamed.status, StatusCode::ACCEPTED);
        assert_eq!(renamed.body["operation"]["status"], "succeeded");
        assert_eq!(rating_of(&router, headers, "/trip/p.png").await?.1, json!({"rating": "good"}));
        assert_eq!(
            rating_of(&router, headers, "/album2/p.png").await?.1,
            json!({"rating": "good"}),
            "名前が前方一致するだけのフォルダは動かない"
        );
        Ok(())
    }

    #[tokio::test]
    async fn deleting_a_file_or_folder_forgets_its_marks_but_a_copy_does_not_inherit_them_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];
        marked(&fixture, &router, headers, "/p.png").await?;
        marked(&fixture, &router, headers, "/dir/q.png").await?;
        let user = user_id(&router, headers).await?;

        let copied = send(
            &router,
            Method::POST,
            "/api/v1/entries/copy",
            headers,
            Some(json!({"sources": [{"libraryId": "photos", "path": "/p.png", "view": "file"}], "destination": {"libraryId": "photos", "path": "/copy.png"}})),
        )
        .await?;
        assert_eq!(copied.status, StatusCode::ACCEPTED);
        let operation = copied.body["operation"]["id"].as_str().ok_or("operation id is missing")?;
        assert_eq!(
            wait_operation(&fixture.server.state, operation).await?.status,
            omnius_pxna_interface::models::OperationStatus::Succeeded
        );
        assert_eq!(
            rating_of(&router, headers, "/copy.png").await?.1,
            json!({"rating": "none"}),
            "コピーは別の Entry で、評価を引き継がない"
        );
        assert_eq!(rating_of(&router, headers, "/p.png").await?.1, json!({"rating": "good"}));

        let deleted = send(
            &router,
            Method::POST,
            "/api/v1/entries/delete",
            headers,
            Some(json!({"targets": [{"libraryId": "photos", "path": "/p.png", "view": "file"}]})),
        )
        .await?;
        assert_eq!(deleted.status, StatusCode::ACCEPTED);
        assert_eq!(deleted.body["operation"]["status"], "succeeded");
        let marks = &fixture.server.state.rating_tags;
        assert!(!marks.all_rated_paths(&user, "photos").await?.contains("/p.png"));
        assert!(marks.tagged_paths(&user, "photos", "trip").await?.iter().all(|path| path != "/p.png"));

        let folder = send(
            &router,
            Method::POST,
            "/api/v1/entries/delete",
            headers,
            Some(json!({"targets": [{"libraryId": "photos", "path": "/dir", "view": "folder"}], "recursive": true})),
        )
        .await?;
        assert!(folder.status.is_success(), "{}", folder.status);
        gone(marks, &user, "/dir/q.png").await?;
        assert!(marks.tags(&user).await?.contains(&"trip".to_string()), "語彙は残る");
        Ok(())
    }

    /// ログイン中のユーザの識別子。保存した行を `RatingTagService` から直接確かめるのに使う。
    async fn user_id(router: &axum::Router, headers: &[(&str, &str)]) -> TestResult<String> {
        let me = send(router, Method::GET, "/api/v1/auth/me", headers, None).await?;
        Ok(me.body["user"]["id"].as_str().ok_or("user id is missing")?.to_string())
    }

    /// 非同期の操作が終わって行が消えるまで待つ。
    async fn gone(marks: &omnius_pxna_engine::service::RatingTagService, user: &str, path: &str) -> TestResult {
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while marks.all_rated_paths(user, "photos").await?.contains(path) {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            TestResult::Ok(())
        })
        .await??;
        Ok(())
    }

    /// A listing of `photos` through `listEntries`, from the compact query `path=/&rating=good&tag=trip&query=clip&recursive=true&sort=name`.
    async fn names(router: &axum::Router, headers: &[(&str, &str)], query: &str) -> TestResult<(Vec<String>, serde_json::Value, String)> {
        let (mut path, mut recursive, mut sort) = ("/".to_string(), false, None);
        let mut filter = serde_json::Map::new();
        let mut tags = Vec::new();
        for (key, value) in query.split('&').filter_map(|pair| pair.split_once('=')) {
            match key {
                "path" => path = value.to_string(),
                "recursive" => recursive = value == "true",
                "sort" => sort = Some(value.to_string()),
                "tag" => tags.push(value.to_string()),
                _ => {
                    filter.insert(key.to_string(), json!(value));
                }
            }
        }
        if !tags.is_empty() {
            filter.insert("tags".to_string(), json!(tags));
        }
        let mut body = json!({"scope": {"starts": [{"libraryId": "photos", "path": path}], "recursive": recursive}, "filter": filter, "page": {"kind": "offset"}});
        if let Some(sort) = sort {
            body["sort"] = json!(sort);
        }
        let response = send(router, Method::POST, "/api/v1/entries", headers, Some(body)).await?;
        assert_eq!(response.status, StatusCode::OK, "{query}: {}", response.body);
        let names = response.body["items"]
            .as_array()
            .ok_or("items is missing")?
            .iter()
            .filter_map(|entry| entry["entry"]["path"].as_str().map(str::to_string))
            .collect();
        Ok((
            names,
            response.body["page"]["totalCount"].clone(),
            response.body["page"]["snapshot"].as_str().unwrap_or_default().to_string(),
        ))
    }

    async fn put(router: &axum::Router, headers: &[(&str, &str)], uri: &str, body: serde_json::Value) -> TestResult {
        assert_eq!(send(router, Method::PUT, uri, headers, Some(body)).await?.status, StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn the_listing_keeps_only_entries_with_the_rating_or_the_tag_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let library = fixture._dir.path().join("library");
        for name in ["good.png", "bad.png", "plain.png", "clip.mp4", "memo.txt"] {
            std::fs::write(library.join(name), b"x")?;
        }
        std::fs::create_dir_all(library.join("sub"))?;
        std::fs::write(library.join("sub/deep.png"), b"x")?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];
        put(&router, headers, &format!("{RATING}?path=/good.png"), json!({"rating": "good"})).await?;
        put(&router, headers, &format!("{RATING}?path=/sub/deep.png"), json!({"rating": "good"})).await?;
        put(&router, headers, &format!("{RATING}?path=/bad.png"), json!({"rating": "bad"})).await?;
        put(&router, headers, &format!("{TAGS}?path=/good.png"), json!({"tags": ["trip", "cat"]})).await?;
        put(&router, headers, &format!("{TAGS}?path=/clip.mp4"), json!({"tags": ["trip"]})).await?;

        let (good, total, _) = names(&router, headers, "path=/&rating=good").await?;
        assert_eq!((good, total), (vec!["/good.png".to_string()], json!(1)), "直下だけを見る");
        let (deep, ..) = names(&router, headers, "path=/&rating=good&recursive=true&sort=name").await?;
        assert_eq!(
            deep,
            vec!["/sub/deep.png".to_string(), "/good.png".to_string()],
            "再帰の結果は名前（deep.png、good.png）の順"
        );
        let (bad, ..) = names(&router, headers, "path=/&rating=bad").await?;
        assert_eq!(bad, vec!["/bad.png".to_string()]);

        let (unrated, ..) = names(&router, headers, "path=/&rating=none&sort=name").await?;
        assert_eq!(
            unrated,
            vec!["/clip.mp4".to_string(), "/plain.png".to_string()],
            "未評価は画像と動画だけで、フォルダと memo.txt は含まない"
        );

        let (tagged, ..) = names(&router, headers, "path=/&tag=trip&sort=name").await?;
        assert_eq!(tagged, vec!["/clip.mp4".to_string(), "/good.png".to_string()]);
        let (both, ..) = names(&router, headers, "path=/&tag=trip&rating=good").await?;
        assert_eq!(both, vec!["/good.png".to_string()]);
        let (all_tags, ..) = names(&router, headers, "path=/&tag=trip&tag=cat&sort=name").await?;
        assert_eq!(all_tags, vec!["/good.png".to_string()], "複数のタグは、すべてを持つ Entry だけを選ぶ");
        let (named, ..) = names(&router, headers, "path=/&tag=trip&query=clip").await?;
        assert_eq!(named, vec!["/clip.mp4".to_string()], "名前の条件とも組み合わせられる");
        let (none, total, _) = names(&router, headers, "path=/&tag=unused").await?;
        assert_eq!((none.len(), total), (0, json!(0)));

        let (everything, ..) = names(&router, headers, "path=/&sort=name").await?;
        assert!(
            everything.contains(&"/memo.txt".to_string()) && everything.contains(&"/sub".to_string()),
            "指定がなければ絞らない"
        );

        // The summary of direct children takes no rating or tag condition, only a name condition.
        let summary = |filter: serde_json::Value| json!({"scope": {"starts": [{"libraryId": "photos", "path": "/"}]}, "filter": filter, "options": {"fields": "summary"}, "page": {"kind": "offset"}});
        let marked = send(
            &router,
            Method::POST,
            "/api/v1/entries",
            headers,
            Some(summary(json!({"rating": "good", "tags": ["trip"]}))),
        )
        .await?;
        assert_eq!(marked.status, StatusCode::BAD_REQUEST, "{}", marked.body);
        let named = send(&router, Method::POST, "/api/v1/entries", headers, Some(summary(json!({"query": "memo"})))).await?;
        assert_eq!(named.body["page"]["totalCount"], json!(1), "{}", named.body);
        Ok(())
    }

    #[tokio::test]
    async fn filter_values_are_checked_and_the_snapshot_follows_writes_while_filtering_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        std::fs::write(fixture._dir.path().join("library/p.png"), b"x")?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];

        for filter in [json!({"rating": "great"}), json!({"tags": [" "]}), json!({"tags": ["a\nb"]})] {
            let body = json!({"scope": {"starts": [{"libraryId": "photos", "path": "/"}]}, "filter": filter});
            let response = send(&router, Method::POST, "/api/v1/entries", headers, Some(body)).await?;
            assert!(response.status.is_client_error(), "{filter}: {}", response.status);
        }

        let (_, _, plain) = names(&router, headers, "path=/").await?;
        assert!(!plain.contains(".m"), "絞り込まない一覧の識別子には評価の版を足さない: {plain}");
        // 一覧の走査そのものが新しくなると、識別子の前半は変わる。評価とタグの版は、末尾の `.m<版>` で比べる。
        let version = |snapshot: &str| snapshot.rsplit_once(".m").map(|(_, version)| version.to_string());
        let (_, _, first) = names(&router, headers, "path=/&rating=good").await?;
        let (_, _, again) = names(&router, headers, "path=/&rating=good").await?;
        assert!(version(&first).is_some(), "絞り込み中の識別子には評価の版を足す: {first}");
        assert_eq!(version(&first), version(&again), "書き込みがなければ、評価の版は変わらない");
        put(&router, headers, &format!("{RATING}?path=/p.png"), json!({"rating": "good"})).await?;
        let (listed, _, after) = names(&router, headers, "path=/&rating=good").await?;
        assert_ne!(version(&first), version(&after), "絞り込み中の識別子は、書き込みで変わる");
        assert_eq!(listed, vec!["/p.png".to_string()]);
        Ok(())
    }
}
