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
impl apis::rating::Rating for ApiServer {
    type Claims = Claims;

    async fn get_rating(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetRatingPathParams,
        query_params: &models::GetRatingQueryParams,
    ) -> std::result::Result<apis::rating::GetRatingResponse, ()> {
        let rating = async {
            let path = self.rating_target(&path_params.library_id, &query_params.path).await?;
            Ok::<_, Error>(self.state.rating_tags.rating(&claims.user.id, &path_params.library_id, &path).await?)
        }
        .await;
        Ok(match rating {
            Ok(rating) => apis::rating::GetRatingResponse::Status200_TheRating(models::RatingResponse::new(Self::rating_to_model(rating))),
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
        body: &models::PutRatingRequest,
    ) -> std::result::Result<apis::rating::PutRatingResponse, ()> {
        let saved = async {
            let path = self.rating_target(&path_params.library_id, &body.path).await?;
            let rating = Self::rating_from_model(&body.rating);
            self.state.rating_tags.set_rating(&claims.user.id, &path_params.library_id, &path, rating).await?;
            Ok::<_, Error>(rating)
        }
        .await;
        Ok(match saved {
            Ok(rating) => apis::rating::PutRatingResponse::Status200_TheSavedRating(models::RatingResponse::new(Self::rating_to_model(rating))),
            Err(error) => ApiError::from(error).into(),
        })
    }
}

#[allow(unused_variables)]
#[async_trait]
impl apis::tags::Tags for ApiServer {
    type Claims = Claims;

    async fn get_entry_tags(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetEntryTagsPathParams,
        query_params: &models::GetEntryTagsQueryParams,
    ) -> std::result::Result<apis::tags::GetEntryTagsResponse, ()> {
        let tags = async {
            let path = self.rating_target(&path_params.library_id, &query_params.path).await?;
            Ok::<_, Error>(self.state.rating_tags.entry_tags(&claims.user.id, &path_params.library_id, &path).await?)
        }
        .await;
        Ok(match tags {
            Ok(tags) => apis::tags::GetEntryTagsResponse::Status200_TheTags(models::EntryTagsResponse::new(Self::tags_to_model(tags))),
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
        body: &models::PutEntryTagsRequest,
    ) -> std::result::Result<apis::tags::PutEntryTagsResponse, ()> {
        let saved = async {
            let path = self.rating_target(&path_params.library_id, &body.path).await?;
            let names = body.tags.iter().map(|tag| tag.0.clone()).collect::<Vec<_>>();
            Ok::<_, Error>(self.state.rating_tags.set_entry_tags(&claims.user.id, &path_params.library_id, &path, &names).await?)
        }
        .await;
        Ok(match saved {
            Ok(tags) => apis::tags::PutEntryTagsResponse::Status200_TheSavedTags(models::EntryTagsResponse::new(Self::tags_to_model(tags))),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn list_tags(&self, method: &Method, host: &Host, cookies: &CookieJar, claims: &Self::Claims) -> std::result::Result<apis::tags::ListTagsResponse, ()> {
        Ok(match self.state.rating_tags.tags(&claims.user.id).await {
            Ok(tags) => apis::tags::ListTagsResponse::Status200_TheWords(models::TagsResponse::new(Self::tags_to_model(tags))),
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

impl From<ApiError> for apis::rating::GetRatingResponse {
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

impl From<ApiError> for apis::rating::PutRatingResponse {
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

impl From<ApiError> for apis::tags::GetEntryTagsResponse {
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

impl From<ApiError> for apis::tags::PutEntryTagsResponse {
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

impl From<ApiError> for apis::tags::ListTagsResponse {
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

    const RATING: &str = "/api/v1/libraries/photos/rating";
    const TAGS: &str = "/api/v1/libraries/photos/tags";

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
            let saved = send(&router, Method::PUT, RATING, headers, Some(json!({"path": "/p.png", "rating": value}))).await?;
            assert_eq!((saved.status, saved.body), (StatusCode::OK, json!({"rating": value})));
            let read = send(&router, Method::GET, &format!("{RATING}?path=/p.png"), headers, None).await?;
            assert_eq!(read.body, json!({"rating": value}));
        }

        let unknown = send(&router, Method::PUT, RATING, headers, Some(json!({"path": "/p.png", "rating": "great"}))).await?;
        assert!(unknown.status.is_client_error(), "{}", unknown.status);
        Ok(())
    }

    #[tokio::test]
    async fn only_an_existing_image_or_video_can_be_rated_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];

        let text = send(&router, Method::PUT, RATING, headers, Some(json!({"path": "/photos/notes.txt", "rating": "good"}))).await?;
        assert_eq!(text.status, StatusCode::BAD_REQUEST);
        let missing = send(&router, Method::PUT, RATING, headers, Some(json!({"path": "/missing.png", "rating": "good"}))).await?;
        assert_eq!(missing.status, StatusCode::NOT_FOUND);
        let folder = send(&router, Method::PUT, RATING, headers, Some(json!({"path": "/", "rating": "good"}))).await?;
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
            "/api/v1/libraries/videos/rating",
            headers,
            Some(json!({"path": "/v.mp4", "rating": "bad"})),
        )
        .await?;
        assert_eq!((rated.status, rated.body), (StatusCode::OK, json!({"rating": "bad"})));
        let tagged = send(
            &router,
            Method::PUT,
            "/api/v1/libraries/videos/tags",
            headers,
            Some(json!({"path": "/v.mp4", "tags": ["clip"]})),
        )
        .await?;
        assert_eq!((tagged.status, tagged.body), (StatusCode::OK, json!({"tags": ["clip"]})));
        assert_eq!(
            (std::fs::read(&video)?, std::fs::metadata(&video)?.modified()?),
            (b"video".to_vec(), before),
            "Entry は変わらない"
        );
        // 変更系の API は readonly の Library を拒否したままである。
        let upload = send(&router, Method::POST, "/api/v1/libraries/videos/folders", headers, Some(json!({"path": "/new"}))).await?;
        assert_eq!(upload.status, StatusCode::FORBIDDEN);

        let inner = send(&router, Method::PUT, RATING, headers, Some(json!({"path": "/photos/a.7z/dir/x.png", "rating": "good"}))).await?;
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

        let saved = send(&router, Method::PUT, TAGS, headers, Some(json!({"path": "/p.png", "tags": [" trip ", "cat", "trip"]}))).await?;
        assert_eq!((saved.status, saved.body), (StatusCode::OK, json!({"tags": ["cat", "trip"]})));
        send(&router, Method::PUT, TAGS, headers, Some(json!({"path": "/q.png", "tags": ["trip", "dog"]}))).await?;
        send(&router, Method::PUT, TAGS, headers, Some(json!({"path": "/p.png", "tags": ["cat"]}))).await?;

        let words = send(&router, Method::GET, "/api/v1/tags", headers, None).await?;
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
        send(&router, Method::PUT, TAGS, headers, Some(json!({"path": "/p.png", "tags": ["keep"]}))).await?;

        let too_many = (0..33).map(|index| format!("t{index}")).collect::<Vec<_>>();
        for tags in [json!(["  "]), json!(["a\nb"]), json!(["あ".repeat(65)]), json!(too_many)] {
            let rejected = send(&router, Method::PUT, TAGS, headers, Some(json!({"path": "/p.png", "tags": tags}))).await?;
            assert_eq!(rejected.status, StatusCode::BAD_REQUEST, "{tags}");
        }
        let read = send(&router, Method::GET, &format!("{TAGS}?path=/p.png"), headers, None).await?;
        assert_eq!(read.body, json!({"tags": ["keep"]}));
        let words = send(&router, Method::GET, "/api/v1/tags", headers, None).await?;
        assert_eq!(words.body, json!({"tags": ["keep"]}));
        Ok(())
    }

    /// 評価とタグを付けた画像を `path` に置く。
    async fn marked(fixture: &Fixture, router: &axum::Router, headers: &[(&str, &str)], path: &str) -> TestResult {
        let local = fixture._dir.path().join("library").join(path.trim_start_matches('/'));
        std::fs::create_dir_all(local.parent().ok_or("no parent")?)?;
        std::fs::write(local, b"pixel")?;
        send(router, Method::PUT, RATING, headers, Some(json!({"path": path, "rating": "good"}))).await?;
        send(router, Method::PUT, TAGS, headers, Some(json!({"path": path, "tags": ["trip"]}))).await?;
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
            "/api/v1/libraries/photos/files/move",
            headers,
            Some(json!({"srcPaths": ["/p.png"], "dstPath": "/q.png"})),
        )
        .await?;
        assert_eq!(renamed.status, StatusCode::OK);
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

        send(&router, Method::POST, "/api/v1/libraries/photos/folders", headers, Some(json!({"path": "/moved"}))).await?;
        let moved = send(
            &router,
            Method::POST,
            "/api/v1/libraries/photos/files/move",
            headers,
            Some(json!({"srcPaths": ["/q.png"], "dstPath": "/moved/q.png"})),
        )
        .await?;
        assert_eq!(moved.status, StatusCode::OK);
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
            "/api/v1/libraries/photos/folders/move",
            headers,
            Some(json!({"srcPaths": ["/album"], "dstPath": "/trip"})),
        )
        .await?;
        assert_eq!(renamed.status, StatusCode::OK);
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
            "/api/v1/libraries/photos/files/copy",
            headers,
            Some(json!({"srcPaths": ["/p.png"], "dstPath": "/copy.png"})),
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
            "/api/v1/libraries/photos/files/delete",
            headers,
            Some(json!({"srcPaths": ["/p.png"]})),
        )
        .await?;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        let marks = &fixture.server.state.rating_tags;
        assert!(!marks.all_rated_paths(&user, "photos").await?.contains("/p.png"));
        assert!(marks.tagged_paths(&user, "photos", "trip").await?.iter().all(|path| path != "/p.png"));

        let folder = send(
            &router,
            Method::POST,
            "/api/v1/libraries/photos/folders/delete",
            headers,
            Some(json!({"srcPaths": ["/dir"], "recursive": true})),
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

    const ENTRIES: &str = "/api/v1/libraries/photos/folders/entries";

    async fn names(router: &axum::Router, headers: &[(&str, &str)], query: &str) -> TestResult<(Vec<String>, serde_json::Value, String)> {
        let response = send(router, Method::GET, &format!("{ENTRIES}?{query}"), headers, None).await?;
        assert_eq!(response.status, StatusCode::OK, "{query}: {}", response.body);
        let names = response.body["entries"]
            .as_array()
            .ok_or("entries is missing")?
            .iter()
            .filter_map(|entry| entry["path"].as_str().map(str::to_string))
            .collect();
        Ok((
            names,
            response.body["totalCount"].clone(),
            response.body["snapshot"].as_str().unwrap_or_default().to_string(),
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
        put(&router, headers, RATING, json!({"path": "/good.png", "rating": "good"})).await?;
        put(&router, headers, RATING, json!({"path": "/sub/deep.png", "rating": "good"})).await?;
        put(&router, headers, RATING, json!({"path": "/bad.png", "rating": "bad"})).await?;
        put(&router, headers, TAGS, json!({"path": "/good.png", "tags": ["trip", "cat"]})).await?;
        put(&router, headers, TAGS, json!({"path": "/clip.mp4", "tags": ["trip"]})).await?;

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
        let (named, ..) = names(&router, headers, "path=/&tag=trip&query=clip").await?;
        assert_eq!(named, vec!["/clip.mp4".to_string()], "名前の条件とも組み合わせられる");
        let (none, total, _) = names(&router, headers, "path=/&tag=unused").await?;
        assert_eq!((none.len(), total), (0, json!(0)));

        let (everything, ..) = names(&router, headers, "path=/&sort=name").await?;
        assert!(
            everything.contains(&"/memo.txt".to_string()) && everything.contains(&"/sub".to_string()),
            "指定がなければ絞らない"
        );
        Ok(())
    }

    #[tokio::test]
    async fn filter_values_are_checked_and_the_snapshot_follows_writes_while_filtering_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        std::fs::write(fixture._dir.path().join("library/p.png"), b"x")?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];

        for query in ["path=/&rating=great", "path=/&tag=%20", "path=/&tag=a%0Ab"] {
            let response = send(&router, Method::GET, &format!("{ENTRIES}?{query}"), headers, None).await?;
            assert!(response.status.is_client_error(), "{query}: {}", response.status);
        }

        let (_, _, plain) = names(&router, headers, "path=/").await?;
        assert!(!plain.contains(".m"), "絞り込まない一覧の識別子には評価の版を足さない: {plain}");
        // 一覧の走査そのものが新しくなると、識別子の前半は変わる。評価とタグの版は、末尾の `.m<版>` で比べる。
        let version = |snapshot: &str| snapshot.rsplit_once(".m").map(|(_, version)| version.to_string());
        let (_, _, first) = names(&router, headers, "path=/&rating=good").await?;
        let (_, _, again) = names(&router, headers, "path=/&rating=good").await?;
        assert!(version(&first).is_some(), "絞り込み中の識別子には評価の版を足す: {first}");
        assert_eq!(version(&first), version(&again), "書き込みがなければ、評価の版は変わらない");
        put(&router, headers, RATING, json!({"path": "/p.png", "rating": "good"})).await?;
        let (listed, _, after) = names(&router, headers, "path=/&rating=good").await?;
        assert_ne!(version(&first), version(&after), "絞り込み中の識別子は、書き込みで変わる");
        assert_eq!(listed, vec!["/p.png".to_string()]);
        Ok(())
    }
}
