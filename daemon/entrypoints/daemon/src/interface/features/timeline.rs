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
    service::timeline::{TimelineAnchor, TimelineCursor, TimelineRow, TimelineStatus},
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
        path_params: &models::GetTimelineStatusPathParams,
    ) -> std::result::Result<apis::timeline::GetTimelineStatusResponse, ()> {
        Ok(match self.state.timeline.status(&path_params.library_id).await {
            Ok(status) => apis::timeline::GetTimelineStatusResponse::Status200_StateOfTheScan(Self::status(status)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn list_timeline_entries(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::ListTimelineEntriesPathParams,
        query_params: &models::ListTimelineEntriesQueryParams,
    ) -> std::result::Result<apis::timeline::ListTimelineEntriesResponse, ()> {
        let listing = async {
            let anchor = Self::anchor(query_params)?;
            let limit = query_params.limit.map(|limit| limit as usize).unwrap_or(DEFAULT_LIMIT);
            let page = self.state.timeline.page(&path_params.library_id, &anchor, limit).await?;
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
        path_params: &models::ListTimelineMonthsPathParams,
    ) -> std::result::Result<apis::timeline::ListTimelineMonthsResponse, ()> {
        Ok(match self.state.timeline.months(&path_params.library_id).await {
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
        path_params: &models::RefreshTimelinePathParams,
    ) -> std::result::Result<apis::timeline::RefreshTimelineResponse, ()> {
        Ok(match self.state.timeline.refresh(&path_params.library_id).await {
            Ok(status) => apis::timeline::RefreshTimelineResponse::Status202_TheScanIsRunning(Self::status(status)),
            Err(error) => ApiError::from(error).into(),
        })
    }
}

impl ApiServer {
    /// `after`、`before`、`month`、`around` のうち、指定できるのは 1 つだけである。
    fn anchor(query: &models::ListTimelineEntriesQueryParams) -> Result<TimelineAnchor> {
        let given = [query.after.is_some(), query.before.is_some(), query.month.is_some(), query.around.is_some()];
        if given.iter().filter(|given| **given).count() > 1 {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("at most one of after, before, month and around can be given"));
        }
        Ok(if let Some(after) = &query.after {
            TimelineAnchor::After(TimelineCursor::parse(after)?)
        } else if let Some(before) = &query.before {
            TimelineAnchor::Before(TimelineCursor::parse(before)?)
        } else if let Some(month) = &query.month {
            TimelineAnchor::Month(month.clone())
        } else if let Some(around) = &query.around {
            TimelineAnchor::Around(format!("/{}", crate::service::library::LibraryRelativePath::parse(around)?.segments().join("/")))
        } else {
            TimelineAnchor::Newest
        })
    }

    fn status(status: TimelineStatus) -> models::TimelineStatus {
        let mut model = models::TimelineStatus::new(status.running, status.processed_files);
        model.completed_at = status.completed_at;
        model
    }

    /// 索引の行を、フォルダの一覧と同じ形の Entry に並びの日時を足した応答にする。
    fn entry(row: &TimelineRow) -> models::TimelineEntry {
        let name = row.path.rsplit('/').next().unwrap_or_default().to_string();
        let mut metadata = models::FileSystemMetadata::new(row.size, row.readonly);
        metadata.modified_at = row.modified.map(Nullable::Present);
        metadata.content_type = MediaKind::content_type(&name).map(|content_type| Nullable::Present(content_type.to_string()));
        let mut entry = models::FileSystemEntry::new(row.path.clone(), name, metadata, models::FileSystemEntryKind::File);
        entry.file = Some(Nullable::Present(models::FileSystemEntryFileDetail::new(false)));
        models::TimelineEntry::new(entry, row.date.text(), row.date.source.as_str().to_string())
    }
}

impl From<ApiError> for apis::timeline::GetTimelineStatusResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
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
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::timeline::RefreshTimelineResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}
