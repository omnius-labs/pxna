use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_engine::service::MediaKind;
use omnius_pxna_interface::{apis, models, types::Nullable};

use crate::{
    error::{ApiError, ApiStatus},
    interface::{ApiServer, session::Claims},
    prelude::*,
    service::timeline::{TimelineAnchor, TimelineCursor, TimelineListedRow, TimelineStatus},
};

const DEFAULT_LIMIT: usize = 200;

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

    async fn list_timeline_entries(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        query_params: &models::ListTimelineEntriesQueryParams,
    ) -> std::result::Result<apis::timeline::ListTimelineEntriesResponse, ()> {
        let listing = async {
            let library_ids = Self::library_ids(&query_params.library_ids);
            let anchor = Self::anchor(query_params)?;
            let limit = query_params.limit.map(|limit| limit as usize).unwrap_or(DEFAULT_LIMIT);
            let page = self.state.timeline.page(&library_ids, &anchor, limit).await?;
            let entries = page.rows.iter().map(Self::entry).collect::<Vec<_>>();
            let mut listing = models::TimelineListing::new(entries);
            listing.previous = page.previous.map(|cursor| cursor.encode());
            listing.next = page.next.map(|cursor| cursor.encode());
            Ok::<_, Error>(listing)
        }
        .await;
        Ok(match listing {
            Ok(listing) => apis::timeline::ListTimelineEntriesResponse::Status200_TimelineEntries(listing),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn list_timeline_months(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        query_params: &models::ListTimelineMonthsQueryParams,
    ) -> std::result::Result<apis::timeline::ListTimelineMonthsResponse, ()> {
        let library_ids = Self::library_ids(&query_params.library_ids);
        Ok(match self.state.timeline.months(&library_ids).await {
            Ok(months) => {
                let months = months.into_iter().map(|(month, count)| models::TimelineMonth::new(month, count)).collect();
                apis::timeline::ListTimelineMonthsResponse::Status200_EntryCountsPerMonth(models::TimelineMonths::new(months))
            }
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

    /// `after`、`before`、`month`、`around` のうち、指定できるのは 1 つだけである。`around` には `aroundLibraryId` が要る。
    fn anchor(query: &models::ListTimelineEntriesQueryParams) -> Result<TimelineAnchor> {
        let given = [query.after.is_some(), query.before.is_some(), query.month.is_some(), query.around.is_some()];
        if given.iter().filter(|given| **given).count() > 1 {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("at most one of after, before, month and around can be given"));
        }
        if query.around.is_some() != query.around_library_id.is_some() {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("around and aroundLibraryId must be given together"));
        }
        Ok(if let Some(after) = &query.after {
            TimelineAnchor::After(TimelineCursor::parse(after)?)
        } else if let Some(before) = &query.before {
            TimelineAnchor::Before(TimelineCursor::parse(before)?)
        } else if let Some(month) = &query.month {
            TimelineAnchor::Month(month.clone())
        } else if let (Some(around), Some(library)) = (&query.around, &query.around_library_id) {
            if !query.library_ids.iter().any(|id| &id.0 == library) {
                return Err(Error::new(ErrorKind::InvalidInput).with_message("aroundLibraryId must be one of libraryIds"));
            }
            TimelineAnchor::Around {
                library: library.clone(),
                path: format!("/{}", crate::service::library::LibraryRelativePath::parse(around)?.segments().join("/")),
            }
        } else {
            TimelineAnchor::Newest
        })
    }

    fn status(status: TimelineStatus) -> models::TimelineStatus {
        let mut model = models::TimelineStatus::new(status.library_id, status.running, status.processed_files);
        model.completed_at = status.completed_at;
        model
    }

    /// 索引の行を、フォルダの一覧と同じ形の Entry に Library と並びの日時を足した応答にする。
    fn entry(listed: &TimelineListedRow) -> models::TimelineEntry {
        let row = &listed.row;
        let name = row.path.rsplit('/').next().unwrap_or_default().to_string();
        let mut metadata = models::FileSystemMetadata::new(row.size, row.readonly);
        metadata.modified_at = row.modified.map(Nullable::Present);
        metadata.content_type = MediaKind::content_type(&name).map(|content_type| Nullable::Present(content_type.to_string()));
        let mut entry = models::FileSystemEntry::new(row.path.clone(), name, metadata, models::FileSystemEntryKind::File);
        entry.file = Some(Nullable::Present(models::FileSystemEntryFileDetail::new(false)));
        models::TimelineEntry::new(listed.library.clone(), entry, row.date.text(), row.date.source.as_str().to_string())
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

impl From<ApiError> for apis::timeline::ListTimelineEntriesResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::timeline::ListTimelineMonthsResponse {
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

        let entries = send(&router, Method::GET, &format!("/api/v1/timeline/entries?{both}"), headers, None).await?;
        assert_eq!(entries.status, StatusCode::OK);
        let listed = entries.body["entries"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|entry| format!("{}{}", entry["libraryId"].as_str().unwrap_or_default(), entry["entry"]["path"].as_str().unwrap_or_default()))
            .collect::<Vec<_>>();
        assert!(listed.contains(&"photos/photos/p.png".to_string()), "{listed:?}");
        assert!(listed.contains(&"videos/v.png".to_string()), "{listed:?}");
        let only = send(&router, Method::GET, "/api/v1/timeline/entries?libraryIds=videos", headers, None).await?;
        assert_eq!(only.body["entries"].as_array().map(Vec::len), Some(1));

        let months = send(&router, Method::GET, &format!("/api/v1/timeline/months?{both}"), headers, None).await?;
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
            Method::GET,
            &format!("/api/v1/timeline/entries?{both}&around=v.png&aroundLibraryId=videos&limit=1"),
            headers,
            None,
        )
        .await?;
        assert_eq!(around.body["entries"][0]["libraryId"], "videos");

        for (uri, expected) in [
            ("/api/v1/timeline/entries?libraryIds=photos&libraryIds=nope", StatusCode::NOT_FOUND),
            ("/api/v1/timeline/months?libraryIds=nope", StatusCode::NOT_FOUND),
            ("/api/v1/timeline/status?libraryIds=nope", StatusCode::NOT_FOUND),
            ("/api/v1/timeline/entries", StatusCode::BAD_REQUEST),
            ("/api/v1/timeline/entries?libraryIds=photos&around=v.png", StatusCode::BAD_REQUEST),
            ("/api/v1/timeline/entries?libraryIds=photos&around=v.png&aroundLibraryId=videos", StatusCode::BAD_REQUEST),
            ("/api/v1/libraries/photos/timeline/entries", StatusCode::NOT_FOUND),
        ] {
            assert_eq!(send(&router, Method::GET, uri, headers, None).await?.status, expected, "{uri}");
        }
        let rejected = send(&router, Method::POST, "/api/v1/timeline/refresh", headers, Some(json!({"libraryIds": ["photos", "nope"]}))).await?;
        assert_eq!(rejected.status, StatusCode::NOT_FOUND);
        Ok(())
    }
}
