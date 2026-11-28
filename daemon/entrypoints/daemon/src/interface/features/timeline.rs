use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_interface::{apis, models};

use crate::{
    error::{ApiError, ApiStatus},
    interface::{ApiServer, session::Claims},
    prelude::*,
    service::timeline::TimelineStatus,
};

#[allow(unused_variables)]
#[async_trait]
impl apis::timeline::Timeline for ApiServer {
    type Claims = Claims;

    async fn get_timeline_status(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        query_params: &models::GetTimelineStatusQueryParams,
    ) -> std::result::Result<apis::timeline::GetTimelineStatusResponse, ()> {
        let statuses = async {
            let library_ids = Self::library_ids(&query_params.library_ids);
            self.state.timeline.libraries(&library_ids)?;
            let mut statuses = Vec::new();
            for library_id in &library_ids {
                statuses.push(Self::status(self.state.timeline.status(library_id).await?));
            }
            Ok::<_, Error>(models::TimelineStatuses::new(statuses))
        }
        .await;
        Ok(match statuses {
            Ok(statuses) => apis::timeline::GetTimelineStatusResponse::Status200_StateOfTheScans(statuses),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn refresh_timeline(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        body: &models::TimelineRefreshRequest,
    ) -> std::result::Result<apis::timeline::RefreshTimelineResponse, ()> {
        let statuses = async {
            let library_ids = Self::library_ids(&body.library_ids);
            // 1 つでも知らない Library があれば、どの集計も始めない。
            self.state.timeline.libraries(&library_ids)?;
            let mut statuses = Vec::new();
            for library_id in &library_ids {
                statuses.push(Self::status(self.state.timeline.refresh(library_id).await?));
            }
            Ok::<_, Error>(models::TimelineStatuses::new(statuses))
        }
        .await;
        Ok(match statuses {
            Ok(statuses) => apis::timeline::RefreshTimelineResponse::Status202_TheScansAreRunning(statuses),
            Err(error) => ApiError::from(error).into(),
        })
    }
}

impl ApiServer {
    fn library_ids(ids: &[models::LibraryId]) -> Vec<String> {
        ids.iter().map(|id| id.0.clone()).collect()
    }

    fn status(status: TimelineStatus) -> models::TimelineStatus {
        let mut model = models::TimelineStatus::new(status.library_id, status.running, status.processed_files);
        model.completed_at = status.completed_at;
        model
    }
}

impl From<ApiError> for apis::timeline::GetTimelineStatusResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::timeline::RefreshTimelineResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
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

    #[tokio::test]
    async fn timeline_spans_the_given_libraries_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let state = &fixture.server.state;
        std::fs::write(state.conf.libraries[0].path.join("photos/p.png"), b"p")?;
        std::fs::write(state.conf.libraries[1].path.join("v.png"), b"v")?;
        let router = router(&fixture);
        let setup = send(&router, Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "bearer")).await?;
        let bearer = bearer(&setup)?;
        let headers = &[("authorization", bearer.as_str())];

        let refreshed = send(
            &router,
            Method::POST,
            "/api/v1/timeline/refresh",
            headers,
            Some(json!({"libraryIds": ["photos", "videos"]})),
        )
        .await?;
        assert_eq!(refreshed.status, StatusCode::ACCEPTED);
        assert_eq!(refreshed.body["libraries"][1]["libraryId"], "videos");
        let both = "libraryIds=photos&libraryIds=videos";
        let mut done = false;
        for _ in 0..1200 {
            let status = send(&router, Method::GET, &format!("/api/v1/timeline/status?{both}"), headers, None).await?;
            assert_eq!(status.status, StatusCode::OK);
            let libraries = status.body["libraries"].as_array().cloned().unwrap_or_default();
            if libraries.len() == 2 && libraries.iter().all(|library| library["running"] == false && library["completedAt"].is_string()) {
                done = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        let last = send(&router, Method::GET, &format!("/api/v1/timeline/status?{both}"), headers, None).await?;
        assert!(done, "the scans did not finish: {}", last.body);

        // The timeline is read through `listEntries`: the roots of the Libraries, with the insides of archives, in timeline-date order.
        let timeline = |libraries: &[&str], page: serde_json::Value| {
            let starts = libraries.iter().map(|id| json!({"libraryId": id, "path": "/"})).collect::<Vec<_>>();
            json!({
                "scope": {"starts": starts, "recursive": true, "insideArchives": true},
                "filter": {"kinds": ["image", "video"]},
                "sort": "timeline_date",
                "page": page,
            })
        };
        let entries = send(
            &router,
            Method::POST,
            "/api/v1/entries",
            headers,
            Some(timeline(&["photos", "videos"], json!({"kind": "cursor"}))),
        )
        .await?;
        assert_eq!(entries.status, StatusCode::OK, "{}", entries.body);
        let listed = entries.body["items"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|entry| format!("{}{}", entry["libraryId"].as_str().unwrap_or_default(), entry["entry"]["path"].as_str().unwrap_or_default()))
            .collect::<Vec<_>>();
        assert!(listed.contains(&"photos/photos/p.png".to_string()), "{listed:?}");
        assert!(listed.contains(&"videos/v.png".to_string()), "{listed:?}");
        let only = send(&router, Method::POST, "/api/v1/entries", headers, Some(timeline(&["videos"], json!({"kind": "cursor"})))).await?;
        assert_eq!(only.body["items"].as_array().map(Vec::len), Some(1));

        let mut months_body = timeline(&["photos", "videos"], json!(null));
        months_body.as_object_mut().ok_or("object")?.retain(|key, _| key == "scope" || key == "filter");
        let months = send(&router, Method::POST, "/api/v1/entries/months", headers, Some(months_body)).await?;
        let total = months.body["months"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|month| month["count"].as_u64().unwrap_or(0))
            .sum::<u64>();
        assert_eq!(total, listed.len() as u64);

        let around = send(
            &router,
            Method::POST,
            "/api/v1/entries",
            headers,
            Some(timeline(
                &["photos", "videos"],
                json!({"kind": "cursor", "limit": 1, "around": {"libraryId": "videos", "path": "/v.png"}}),
            )),
        )
        .await?;
        assert_eq!(around.body["items"][0]["libraryId"], "videos");

        assert_eq!(
            send(&router, Method::GET, "/api/v1/timeline/status?libraryIds=nope", headers, None).await?.status,
            StatusCode::NOT_FOUND
        );
        for removed in ["/api/v1/timeline/entries?libraryIds=photos", "/api/v1/timeline/months?libraryIds=photos"] {
            assert_eq!(send(&router, Method::GET, removed, headers, None).await?.status, StatusCode::NOT_FOUND, "{removed}");
        }
        let rejected = send(&router, Method::POST, "/api/v1/timeline/refresh", headers, Some(json!({"libraryIds": ["photos", "nope"]}))).await?;
        assert_eq!(rejected.status, StatusCode::NOT_FOUND);
        Ok(())
    }
}
