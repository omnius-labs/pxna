use std::collections::HashMap;

use async_trait::async_trait;
use axum::body::Bytes;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_engine::service::ConflictPolicy;
use omnius_pxna_interface::{apis, models, types::ByteArray};

use omnius_core_base::error::OmniError as _;

use crate::{
    error::{ApiError, ApiStatus, Error, ErrorKind},
    interface::{ApiServer, binary_body::BinaryBody, session::Claims},
    service::{
        library::ReadRange,
        listing::{ListOptions, MarkSelection, Scope, ScopeEntry, ScopeListing, ScopeStart, SummaryOptions},
        mutation::LibraryMutation,
        timeline::{ScopeTimeline, ScopeTimelinePage, TimelineAnchor, TimelineCursor, TimelineIndex},
    },
};

mod commands;
mod content;

pub use content::*;

#[allow(unused_variables)]
#[async_trait]
impl apis::entries::Entries for ApiServer {
    type Claims = Claims;

    async fn get_entry_metadata(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::GetEntryMetadataPathParams,
        query_params: &models::GetEntryMetadataQueryParams,
    ) -> std::result::Result<apis::entries::GetEntryMetadataResponse, ()> {
        Ok(match self.state.library.get_entry(&path_params.library_id, &query_params.path, query_params.view).await {
            Ok(entry) => apis::entries::GetEntryMetadataResponse::Status200_TheEntry(models::EntryResponse::new(entry)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn read_file_content(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        header_params: &models::ReadFileContentHeaderParams,
        path_params: &models::ReadFileContentPathParams,
        query_params: &models::ReadFileContentQueryParams,
    ) -> std::result::Result<apis::entries::ReadFileContentResponse, ()> {
        let content = async {
            let range = ReadRange::parse(header_params.range.as_deref(), query_params.offset, query_params.limit)?;
            self.state.library.read_file_content(&path_params.library_id, &query_params.path, &range).await
        }
        .await;
        Ok(match content {
            Ok(content) => {
                let content_length = Some(content.bytes.len() as u64);
                let accept_ranges = Some("bytes".to_string());
                match content.content_range() {
                    Some(content_range) => apis::entries::ReadFileContentResponse::Status206_PartialFileBytes {
                        body: ByteArray(content.bytes),
                        accept_ranges,
                        content_range: Some(content_range),
                        content_length,
                        e_tag: None,
                    },
                    None => apis::entries::ReadFileContentResponse::Status200_FullOrRequestedFileBytes {
                        body: ByteArray(content.bytes),
                        accept_ranges,
                        content_length,
                        e_tag: None,
                    },
                }
            }
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn write_file_content(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::WriteFileContentPathParams,
        query_params: &models::WriteFileContentQueryParams,
        body: &Bytes,
    ) -> std::result::Result<apis::entries::WriteFileContentResponse, ()> {
        if body.len() > BinaryBody::MAX_BYTES {
            return Ok(ApiError::from(Error::new(ErrorKind::PayloadTooLarge)).into());
        }
        Ok(
            match self
                .state
                .mutation
                .write(&path_params.library_id, &query_params.path, body, LibraryMutation::write_conflict(query_params.conflict))
                .await
            {
                Ok(file) => apis::entries::WriteFileContentResponse::Status201_FileWritten(models::EntryResponse::new(file)),
                Err(error) => ApiError::from(error).into(),
            },
        )
    }

    async fn create_file(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CreateFilePathParams,
        query_params: &models::CreateFileQueryParams,
        body: &Bytes,
    ) -> std::result::Result<apis::entries::CreateFileResponse, ()> {
        if body.len() > BinaryBody::MAX_BYTES {
            return Ok(ApiError::from(Error::new(ErrorKind::PayloadTooLarge)).into());
        }
        Ok(
            match self.state.mutation.write(&path_params.library_id, &query_params.path, body, ConflictPolicy::Rename).await {
                Ok(entry) => apis::entries::CreateFileResponse::Status201_FileCreated(models::EntryResponse::new(entry)),
                Err(error) => ApiError::from(error).into(),
            },
        )
    }

    async fn create_folder(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        path_params: &models::CreateFolderPathParams,
        body: &models::CreateFolderRequest,
    ) -> std::result::Result<apis::entries::CreateFolderResponse, ()> {
        Ok(match self.state.mutation.create_folder(&path_params.library_id, body).await {
            Ok(file) => apis::entries::CreateFolderResponse::Status201_FolderCreated(models::EntryResponse::new(file)),
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
    ) -> std::result::Result<apis::entries::ProbeFolderChildrenResponse, ()> {
        Ok(
            match self
                .state
                .library
                .probe_folder_children(&path.library_id, &query.path, query.include_hidden.unwrap_or(false))
                .await
            {
                Ok(result) => apis::entries::ProbeFolderChildrenResponse::Status200_True(result),
                Err(error) => ApiError::from(error).into(),
            },
        )
    }

    async fn list_entries(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        body: &models::EntryListRequest,
    ) -> std::result::Result<apis::entries::ListEntriesResponse, ()> {
        let listing = async {
            if Self::summary(body)? {
                return self.list_summary(body).await;
            }
            let options = ListOptions::from_entries(body)?;
            let scope = Scope::from_request(&body.scope)?;
            let anchor = Self::timeline_anchor(body, &options)?;
            let selections = self.select(&options, &scope, &claims.user.id).await?;
            if let Some(anchor) = anchor {
                let page = ScopeTimeline::new(&self.state.library, &self.state.timeline)
                    .page(&scope, &selections, &options, &anchor, options.limit())
                    .await?;
                return Ok::<_, Error>(models::EntryPage::from(page));
            }
            let listing = match body.page.as_ref().and_then(|page| page.locate.as_ref()) {
                Some(locate) => {
                    if body.page.as_ref().is_some_and(|page| page.offset.is_some()) {
                        return Err(Error::new(ErrorKind::InvalidInput).with_message("locate and offset cannot be given together"));
                    }
                    let entry = ScopeStart::from_request(locate)?;
                    self.state
                        .library
                        .list_entries_located(&scope, &selections, &options, entry.library_id(), &entry.path().absolute())
                        .await?
                }
                None => self.state.library.list_entries(&scope, &selections, &options).await?,
            };
            Ok(listing.into())
        }
        .await;
        Ok(match listing {
            Ok(listing) => apis::entries::ListEntriesResponse::Status200_TheEntriesOfTheScope(listing),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn list_entry_metadata(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        body: &models::EntryMetadataRequest,
    ) -> std::result::Result<apis::entries::ListEntryMetadataResponse, ()> {
        Ok(match self.entry_metadata(&body.places).await {
            Ok(items) => apis::entries::ListEntryMetadataResponse::Status200_ExactlyOneEntryOrErrorKindPerPlace(models::EntryMetadataList::new(items)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn list_entry_months(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        body: &models::EntryMonthsRequest,
    ) -> std::result::Result<apis::entries::ListEntryMonthsResponse, ()> {
        let months = async {
            let options = ListOptions::from_entry_months(body)?;
            let scope = Scope::from_request(&body.scope)?;
            let selections = self.select(&options, &scope, &claims.user.id).await?;
            ScopeTimeline::new(&self.state.library, &self.state.timeline).months(&scope, &selections, &options).await
        }
        .await;
        Ok(match months {
            Ok(months) => {
                let months = months.into_iter().map(|(month, count)| models::TimelineMonth::new(month, count)).collect();
                apis::entries::ListEntryMonthsResponse::Status200_EntryCountsPerMonth(models::TimelineMonths::new(months))
            }
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn copy_entries(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        body: &models::EntryTransferRequest,
    ) -> std::result::Result<apis::entries::CopyEntriesResponse, ()> {
        Ok(match self.state.operations.copy(body).await {
            Ok(operation) => apis::entries::CopyEntriesResponse::Status202_TheOperation(models::OperationResponse::new(operation)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn move_entries(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        body: &models::EntryTransferRequest,
    ) -> std::result::Result<apis::entries::MoveEntriesResponse, ()> {
        Ok(match self.state.operations.move_entries(body).await {
            Ok(operation) => apis::entries::MoveEntriesResponse::Status202_TheOperation(models::OperationResponse::new(operation)),
            Err(error) => ApiError::from(error).into(),
        })
    }

    async fn delete_entries(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        body: &models::EntryDeleteRequest,
    ) -> std::result::Result<apis::entries::DeleteEntriesResponse, ()> {
        Ok(match self.state.operations.delete_entries(body).await {
            Ok(operation) => apis::entries::DeleteEntriesResponse::Status202_TheOperation(models::OperationResponse::new(operation)),
            Err(error) => ApiError::from(error).into(),
        })
    }
}

impl ApiServer {
    /// Whether the request asks for the summary of direct children rather than the Entries.
    fn summary(request: &models::EntryListRequest) -> crate::result::Result<bool> {
        match request.options.as_ref().and_then(|options| options.fields.as_deref()) {
            None | Some("full") => Ok(false),
            Some("summary") => Ok(true),
            Some(other) => Err(Error::new(ErrorKind::InvalidInput).with_message(format!("unknown fields: {other}"))),
        }
    }

    /// The summary of the direct children of the one start, as an Entry page of `summaries` cut by `offset` or at `locate`.
    async fn list_summary(&self, request: &models::EntryListRequest) -> crate::result::Result<models::EntryPage> {
        let options = SummaryOptions::from_entries(request)?;
        let start = ScopeStart::from_request(&request.scope.starts[0])?;
        let library_id = start.library_id();
        let path = start.path().absolute();
        let page = match request.page.as_ref().and_then(|page| page.locate.as_ref()) {
            Some(locate) => {
                if request.page.as_ref().is_some_and(|page| page.offset.is_some()) {
                    return Err(Error::new(ErrorKind::InvalidInput).with_message("locate and offset cannot be given together"));
                }
                let entry = ScopeStart::from_request(locate)?;
                if entry.library_id() != library_id {
                    return Err(Error::new(ErrorKind::NotFound).with_message("the entry is not in the listing"));
                }
                self.state
                    .library
                    .list_folder_summaries_located(library_id, &path, &options, &entry.path().absolute())
                    .await?
            }
            None => self.state.library.list_folder_summaries(library_id, &path, &options).await?,
        };
        let mut info = models::EntryPageInfo::new(models::EntryPageKind::Offset);
        info.offset = Some(page.offset);
        info.total_count = Some(page.total_count);
        info.snapshot = Some(page.snapshot);
        let mut response = models::EntryPage::new(info, models::EntryWarnings::new(vec![], vec![], vec![]));
        response.summaries = Some(
            page.entries
                .into_iter()
                .map(|summary| models::ScopedEntrySummary::new(library_id.to_string(), summary))
                .collect(),
        );
        Ok(response)
    }

    /// One result per place, in the order given. The places of a Library are read together; a Library that does not exist fails only its places.
    async fn entry_metadata(&self, places: &[models::LibraryPath]) -> crate::result::Result<Vec<models::EntryMetadataResult>> {
        let mut libraries: Vec<(&str, Vec<usize>)> = Vec::new();
        for (index, place) in places.iter().enumerate() {
            match libraries.iter_mut().find(|(id, _)| *id == place.library_id) {
                Some((_, indices)) => indices.push(index),
                None => libraries.push((&place.library_id, vec![index])),
            }
        }
        let mut results: Vec<Option<models::EntryMetadataResult>> = vec![None; places.len()];
        for (library_id, indices) in libraries {
            let paths = indices.iter().map(|&index| places[index].path.clone()).collect::<Vec<_>>();
            match self.state.library.entry_details(library_id, &paths).await {
                Ok(details) => {
                    for (index, detail) in indices.into_iter().zip(details) {
                        let mut result = models::EntryMetadataResult::new(library_id.to_string(), detail.path);
                        result.detail = detail.detail;
                        result.error = detail.error;
                        results[index] = Some(result);
                    }
                }
                Err(error) if *error.kind() == ErrorKind::NotFound => {
                    for index in indices {
                        let mut result = models::EntryMetadataResult::new(library_id.to_string(), places[index].path.clone());
                        result.error = Some(ApiError::from_error(&error).body.code);
                        results[index] = Some(result);
                    }
                }
                Err(error) => return Err(error),
            }
        }
        Ok(results.into_iter().flatten().collect())
    }

    /// Where a page in timeline-date order starts, or none when the listing is cut from snapshots.
    ///
    /// At most one of `after`, `before`, `month`, and `around` is given, only with `sort` of `timeline_date`, and `locate` is not accepted with it.
    fn timeline_anchor(request: &models::EntryListRequest, options: &ListOptions) -> crate::result::Result<Option<TimelineAnchor>> {
        let empty = models::EntryPageRequest::new(models::EntryPageKind::Cursor);
        let request = request.page.as_ref().unwrap_or(&empty);
        let given = [request.after.is_some(), request.before.is_some(), request.month.is_some(), request.around.is_some()];
        if given.iter().filter(|given| **given).count() > 1 {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("at most one of after, before, month and around can be given"));
        }
        if !options.is_timeline() {
            if given.contains(&true) {
                return Err(Error::new(ErrorKind::InvalidInput).with_message("after, before, month and around can be given only with sort=timeline_date"));
            }
            return Ok(None);
        }
        if request.locate.is_some() {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("locate cannot be given with sort=timeline_date"));
        }
        Ok(Some(if let Some(after) = &request.after {
            TimelineAnchor::After(TimelineCursor::parse(after)?)
        } else if let Some(before) = &request.before {
            TimelineAnchor::Before(TimelineCursor::parse(before)?)
        } else if let Some(month) = &request.month {
            TimelineIndex::month_end(month)?;
            TimelineAnchor::Month(month.clone())
        } else if let Some(around) = &request.around {
            let entry = ScopeStart::from_request(around)?;
            TimelineAnchor::Around {
                library: entry.library_id().to_string(),
                path: entry.path().absolute(),
            }
        } else {
            TimelineAnchor::Newest
        }))
    }

    /// What the rating and tag conditions of the Filter come to for the user in each Library that the Scope names and that exists.
    async fn select(&self, options: &ListOptions, scope: &Scope, user_id: &str) -> crate::result::Result<HashMap<String, MarkSelection>> {
        let mut selections = HashMap::new();
        if !options.marks().is_active() {
            return Ok(selections);
        }
        for start in scope.starts() {
            let library_id = start.library_id();
            if selections.contains_key(library_id) || self.state.library.get_library(library_id).is_err() {
                continue;
            }
            selections.insert(library_id.to_string(), options.marks().select(&self.state.rating_tags, user_id, library_id).await?);
        }
        Ok(selections)
    }
}

impl From<ScopeListing> for models::EntryPage {
    fn from(listing: ScopeListing) -> Self {
        let mut info = models::EntryPageInfo::new(models::EntryPageKind::Offset);
        info.offset = Some(listing.offset);
        info.total_count = Some(listing.total_count);
        info.snapshot = Some(listing.snapshot);
        let warnings = models::EntryWarnings::new(
            listing.cut_off_starts.iter().map(ScopeStart::to_response).collect(),
            listing.walked_starts.iter().map(ScopeStart::to_response).collect(),
            listing.missing_starts.iter().map(ScopeStart::to_response).collect(),
        );
        let mut response = models::EntryPage::new(info, warnings);
        response.items = Some(listing.entries.into_iter().map(models::ScopedEntry::from).collect());
        response
    }
}

impl From<ScopeTimelinePage> for models::EntryPage {
    fn from(page: ScopeTimelinePage) -> Self {
        let items = page
            .entries
            .into_iter()
            .map(|found| {
                let mut scoped = models::ScopedEntry::new(found.library, found.entry);
                scoped.timeline_date = Some(found.date.text());
                scoped.timeline_date_source = Some(found.date.source.as_str().to_string());
                scoped
            })
            .collect();
        let mut info = models::EntryPageInfo::new(models::EntryPageKind::Cursor);
        info.previous = page.previous.map(|cursor| cursor.encode());
        info.next = page.next.map(|cursor| cursor.encode());
        let warnings = models::EntryWarnings::new(
            page.cut_off_starts.iter().map(ScopeStart::to_response).collect(),
            page.walked_starts.iter().map(ScopeStart::to_response).collect(),
            page.missing_starts.iter().map(ScopeStart::to_response).collect(),
        );
        let mut response = models::EntryPage::new(info, warnings);
        response.items = Some(items);
        response
    }
}

impl From<ScopeEntry> for models::ScopedEntry {
    fn from(scoped: ScopeEntry) -> Self {
        models::ScopedEntry::new(scoped.library_id, scoped.entry)
    }
}

impl From<ApiError> for apis::entries::ListEntryMonthsResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            ApiStatus::Conflict
            | ApiStatus::RangeNotSatisfiable
            | ApiStatus::PayloadTooLarge
            | ApiStatus::InsufficientStorage
            | ApiStatus::Gone
            | ApiStatus::InternalServerError => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::entries::ListEntryMetadataResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::entries::ListEntriesResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            ApiStatus::Conflict
            | ApiStatus::RangeNotSatisfiable
            | ApiStatus::PayloadTooLarge
            | ApiStatus::InsufficientStorage
            | ApiStatus::Gone
            | ApiStatus::InternalServerError => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::entries::GetEntryMetadataResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            ApiStatus::RangeNotSatisfiable | ApiStatus::PayloadTooLarge | ApiStatus::InsufficientStorage | ApiStatus::Gone | ApiStatus::InternalServerError => {
                Self::Status500_InternalServerError(error.body)
            }
        }
    }
}

impl From<ApiError> for apis::entries::ReadFileContentResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            ApiStatus::RangeNotSatisfiable => Self::Status416_RequestedByteRangeCannotBeSatisfied(error.body),
            ApiStatus::GatewayTimeout => Self::Status504_ExtractionOrThumbnailGenerationTimedOut(error.body),
            ApiStatus::PayloadTooLarge | ApiStatus::InsufficientStorage | ApiStatus::Gone | ApiStatus::InternalServerError => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::entries::WriteFileContentResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            ApiStatus::PayloadTooLarge => Self::Status413_TheRequestBodyExceedsTheOne(error.body),
            ApiStatus::InsufficientStorage => Self::Status507_TheDaemonDoesNotHaveEnoughDiskSpace(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::entries::CreateFileResponse {
    fn from(error: ApiError) -> Self {
        match error.status {
            ApiStatus::BadRequest => Self::Status400_InvalidRequest(error.body),
            ApiStatus::Unauthorized => Self::Status401_MissingOrInvalidAuthentication(error.body),
            ApiStatus::Forbidden => Self::Status403_TheRequestedResourceCannotBeAccessedOrWritten(error.body),
            ApiStatus::NotFound => Self::Status404_ResourceNotFound(error.body),
            ApiStatus::Conflict => Self::Status409_TheRequestConflictsWithTheCurrentStateOfTheTarget(error.body),
            ApiStatus::PayloadTooLarge => Self::Status413_TheRequestBodyExceedsTheOne(error.body),
            ApiStatus::InsufficientStorage => Self::Status507_TheDaemonDoesNotHaveEnoughDiskSpace(error.body),
            _ => Self::Status500_InternalServerError(error.body),
        }
    }
}

impl From<ApiError> for apis::entries::CreateFolderResponse {
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

impl From<ApiError> for apis::entries::ProbeFolderChildrenResponse {
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

#[cfg(test)]
mod tests {
    use http::StatusCode;
    use omnius_pxna_engine::service::ArchiveOptions;
    use serde_json::{Value, json};
    use testresult::TestResult;

    use crate::{config::ListingConfig, interface::server::tests::*, state::DaemonState};

    const ENTRIES: &str = "/api/v1/entries";

    async fn login(router: &axum::Router) -> TestResult<String> {
        let setup = send(router, http::Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "bearer")).await?;
        bearer(&setup)
    }

    /// The Library and the path of every Entry of a page, as `library:path`.
    fn listed(body: &Value) -> TestResult<Vec<String>> {
        Ok(body["items"]
            .as_array()
            .ok_or("items is missing")?
            .iter()
            .map(|entry| {
                format!(
                    "{}:{}",
                    entry["libraryId"].as_str().unwrap_or_default(),
                    entry["entry"]["path"].as_str().unwrap_or_default()
                )
            })
            .collect())
    }

    /// The starts that a response names in `field`, each as its Library id and its path as given, the root without its `/`.
    fn strings(body: &Value, field: &str) -> Vec<String> {
        body["warnings"][field]
            .as_array()
            .map(|starts| {
                starts
                    .iter()
                    .map(|start| {
                        let path = start["path"].as_str().unwrap_or_default();
                        format!("{}{}", start["libraryId"].as_str().unwrap_or_default(), if path == "/" { "" } else { path })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A start or an Entry as the body holds it, from the text `library/path`: the Library id and the path with its leading `/`.
    fn library_path(value: &str) -> Value {
        let (library_id, path) = value.split_once('/').unwrap_or((value, ""));
        json!({"libraryId": library_id, "path": format!("/{path}")})
    }

    /// The JSON body of a request, from the parameters in the compact form `start=photos/x&start=videos&recursive=true&kind=image&sort=name`.
    /// Each `start`, `locate`, and `around` is a `library/path` text that becomes an object of `libraryId` and `path`.
    fn body_of(spec: &str) -> Value {
        let mut starts = Vec::new();
        let (mut scope, mut filter, mut body) = (serde_json::Map::new(), serde_json::Map::new(), serde_json::Map::new());
        let (mut options, mut page) = (serde_json::Map::new(), serde_json::Map::new());
        for (key, value) in spec.split('&').filter(|pair| !pair.is_empty()).map(|pair| pair.split_once('=').unwrap_or((pair, ""))) {
            let number = || value.parse::<u64>().map(Value::from).unwrap_or_else(|_| json!(value));
            match key {
                "start" => starts.push(library_path(value)),
                "recursive" | "insideArchives" => {
                    scope.insert(key.to_string(), json!(value == "true"));
                }
                "query" | "rating" => {
                    filter.insert(key.to_string(), json!(value));
                }
                "tag" | "kind" => {
                    let name = if key == "tag" { "tags" } else { "kinds" };
                    filter
                        .entry(name)
                        .or_insert_with(|| json!([]))
                        .as_array_mut()
                        .into_iter()
                        .for_each(|values| values.push(json!(value)));
                }
                "locate" | "around" => {
                    page.insert(key.to_string(), library_path(value));
                }
                "after" | "before" | "month" => {
                    page.insert(key.to_string(), json!(value));
                }
                "pageKind" => {
                    page.insert("kind".to_string(), json!(value));
                }
                "foldersFirst" | "includeHidden" => {
                    options.insert(key.to_string(), json!(value == "true"));
                }
                "fields" => {
                    options.insert(key.to_string(), json!(value));
                }
                "offset" | "limit" => {
                    page.insert(key.to_string(), number());
                }
                _ => {
                    body.insert(key.to_string(), json!(value));
                }
            }
        }
        scope.insert("starts".to_string(), Value::Array(starts));
        body.insert("scope".to_string(), Value::Object(scope));
        if !options.is_empty() {
            body.insert("options".to_string(), Value::Object(options));
        }
        let cursor = body.get("sort").is_some_and(|sort| sort == "timeline_date");
        page.entry("kind").or_insert_with(|| json!(if cursor { "cursor" } else { "offset" }));
        body.insert("page".to_string(), Value::Object(page));
        if !filter.is_empty() {
            body.insert("filter".to_string(), Value::Object(filter));
        }
        Value::Object(body)
    }

    async fn get(router: &axum::Router, token: &str, spec: &str) -> TestResult<HttpResponse> {
        send(router, http::Method::POST, ENTRIES, &[("authorization", token)], Some(body_of(spec))).await
    }

    /// Runs a full aggregation of `library` and waits for it, which makes its name index usable.
    async fn aggregate(state: &DaemonState, library: &str) -> TestResult {
        state.names.verified().await;
        state.timeline.scan(library)?;
        for _ in 0..400 {
            let status = state.timeline.status(library).await?;
            if !status.running && status.completed_at.is_some() {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        Err("the aggregation did not finish".into())
    }

    /// The test Libraries are `photos`, whose root holds the folder `photos` with `a.7z`, `outer.7z`, and `notes.txt`, and `videos`, which is empty.
    /// This adds a folder `x` with the same names to both, so that the same path is in two Libraries.
    fn write_pairs(fixture: &Fixture) -> TestResult {
        let dir = fixture._dir.path();
        for (root, names) in [(dir.join("library"), ["a.png", "same.png", "z.txt"]), (dir.join("videos"), ["b.mp4", "same.png", "y.txt"])] {
            std::fs::create_dir_all(root.join("x"))?;
            for name in names {
                std::fs::write(root.join("x").join(name), b"x")?;
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn lists_starts_in_two_libraries_as_one_sequence_with_joined_pages_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        write_pairs(&fixture)?;
        let router = router(&fixture);
        let token = login(&router).await?;

        let query = "start=photos/x&start=videos/x&recursive=true&sort=name";
        let all = get(&router, &token, &format!("{query}&limit=100")).await?;
        assert_eq!(all.status, StatusCode::OK, "{}", all.body);
        let expected = [
            "photos:/x/a.png",
            "videos:/x/b.mp4",
            "photos:/x/same.png",
            "videos:/x/same.png",
            "videos:/x/y.txt",
            "photos:/x/z.txt",
        ];
        assert_eq!(listed(&all.body)?, expected, "the same path follows the order of the starts");
        assert_eq!((&all.body["page"]["totalCount"], &all.body["page"]["offset"]), (&json!(6), &json!(0)));
        assert!(
            all.body["page"]["snapshot"].as_str().is_some_and(|snapshot| snapshot.contains(',')),
            "the ids of the two starts are joined"
        );
        for field in ["cutOffStarts", "walkedStarts", "missingStarts"] {
            assert_eq!(all.body["warnings"][field], json!([]), "{field}");
        }

        let reversed = get(&router, &token, "start=videos/x&start=photos/x&recursive=true&sort=name&limit=100").await?;
        assert_eq!(&listed(&reversed.body)?[2..4], ["videos:/x/same.png", "photos:/x/same.png"]);

        for limit in [1, 2, 4] {
            let mut joined = Vec::new();
            let mut snapshots = Vec::new();
            for offset in (0..6).step_by(limit) {
                let page = get(&router, &token, &format!("{query}&limit={limit}&offset={offset}")).await?;
                assert_eq!(page.body["page"]["offset"], offset);
                assert_eq!(page.body["page"]["totalCount"], 6);
                joined.extend(listed(&page.body)?);
                snapshots.push(page.body["page"]["snapshot"].clone());
            }
            assert_eq!(joined, expected, "limit {limit}");
            assert!(snapshots.iter().all(|snapshot| *snapshot == all.body["page"]["snapshot"]));
        }

        let descending = get(&router, &token, &format!("{query}&order=desc&limit=100")).await?;
        assert_eq!(listed(&descending.body)?.first().map(String::as_str), Some("photos:/x/z.txt"));
        Ok(())
    }

    #[tokio::test]
    async fn a_recursive_listing_returns_no_folder_unless_asked_and_direct_children_every_kind_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let root = fixture._dir.path().join("library");
        std::fs::create_dir_all(root.join("x/sub"))?;
        std::fs::create_dir_all(root.join("x/.hidden"))?;
        std::fs::write(root.join("x/sub/deep.png"), b"x")?;
        std::fs::write(root.join("x/clip.mp4"), b"x")?;
        std::fs::write(root.join("x/note.txt"), b"x")?;
        std::fs::write(root.join("x/.secret.png"), b"x")?;
        std::fs::write(root.join("x/.hidden/inside.png"), b"x")?;
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.join("x/clip.mp4"), root.join("x/link.png"))?;
        std::fs::copy(root.join("photos/a.7z"), root.join("x/pack.7z"))?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let (router_ref, token_ref) = (&router, &token);
        let list = |extra: &str| {
            let query = format!("start=photos/x&sort=name&limit=100&{extra}");
            async move { get(router_ref, token_ref, &query).await }
        };
        let link = cfg!(unix).then_some("photos:/x/link.png");

        // The index is not usable, so the start is walked and its archive is a file like the others.
        let default = list("recursive=true").await?;
        let mut expected = vec!["photos:/x/clip.mp4", "photos:/x/note.txt", "photos:/x/pack.7z", "photos:/x/sub/deep.png"];
        expected.extend(link);
        expected.sort();
        let mut found = listed(&default.body)?;
        found.sort();
        assert_eq!(found, expected, "no folder, no hidden Entry, and the link is a file");

        let named = |kinds: &[&str]| kinds.iter().map(|kind| format!("kind={kind}")).collect::<Vec<_>>().join("&");
        let only = |response: HttpResponse| -> TestResult<Vec<String>> {
            let mut found = listed(&response.body)?;
            found.sort();
            Ok(found)
        };
        assert_eq!(only(list(&format!("recursive=true&{}", named(&["folder"]))).await?)?, ["photos:/x/sub"]);
        assert_eq!(only(list(&format!("recursive=true&{}", named(&["archive"]))).await?)?, ["photos:/x/pack.7z"]);
        assert_eq!(only(list(&format!("recursive=true&{}", named(&["image", "video"]))).await?)?, {
            let mut media = vec!["photos:/x/clip.mp4", "photos:/x/sub/deep.png"];
            media.sort();
            media
        });
        let mut others = vec!["photos:/x/note.txt"];
        others.extend(link);
        others.sort();
        assert_eq!(
            only(list(&format!("recursive=true&{}", named(&["other"]))).await?)?,
            others,
            "a symbolic link is other whatever its name"
        );
        assert_eq!(only(list(&format!("recursive=true&{}", named(&["other", "folder"]))).await?)?.len(), others.len() + 1);

        // A listing of direct children returns every kind, the folder and the archive too, and no hidden Entry.
        let direct = only(list("").await?)?;
        let mut expected = vec!["photos:/x/clip.mp4", "photos:/x/note.txt", "photos:/x/pack.7z", "photos:/x/sub"];
        expected.extend(link);
        expected.sort();
        assert_eq!(direct, expected);
        // The insides of an archive are not read without recursion.
        assert_eq!(only(list("insideArchives=true").await?)?, expected);
        Ok(())
    }

    #[tokio::test]
    async fn the_insides_of_archives_come_from_the_index_and_from_an_archive_start_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let root = fixture._dir.path().join("library");
        std::fs::write(root.join("photos/plain.png"), b"x")?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let state = fixture.server.state.clone();

        // Before the aggregation a walk under a local folder reads no archive, so the archives are files, and the start is named as walked.
        let before = get(&router, &token, "start=photos/photos&recursive=true&insideArchives=true&limit=100").await?;
        let mut found = listed(&before.body)?;
        found.sort();
        assert_eq!(
            found,
            ["photos:/photos/a.7z", "photos:/photos/notes.txt", "photos:/photos/outer.7z", "photos:/photos/plain.png"]
        );
        assert_eq!(strings(&before.body, "walkedStarts"), ["photos/photos"]);

        aggregate(&state, "photos").await?;
        let inside = get(&router, &token, "start=photos/photos&recursive=true&insideArchives=true&limit=100").await?;
        let mut found = listed(&inside.body)?;
        found.sort();
        assert_eq!(
            found,
            [
                "photos:/photos/a.7z/dir/x.png",
                "photos:/photos/a.7z/y.txt",
                "photos:/photos/notes.txt",
                "photos:/photos/outer.7z/inner.7z/x.png",
                "photos:/photos/plain.png"
            ],
            "the insides from the index, nested archives down to the limit, and none of the archives"
        );
        assert!(strings(&inside.body, "walkedStarts").is_empty());
        // With no name, rating, or tag condition too, and the name condition reaches inside.
        let named = get(&router, &token, "start=photos/photos&recursive=true&insideArchives=true&query=x.png&limit=100").await?;
        assert_eq!(listed(&named.body)?, ["photos:/photos/a.7z/dir/x.png", "photos:/photos/outer.7z/inner.7z/x.png"]);
        let images = get(&router, &token, "start=photos/photos&recursive=true&insideArchives=true&kind=image&limit=100").await?;
        assert_eq!(listed(&images.body)?.len(), 3);

        // Without the insides each archive is one Entry and nothing inside it is listed.
        let outside = get(&router, &token, "start=photos/photos&recursive=true&query=x.png&limit=100").await?;
        assert!(listed(&outside.body)?.is_empty());
        let archives = get(&router, &token, "start=photos/photos&recursive=true&kind=archive&limit=100").await?;
        assert_eq!(listed(&archives.body)?, ["photos:/photos/a.7z", "photos:/photos/outer.7z"]);

        // A start that is an archive is walked inside whatever the Scope reads, and reads the archives nested in it only with the insides.
        let archive = get(&router, &token, "start=photos/photos/a.7z&recursive=true&limit=100").await?;
        assert_eq!(listed(&archive.body)?, ["photos:/photos/a.7z/dir/x.png", "photos:/photos/a.7z/y.txt"]);
        assert!(strings(&archive.body, "walkedStarts").is_empty(), "an archive start is walked as intended");
        let outer = get(&router, &token, "start=photos/photos/outer.7z&recursive=true&limit=100").await?;
        assert_eq!(listed(&outer.body)?, ["photos:/photos/outer.7z/inner.7z"], "the nested archive is one Entry");
        let outer_inside = get(&router, &token, "start=photos/photos/outer.7z&recursive=true&insideArchives=true&limit=100").await?;
        assert_eq!(listed(&outer_inside.body)?, ["photos:/photos/outer.7z/inner.7z/x.png"]);
        Ok(())
    }

    #[tokio::test]
    async fn covered_and_duplicated_starts_add_nothing_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        write_pairs(&fixture)?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let state = fixture.server.state.clone();
        let base = get(&router, &token, "start=photos/x&recursive=true&limit=100").await?;
        assert_eq!(base.body["page"]["totalCount"], 3);

        for starts in [
            "start=photos/x&start=photos/x",
            "start=photos/x&start=photos/x/",
            "start=photos/x/../x&start=photos/x",
            "start=photos&start=photos/x",
        ] {
            let response = get(&router, &token, &format!("{starts}&recursive=true&limit=100&kind=image&kind=other&kind=video")).await?;
            let direct = get(
                &router,
                &token,
                &format!("{}&recursive=true&limit=100&kind=image&kind=other&kind=video", starts.split('&').next().unwrap_or_default()),
            )
            .await?;
            assert_eq!(response.body["page"]["totalCount"], direct.body["page"]["totalCount"], "{starts}");
            assert_eq!(listed(&response.body)?, listed(&direct.body)?, "{starts}");
        }
        // Without recursion only the same start is dropped: the direct children of the root and of `x` are both listed, `x` once.
        let flat = get(&router, &token, "start=photos/x&start=photos/x&start=photos&limit=100").await?;
        assert_eq!(
            listed(&flat.body)?,
            ["photos:/x/a.png", "photos:/photos", "photos:/x/same.png", "photos:/x", "photos:/x/z.txt"]
        );
        assert_eq!(flat.body["page"]["totalCount"], 5);

        // A start inside an archive is covered by a recursive start above it only with the insides, which the index answers once it is usable.
        aggregate(&state, "photos").await?;
        let root = get(&router, &token, "start=photos&recursive=true&limit=1000").await?;
        let with_archive = get(&router, &token, "start=photos&start=photos/photos/a.7z&recursive=true&limit=1000").await?;
        let inside_only = get(&router, &token, "start=photos/photos/a.7z&recursive=true&limit=1000").await?;
        assert_eq!(
            with_archive.body["page"]["totalCount"].as_u64(),
            Some(root.body["page"]["totalCount"].as_u64().unwrap_or_default() + inside_only.body["page"]["totalCount"].as_u64().unwrap_or_default()),
            "without the insides the root lists the archive as one Entry, and the archive start adds its insides"
        );
        let root_inside = get(&router, &token, "start=photos&recursive=true&insideArchives=true&limit=1000").await?;
        let covered = get(
            &router,
            &token,
            "start=photos&start=photos/photos/a.7z&start=photos/photos/a.7z/dir&recursive=true&insideArchives=true&limit=1000",
        )
        .await?;
        assert_eq!(
            covered.body["page"]["totalCount"], root_inside.body["page"]["totalCount"],
            "with the insides the root covers the archive and what is inside it"
        );
        assert_eq!(listed(&covered.body)?, listed(&root_inside.body)?);
        Ok(())
    }

    #[tokio::test]
    async fn missing_starts_are_named_and_the_others_listed_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        write_pairs(&fixture)?;
        let router = router(&fixture);
        let token = login(&router).await?;

        // A deleted folder, a Library that is not configured, and a plain file; one of them under a start that covers its path.
        let response = get(
            &router,
            &token,
            "start=photos/gone&start=nolibrary/x&start=photos/x&start=photos/photos/notes.txt&start=photos/x/missing&recursive=true&limit=100",
        )
        .await?;
        assert_eq!(response.status, StatusCode::OK, "{}", response.body);
        assert_eq!(listed(&response.body)?, ["photos:/x/a.png", "photos:/x/same.png", "photos:/x/z.txt"]);
        assert_eq!(
            strings(&response.body, "missingStarts"),
            ["photos/gone", "nolibrary/x", "photos/photos/notes.txt", "photos/x/missing"]
        );
        assert_eq!(response.body["page"]["totalCount"], 3);

        // The starts that are missing are named on every page.
        let later = get(&router, &token, "start=photos/gone&start=photos/x&recursive=true&limit=1&offset=2").await?;
        assert_eq!(strings(&later.body, "missingStarts"), ["photos/gone"]);

        // With every start missing the result is empty and every start is named.
        let none = get(&router, &token, "start=photos/gone&start=nolibrary&start=photos/photos/notes.txt&recursive=true").await?;
        assert_eq!(none.status, StatusCode::OK, "{}", none.body);
        assert_eq!(listed(&none.body)?, Vec::<String>::new());
        assert_eq!((&none.body["page"]["totalCount"], &none.body["page"]["offset"]), (&json!(0), &json!(0)));
        assert_eq!(strings(&none.body, "missingStarts"), ["photos/gone", "nolibrary", "photos/photos/notes.txt"]);
        Ok(())
    }

    #[tokio::test]
    async fn a_start_through_a_plain_file_is_missing_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        write_pairs(&fixture)?;
        let router = router(&fixture);
        let token = login(&router).await?;

        let response = get(
            &router,
            &token,
            "start=photos/photos/notes.txt/x&start=photos/x&start=photos/photos/notes.txt/x/y/z&recursive=true&limit=100",
        )
        .await?;
        assert_eq!(response.status, StatusCode::OK, "{}", response.body);
        assert_eq!(listed(&response.body)?, ["photos:/x/a.png", "photos:/x/same.png", "photos:/x/z.txt"]);
        assert_eq!(strings(&response.body, "missingStarts"), ["photos/photos/notes.txt/x", "photos/photos/notes.txt/x/y/z"]);
        Ok(())
    }

    #[tokio::test]
    async fn a_walked_start_and_a_cut_off_start_are_named_test() -> TestResult {
        let listing = ListingConfig {
            recursive_max_entries: 3,
            ..Default::default()
        };
        let fixture = fixture_with_listing(
            ArchiveOptions::default(),
            None,
            std::sync::Arc::new(omnius_core_base::clock::ClockUtc),
            Default::default(),
            listing,
        )
        .await?;
        let dir = fixture._dir.path();
        std::fs::create_dir_all(dir.join("library/big"))?;
        for number in 0..6 {
            std::fs::write(dir.join(format!("library/big/sun-{number}.png")), b"x")?;
        }
        std::fs::write(dir.join("videos/sun.mp4"), b"x")?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let state = fixture.server.state.clone();

        // The index of `photos` is not usable, so its start is walked, and the walk is cut off at the limit.
        let before = get(&router, &token, "start=photos/big&start=videos&query=sun&recursive=true&limit=100").await?;
        assert_eq!(before.body["page"]["totalCount"], 4);
        assert_eq!(strings(&before.body, "walkedStarts"), ["photos/big", "videos"]);
        assert_eq!(strings(&before.body, "cutOffStarts"), ["photos/big"]);

        // A walk that is the intended answer is not named, and neither is a start answered by the index.
        let plain = get(&router, &token, "start=photos/big&recursive=true&limit=100").await?;
        assert!(strings(&plain.body, "walkedStarts").is_empty());
        aggregate(&state, "photos").await?;
        aggregate(&state, "videos").await?;
        let after = get(&router, &token, "start=photos/big&start=videos&query=sun&recursive=true&limit=100").await?;
        assert!(strings(&after.body, "walkedStarts").is_empty());
        assert_eq!(strings(&after.body, "cutOffStarts"), ["photos/big"], "the limit counts the matches of the index too");
        Ok(())
    }

    #[tokio::test]
    async fn locate_returns_the_page_of_its_offset_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        write_pairs(&fixture)?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let query = "start=photos/x&start=videos/x&recursive=true&sort=name&limit=2";

        for (target, offset) in [
            ("photos/x/a.png", 0),
            ("videos/x/b.mp4", 0),
            ("photos/x/same.png", 2),
            ("videos/x/same.png", 2),
            ("videos/x/y.txt", 4),
            ("photos/x/z.txt", 4),
        ] {
            let located = get(&router, &token, &format!("{query}&locate={target}")).await?;
            assert_eq!(located.status, StatusCode::OK, "{target}: {}", located.body);
            assert_eq!(located.body["page"]["offset"], offset, "{target}");
            let page = get(&router, &token, &format!("{query}&offset={offset}")).await?;
            assert_eq!(located.body["items"], page.body["items"], "{target}");
            assert_eq!(located.body["page"]["totalCount"], page.body["page"]["totalCount"]);
            assert_eq!(located.body["page"]["snapshot"], page.body["page"]["snapshot"]);
            assert!(listed(&located.body)?.contains(&target.replacen('/', ":/", 1)), "{target}");
        }
        for target in ["photos/x/missing.png", "videos/x/a.png", "nolibrary/x/a.png", "photos"] {
            let response = get(&router, &token, &format!("{query}&locate={target}")).await?;
            assert_eq!(response.status, StatusCode::NOT_FOUND, "{target}: {}", response.body);
        }
        Ok(())
    }

    #[tokio::test]
    async fn invalid_requests_are_rejected_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        write_pairs(&fixture)?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let sixty_five = (0..65).map(|number| format!("start=photos/{number}")).collect::<Vec<_>>().join("&");
        let sixty_four = (0..64).map(|number| format!("start=photos/{number}")).collect::<Vec<_>>().join("&");

        let accepted = get(&router, &token, &sixty_four).await?;
        assert_eq!(accepted.status, StatusCode::OK, "{}", accepted.body);
        assert_eq!(strings(&accepted.body, "missingStarts").len(), 64);

        let invalid = [
            sixty_five,
            String::new(),
            "recursive=true".to_string(),
            "start=".to_string(),
            "start=/photos".to_string(),
            "start=photos/../..".to_string(),
            "start=photos/a\\b".to_string(),
            "start=photos&locate=photos/x/a.png&offset=0".to_string(),
            "start=photos&locate=/x".to_string(),
            "start=photos&locate=photos/../..".to_string(),
            "start=photos&sort=bogus".to_string(),
            "start=photos&sort=timeline".to_string(),
            "start=photos&order=sideways".to_string(),
            "start=photos&kind=bogus".to_string(),
            "start=photos&kind=Image".to_string(),
            "start=photos&rating=great".to_string(),
            "start=photos&limit=0".to_string(),
            "start=photos&limit=1001".to_string(),
        ];
        for query in invalid {
            let response = get(&router, &token, &query).await?;
            assert_eq!(response.status, StatusCode::BAD_REQUEST, "{:.80}: {}", query, response.body);
        }
        let unauthenticated = send(&router, http::Method::POST, ENTRIES, &[], Some(body_of("start=photos"))).await?;
        assert_eq!(unauthenticated.status, StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    async fn a_start_or_an_entry_that_is_not_a_library_path_is_invalid_input_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        write_pairs(&fixture)?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];
        let long = "a".repeat(65);
        let start = |library_id: &str, path: &str| json!({"libraryId": library_id, "path": path});

        let mut bodies = vec![
            json!({"scope": {"starts": []}}),
            json!({"scope": {"starts": (0..65).map(|number| start("photos", &format!("/{number}"))).collect::<Vec<_>>()}}),
        ];
        for (library_id, path) in [
            ("", "/x"),
            (".", "/x"),
            ("..", "/x"),
            ("a b", "/x"),
            ("a/b", "/x"),
            (long.as_str(), "/x"),
            ("photos", ""),
            ("photos", "/.."),
            ("photos", "/x/../.."),
            ("photos", "/a\\b"),
            ("photos", "/C:/x"),
        ] {
            bodies.push(json!({"scope": {"starts": [start("photos", "/x"), start(library_id, path)]}}));
            bodies.push(json!({"scope": {"starts": [start("photos", "/x")]}, "page": {"kind": "offset", "locate": start(library_id, path)}}));
            bodies.push(json!({"scope": {"starts": [start("photos", "/x")]}, "sort": "timeline_date", "page": {"kind": "cursor", "around": start(library_id, path)}}));
        }
        for body in &bodies {
            for uri in [ENTRIES, &format!("{ENTRIES}/months")] {
                let response = send(&router, http::Method::POST, uri, headers, Some(body.clone())).await?;
                // The months request has no `locate` and no `around`, which it ignores, so it answers the starts only.
                let sorted_out = uri.ends_with("/months") && body.get("page").is_some() && body["scope"]["starts"].as_array().is_some_and(|starts| starts.len() == 1);
                if sorted_out {
                    continue;
                }
                assert_eq!(response.status, StatusCode::BAD_REQUEST, "{uri} {body}: {}", response.body);
            }
        }
        // Whatever the body, a body that is not a request is not answered with a listing.
        for body in [
            json!({}),
            json!({"filter": {}}),
            json!({"scope": {}}),
            json!({"scope": {"starts": "photos/x"}}),
            json!({"scope": {"starts": ["photos/x"]}}),
        ] {
            let response = send(&router, http::Method::POST, ENTRIES, headers, Some(body.clone())).await?;
            assert!(response.status.is_client_error(), "{body}: {}", response.status);
        }
        // A Library that is not configured is a start that is missing.
        let response = send(
            &router,
            http::Method::POST,
            ENTRIES,
            headers,
            Some(json!({"scope": {"starts": [start("nolibrary", "/x"), start("photos", "/")]}})),
        )
        .await?;
        assert_eq!(response.status, StatusCode::OK, "{}", response.body);
        assert_eq!(response.body["warnings"]["missingStarts"], json!([start("nolibrary", "/x")]));
        Ok(())
    }

    #[tokio::test]
    async fn starts_are_named_in_the_response_as_objects_with_the_path_given_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        write_pairs(&fixture)?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];
        let start = |library_id: &str, path: &str| json!({"libraryId": library_id, "path": path});

        let body = json!({"scope": {
            "recursive": true,
            "starts": [start("photos", "/gone/"), start("nolibrary", "/"), start("photos", "/gone/"), start("photos", "/x/../gone"), start("photos", "/x")],
        }});
        let response = send(&router, http::Method::POST, ENTRIES, headers, Some(body)).await?;
        assert_eq!(response.status, StatusCode::OK, "{}", response.body);
        assert_eq!(
            response.body["warnings"]["missingStarts"],
            json!([start("photos", "/gone/"), start("nolibrary", "/"), start("photos", "/x/../gone")]),
            "as given, and a start given twice once"
        );
        assert_eq!(response.body["page"]["totalCount"], 3);
        for field in ["cutOffStarts", "walkedStarts"] {
            assert_eq!(response.body["warnings"][field], json!([]), "{field}");
        }
        // The Filter and the choices of the Scope are optional, and `filter` may be empty.
        for body in [
            json!({"scope": {"starts": [start("photos", "/x")]}}),
            json!({"scope": {"starts": [start("photos", "/x")]}, "filter": {}}),
            json!({"scope": {"starts": [start("photos", "/x")], "recursive": false, "insideArchives": false}, "filter": {"tags": [], "kinds": []}}),
        ] {
            let response = send(&router, http::Method::POST, ENTRIES, headers, Some(body.clone())).await?;
            assert_eq!(response.status, StatusCode::OK, "{body}: {}", response.body);
            assert_eq!(response.body["page"]["totalCount"], 3, "{body}");
        }
        // `locate` is an Entry of a Library and a path.
        let located = send(
            &router,
            http::Method::POST,
            ENTRIES,
            headers,
            Some(
                json!({"scope": {"starts": [start("photos", "/x"), start("videos", "/x")], "recursive": true}, "sort": "name", "page": {"kind": "offset", "limit": 2, "locate": start("videos", "/x/y.txt")}}),
            ),
        )
        .await?;
        assert_eq!(located.status, StatusCode::OK, "{}", located.body);
        assert_eq!(located.body["page"]["offset"], 4);
        Ok(())
    }

    /// A folder written just now is rescanned after the first listing; age it so that two requests share a snapshot.
    fn age(path: &std::path::Path) -> std::io::Result<()> {
        let modified = std::time::SystemTime::now() - std::time::Duration::from_secs(10);
        filetime::set_file_mtime(path, filetime::FileTime::from_system_time(modified))
    }

    #[tokio::test]
    async fn a_summary_lists_the_direct_children_of_one_start_and_locates_an_entry_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let shelf = fixture._dir.path().join("library/shelf");
        std::fs::create_dir_all(&shelf)?;
        for name in ["a.png", "b.png", "c.png"] {
            std::fs::write(shelf.join(name), b"png")?;
        }
        age(&shelf)?;
        let router = router(&fixture);
        let token = login(&router).await?;

        let located = get(&router, &token, "start=photos/shelf&fields=summary&limit=1&locate=photos/shelf/b.png").await?;
        assert_eq!(located.status, StatusCode::OK, "{}", located.body);
        assert!(located.body.get("items").is_none(), "a summary carries summaries in place of items");
        assert_eq!(located.body["page"]["offset"], 1);
        assert_eq!(located.body["summaries"][0]["summary"]["name"], "b.png");
        assert_eq!(located.body["summaries"][0]["libraryId"], "photos");
        assert!(located.body["summaries"][0]["summary"].get("metadata").is_none(), "a summary has no metadata");
        let same = get(&router, &token, "start=photos/shelf&fields=summary&limit=1&offset=1").await?;
        assert_eq!(located.body["summaries"], same.body["summaries"]);
        assert_eq!(located.body["page"]["snapshot"], same.body["page"]["snapshot"]);

        for (spec, status) in [
            ("start=photos/shelf&fields=summary&locate=photos/shelf/a.png&offset=0", StatusCode::BAD_REQUEST),
            ("start=photos/shelf&fields=summary&locate=photos/shelf/missing.png", StatusCode::NOT_FOUND),
            ("start=photos/shelf&fields=summary&recursive=true", StatusCode::BAD_REQUEST),
            ("start=photos/shelf&start=videos&fields=summary", StatusCode::BAD_REQUEST),
            ("start=photos/shelf&fields=summary&sort=size", StatusCode::BAD_REQUEST),
            ("start=photos/shelf&fields=summary&kind=image", StatusCode::BAD_REQUEST),
        ] {
            assert_eq!(get(&router, &token, spec).await?.status, status, "{spec}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn a_listing_reads_hidden_entries_only_when_asked_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let photos = fixture._dir.path().join("library/photos");
        std::fs::create_dir_all(photos.join(".hidden"))?;
        std::fs::write(photos.join(".hidden/h.png"), b"h")?;
        std::fs::write(photos.join(".dot.png"), b"d")?;
        let router = router(&fixture);
        let token = login(&router).await?;

        let plain = get(&router, &token, "start=photos/photos&sort=name").await?;
        assert!(!listed(&plain.body)?.iter().any(|path| path.contains("/.")), "{:?}", listed(&plain.body)?);
        let hidden = get(&router, &token, "start=photos/photos&sort=name&includeHidden=true").await?;
        let paths = listed(&hidden.body)?;
        assert!(
            paths.contains(&"photos:/photos/.hidden".to_string()) && paths.contains(&"photos:/photos/.dot.png".to_string()),
            "{paths:?}"
        );
        let deep = get(&router, &token, "start=photos/photos&recursive=true&includeHidden=true&sort=name").await?;
        assert!(
            listed(&deep.body)?.contains(&"photos:/photos/.hidden/h.png".to_string()),
            "a recursive listing walks hidden folders when asked"
        );
        let tree = get(&router, &token, "start=photos/photos&kind=folder&kind=archive&includeHidden=true&fields=summary").await?;
        let names = tree.body["summaries"]
            .as_array()
            .ok_or("summaries is missing")?
            .iter()
            .map(|item| item["summary"]["name"].as_str().unwrap_or_default().to_string())
            .collect::<Vec<_>>();
        assert_eq!(names, [".hidden", "a.7z", "outer.7z"], "the tree's summary keeps folders and archives");
        Ok(())
    }

    #[tokio::test]
    async fn an_archive_start_lists_its_root_with_the_content_types_of_images_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        let photos = fixture._dir.path().join("library/photos");
        std::fs::write(photos.join("Pic.PNG"), b"png")?;
        std::fs::write(photos.join("clip.mp4"), b"mp4")?;
        let router = router(&fixture);
        let token = login(&router).await?;

        let root = get(&router, &token, "start=photos/photos/a.7z&sort=name").await?;
        assert_eq!(listed(&root.body)?, ["photos:/photos/a.7z/dir", "photos:/photos/a.7z/y.txt"]);
        assert_eq!(root.body["items"][0]["entry"]["kind"], "folder");

        let folder = get(&router, &token, "start=photos/photos&sort=name").await?;
        let content_type = |name: &str| {
            folder.body["items"]
                .as_array()
                .and_then(|items| items.iter().find(|item| item["entry"]["name"] == name))
                .and_then(|item| item["entry"]["metadata"]["contentType"].as_str().map(str::to_string))
        };
        assert_eq!(content_type("Pic.PNG").as_deref(), Some("image/png"));
        assert_eq!(content_type("clip.mp4").as_deref(), Some("video/mp4"));
        assert_eq!(content_type("notes.txt"), None);
        assert_eq!(content_type("a.7z"), None);
        Ok(())
    }

    #[tokio::test]
    async fn metadata_answers_each_place_of_several_libraries_in_order_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        write_pairs(&fixture)?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];
        let places = json!({"places": [
            library_path("videos/x/b.mp4"),
            library_path("photos/x/a.png"),
            library_path("photos/x/missing.png"),
            library_path("nolibrary/x"),
            library_path("photos/photos/a.7z/dir"),
        ]});
        let response = send(&router, http::Method::POST, &format!("{ENTRIES}/metadata"), headers, Some(places)).await?;
        assert_eq!(response.status, StatusCode::OK, "{}", response.body);
        let items = response.body["items"].as_array().ok_or("items is missing")?;
        let summary = items
            .iter()
            .map(|item| {
                format!(
                    "{}:{}:{}",
                    item["libraryId"].as_str().unwrap_or_default(),
                    item["path"].as_str().unwrap_or_default(),
                    item["error"].as_str().unwrap_or(if item["detail"].is_object() { "ok" } else { "-" })
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            summary,
            [
                "videos:/x/b.mp4:ok",
                "photos:/x/a.png:ok",
                "photos:/x/missing.png:NotFound",
                "nolibrary:/x:NotFound",
                "photos:/photos/a.7z/dir:ok",
            ],
            "one result per place in the order given, and a missing Library fails only its places"
        );
        let duplicated = json!({"places": [library_path("photos/x/a.png"), library_path("photos/x/./a.png")]});
        assert_eq!(
            send(&router, http::Method::POST, &format!("{ENTRIES}/metadata"), headers, Some(duplicated)).await?.status,
            StatusCode::BAD_REQUEST
        );
        Ok(())
    }

    #[tokio::test]
    async fn the_get_operations_are_gone_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        write_pairs(&fixture)?;
        let router = router(&fixture);
        let token = login(&router).await?;
        for uri in [
            format!("{ENTRIES}?start=photos/x&recursive=true"),
            format!("{ENTRIES}/months?start=photos/x&recursive=true"),
            ENTRIES.to_string(),
            format!("{ENTRIES}/months"),
        ] {
            let response = send(&router, http::Method::GET, &uri, &[("authorization", token.as_str())], None).await?;
            assert_eq!(response.status, StatusCode::METHOD_NOT_ALLOWED, "{uri}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn a_cookie_request_meets_the_origin_check_like_any_post_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        write_pairs(&fixture)?;
        let router = router(&fixture);
        let setup = send(&router, http::Method::POST, "/api/v1/setup", &[], credentials("owner", "password1", "cookie")).await?;
        let cookie = cookie(&setup)?;
        let body = body_of("start=photos/x&recursive=true&limit=100");

        for uri in [ENTRIES.to_string(), format!("{ENTRIES}/months")] {
            let same = send(
                &router,
                http::Method::POST,
                &uri,
                &[("cookie", &cookie), ("origin", "http://localhost")],
                Some(body.clone()),
            )
            .await?;
            assert_eq!(same.status, StatusCode::OK, "{uri}: {}", same.body);
            let missing = send(&router, http::Method::POST, &uri, &[("cookie", &cookie)], Some(body.clone())).await?;
            assert_eq!(missing.status, StatusCode::FORBIDDEN, "{uri}");
            let foreign = send(
                &router,
                http::Method::POST,
                &uri,
                &[("cookie", &cookie), ("origin", "http://evil.example")],
                Some(body.clone()),
            )
            .await?;
            assert_eq!(foreign.status, StatusCode::FORBIDDEN, "{uri}");
        }
        // The same Entries as a bearer request, which has no `Origin` to check.
        let login = send(&router, http::Method::POST, "/api/v1/auth/login", &[], credentials("owner", "password1", "bearer")).await?;
        let token = bearer(&login)?;
        let bearer = send(&router, http::Method::POST, ENTRIES, &[("authorization", &token)], Some(body.clone())).await?;
        let same = send(&router, http::Method::POST, ENTRIES, &[("cookie", &cookie), ("origin", "http://localhost")], Some(body)).await?;
        assert_eq!(bearer.body["items"], same.body["items"]);
        Ok(())
    }

    #[tokio::test]
    async fn rating_and_tag_conditions_apply_in_each_library_test() -> TestResult {
        let fixture = fixture(ArchiveOptions::default()).await?;
        write_pairs(&fixture)?;
        std::fs::write(fixture._dir.path().join("videos/x/b.png"), b"x")?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let headers = &[("authorization", token.as_str())];
        for (library, path) in [("photos", "/x/a.png"), ("videos", "/x/b.png")] {
            let saved = send(
                &router,
                http::Method::PUT,
                &format!("/api/v1/libraries/{library}/entries/rating?path={path}"),
                headers,
                Some(json!({"rating": "good"})),
            )
            .await?;
            assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
        }
        let query = "start=photos/x&start=videos/x&recursive=true&limit=100";
        let good = get(&router, &token, &format!("{query}&rating=good")).await?;
        assert_eq!(listed(&good.body)?, ["photos:/x/a.png", "videos:/x/b.png"]);
        let unrated = get(&router, &token, &format!("{query}&rating=none")).await?;
        assert_eq!(listed(&unrated.body)?, ["videos:/x/b.mp4", "photos:/x/same.png", "videos:/x/same.png"]);
        let before = good.body["page"]["snapshot"].clone();
        send(
            &router,
            http::Method::PUT,
            "/api/v1/libraries/photos/entries/rating?path=/x/same.png",
            headers,
            Some(json!({"rating": "good"})),
        )
        .await?;
        let after = get(&router, &token, &format!("{query}&rating=good")).await?;
        assert_ne!(after.body["page"]["snapshot"], before, "a rating write changes the id");
        assert_eq!(after.body["page"]["totalCount"], 3);
        let none = get(&router, &token, &format!("{query}&rating=good&kind=folder")).await?;
        assert_eq!(none.body["page"]["totalCount"], 0);
        Ok(())
    }

    /// Writes a file of `size` bytes whose modification time, which is its timeline date, is noon UTC on `date`, as `year-month-day`.
    fn write_dated(path: &std::path::Path, size: usize, date: (i32, u32, u32)) -> TestResult {
        use chrono::TimeZone as _;
        std::fs::create_dir_all(path.parent().ok_or("a path under a folder")?)?;
        std::fs::write(path, vec![b'x'; size])?;
        let time = chrono::Utc.with_ymd_and_hms(date.0, date.1, date.2, 12, 0, 0).single().ok_or("a date")?;
        std::fs::File::options().write(true).open(path)?.set_modified(time.into())?;
        Ok(())
    }

    /// Images and videos in two Libraries dated by their modification times, and the fixture's folder `photos` with two archives in `photos`.
    /// The same timeline date is in both Libraries, `photos` before `videos`.
    fn write_dated_files(fixture: &Fixture) -> TestResult {
        let dir = fixture._dir.path();
        write_dated(&dir.join("library/photos/p1.png"), 1, (2024, 3, 15))?;
        write_dated(&dir.join("library/photos/p2.mp4"), 2, (2024, 3, 14))?;
        write_dated(&dir.join("library/photos/sub/p3.png"), 3, (2024, 2, 15))?;
        write_dated(&dir.join("library/top.png"), 4, (2024, 1, 15))?;
        write_dated(&dir.join("videos/x/v1.mp4"), 5, (2024, 3, 15))?;
        write_dated(&dir.join("videos/v2.png"), 6, (2023, 12, 15))?;
        Ok(())
    }

    async fn aggregated_fixture() -> TestResult<Fixture> {
        let fixture = fixture(ArchiveOptions::default()).await?;
        write_dated_files(&fixture)?;
        aggregate(&fixture.server.state, "photos").await?;
        aggregate(&fixture.server.state, "videos").await?;
        Ok(fixture)
    }

    const TIMELINE: &str = "sort=timeline_date&limit=100";
    const BOTH: &str = "start=photos&start=videos&recursive=true";
    const OUTSIDE_ORDER: [&str; 6] = [
        "photos:/photos/p1.png",
        "videos:/x/v1.mp4",
        "photos:/photos/p2.mp4",
        "photos:/photos/sub/p3.png",
        "photos:/top.png",
        "videos:/v2.png",
    ];

    /// The months of a response of `listEntryMonths`, as `(month, count)`.
    fn month_counts(body: &Value) -> Vec<(String, u64)> {
        body["months"]
            .as_array()
            .map(|months| {
                months
                    .iter()
                    .map(|month| (month["month"].as_str().unwrap_or_default().to_string(), month["count"].as_u64().unwrap_or_default()))
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn months(router: &axum::Router, token: &str, spec: &str) -> TestResult<HttpResponse> {
        send(router, http::Method::POST, &format!("{ENTRIES}/months"), &[("authorization", token)], Some(body_of(spec))).await
    }

    /// Every Entry that `query` lists in timeline-date order, read page by page by `after` from `next` with `limit`, and the same read back by `before`.
    async fn walk(router: &axum::Router, token: &str, query: &str, limit: usize) -> TestResult<Vec<String>> {
        let mut seen = Vec::new();
        let mut page = get(router, token, &format!("{query}&sort=timeline_date&limit={limit}")).await?;
        assert_eq!(page.status, StatusCode::OK, "{query}: {}", page.body);
        assert!(page.body["page"]["previous"].is_null());
        loop {
            seen.extend(listed(&page.body)?);
            let Some(next) = page.body["page"]["next"].as_str().map(str::to_string) else { break };
            page = get(router, token, &format!("{query}&sort=timeline_date&limit={limit}&after={next}")).await?;
            assert_eq!(page.status, StatusCode::OK, "{query}: {}", page.body);
        }
        let all = get(router, token, &format!("{query}&sort=timeline_date&limit=1000")).await?;
        let mut back = Vec::new();
        if let Some(last) = all.body["items"].as_array().and_then(|entries| entries.last()) {
            let cursor = format!(
                "{}{}{}",
                last["timelineDate"].as_str().unwrap_or_default(),
                last["libraryId"].as_str().unwrap_or_default(),
                last["entry"]["path"].as_str().unwrap_or_default()
            );
            back.push(format!(
                "{}:{}",
                last["libraryId"].as_str().unwrap_or_default(),
                last["entry"]["path"].as_str().unwrap_or_default()
            ));
            let mut page = get(router, token, &format!("{query}&sort=timeline_date&limit={limit}&before={cursor}")).await?;
            loop {
                let mut current = listed(&page.body)?;
                current.extend(back);
                back = current;
                let Some(previous) = page.body["page"]["previous"].as_str().map(str::to_string) else {
                    break;
                };
                page = get(router, token, &format!("{query}&sort=timeline_date&limit={limit}&before={previous}")).await?;
            }
        }
        assert_eq!(back, seen, "{query} read back by {limit}");
        Ok(seen)
    }

    #[tokio::test]
    async fn timeline_date_order_reads_two_libraries_by_cursor_month_and_around_test() -> TestResult {
        let fixture = aggregated_fixture().await?;
        let router = router(&fixture);
        let token = login(&router).await?;

        let all = get(&router, &token, &format!("{BOTH}&{TIMELINE}")).await?;
        assert_eq!(all.status, StatusCode::OK, "{}", all.body);
        assert_eq!(
            listed(&all.body)?,
            OUTSIDE_ORDER,
            "newest first, the same date by Library id, and nothing inside an archive"
        );
        for field in ["totalCount", "offset", "snapshot", "previous", "next"] {
            assert!(all.body.get(field).is_none(), "{field} is absent");
        }
        let first = &all.body["items"][0];
        assert_eq!(
            (first["timelineDate"].as_str().map(|date| &date[..10]), first["timelineDateSource"].as_str()),
            (Some("2024-03-15"), Some("modified"))
        );
        assert_eq!(first["entry"]["name"], "p1.png");
        for field in ["cutOffStarts", "walkedStarts", "missingStarts"] {
            assert!(strings(&all.body, field).is_empty());
        }
        let first_page = get(&router, &token, &format!("{BOTH}&sort=timeline_date&limit=2")).await?;
        assert!(first_page.body["page"]["previous"].is_null() && first_page.body["page"]["next"].is_string());

        // The pages join without a repeat or a gap in both directions, whatever their size.
        for limit in 1..=4 {
            assert_eq!(walk(&router, &token, BOTH, limit).await?, OUTSIDE_ORDER, "limit {limit}");
        }

        // A month starts at its newest Entry, or at the next older month that has one.
        let february = get(&router, &token, &format!("{BOTH}&sort=timeline_date&limit=2&month=2024-02")).await?;
        assert_eq!(listed(&february.body)?, ["photos:/photos/sub/p3.png", "photos:/top.png"]);
        assert!(february.body["page"]["previous"].is_string() && february.body["page"]["next"].is_string());
        let gap = get(&router, &token, &format!("{BOTH}&{TIMELINE}&month=2024-01")).await?;
        assert_eq!(listed(&gap.body)?, ["photos:/top.png", "videos:/v2.png"]);

        // Around names an Entry in the form of a start and puts it near the middle.
        let around = get(&router, &token, &format!("{BOTH}&sort=timeline_date&limit=3&around=photos/photos/p2.mp4")).await?;
        assert_eq!(listed(&around.body)?, ["videos:/x/v1.mp4", "photos:/photos/p2.mp4", "photos:/photos/sub/p3.png"]);
        for missing in ["photos/photos/notes.txt", "photos/nope.png", "videos/photos/p1.png", "other/x.png"] {
            let response = get(&router, &token, &format!("{BOTH}&{TIMELINE}&around={missing}")).await?;
            assert_eq!(response.status, StatusCode::NOT_FOUND, "{missing}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn timeline_date_order_reads_folders_archives_and_direct_children_test() -> TestResult {
        let fixture = aggregated_fixture().await?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let list = |query: &'static str| {
            let (router, token) = (router.clone(), token.clone());
            async move {
                let response = get(&router, &token, &format!("{query}&{TIMELINE}")).await?;
                assert_eq!(response.status, StatusCode::OK, "{query}: {}", response.body);
                TestResult::Ok((listed(&response.body)?, strings(&response.body, "missingStarts")))
            }
        };

        assert_eq!(
            list("start=photos/photos&recursive=true").await?.0,
            ["photos:/photos/p1.png", "photos:/photos/p2.mp4", "photos:/photos/sub/p3.png"],
            "only the rows under the folder, and nothing inside its archives"
        );
        let inside = list("start=photos/photos&recursive=true&insideArchives=true").await?.0;
        assert_eq!(inside.len(), 3 + 2, "the image inside a.7z and the one inside the archive nested in outer.7z");
        assert!(inside.contains(&"photos:/photos/a.7z/dir/x.png".to_string()) && inside.contains(&"photos:/photos/outer.7z/inner.7z/x.png".to_string()));
        assert_eq!(list("start=photos/photos").await?.0, ["photos:/photos/p1.png", "photos:/photos/p2.mp4"], "direct children");
        assert_eq!(list("start=photos/photos&insideArchives=true").await?.0, ["photos:/photos/p1.png", "photos:/photos/p2.mp4"]);

        // A start that is an archive, or inside one, reads every row under it, the nested archives included, with or without insideArchives.
        for query in [
            "start=photos/photos/outer.7z&recursive=true",
            "start=photos/photos/outer.7z&recursive=true&insideArchives=true",
            "start=photos/photos/outer.7z/inner.7z&recursive=true",
        ] {
            assert_eq!(list(query).await?.0, ["photos:/photos/outer.7z/inner.7z/x.png"], "{query}");
        }
        assert!(list("start=photos/photos/a.7z").await?.0.is_empty(), "`dir/x.png` is not a direct child of the archive");
        assert_eq!(list("start=photos/photos/a.7z&recursive=true").await?.0, ["photos:/photos/a.7z/dir/x.png"]);

        // The kinds: both without `kind`, and only the named ones with it.
        assert_eq!(
            list("start=photos&start=videos&recursive=true&kind=video").await?.0,
            ["videos:/x/v1.mp4", "photos:/photos/p2.mp4"]
        );
        assert_eq!(
            list("start=photos&start=videos&recursive=true&kind=image").await?.0,
            ["photos:/photos/p1.png", "photos:/photos/sub/p3.png", "photos:/top.png", "videos:/v2.png"]
        );
        assert_eq!(list("start=photos&start=videos&recursive=true&kind=image&kind=video").await?.0, OUTSIDE_ORDER);
        for kind in ["folder", "archive", "other"] {
            let response = get(&router, &token, &format!("{BOTH}&{TIMELINE}&kind=image&kind={kind}")).await?;
            assert_eq!(response.status, StatusCode::BAD_REQUEST, "{kind}");
            let response = months(&router, &token, &format!("{BOTH}&kind={kind}")).await?;
            assert_eq!(response.status, StatusCode::BAD_REQUEST, "months {kind}");
        }

        // Covered starts add nothing and missing starts are named.
        let (covered, missing) = list("start=photos&start=photos/photos&start=nope/x&start=photos/gone&start=photos&recursive=true").await?;
        assert_eq!(covered, list("start=photos&recursive=true").await?.0);
        assert_eq!(missing, ["nope/x", "photos/gone"]);
        Ok(())
    }

    #[tokio::test]
    async fn timeline_date_order_applies_conditions_and_names_cut_off_and_walked_starts_test() -> TestResult {
        let listing = ListingConfig {
            recursive_max_entries: 3,
            ..Default::default()
        };
        let fixture = fixture_with_listing(
            ArchiveOptions::default(),
            None,
            std::sync::Arc::new(omnius_core_base::clock::ClockUtc),
            Default::default(),
            listing,
        )
        .await?;
        write_dated_files(&fixture)?;
        let dir = fixture._dir.path();
        for number in 0..6 {
            write_dated(&dir.join(format!("library/big/sun-{number}.png")), 1, (2024, 3, 10 + number))?;
        }
        write_dated(&dir.join("videos/sun.mp4"), 1, (2024, 3, 1))?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let state = fixture.server.state.clone();

        // The index of `photos` is not usable and nothing is aggregated, so the matches are found by a walk and have no timeline date yet.
        let query = "start=photos/big&start=videos&query=sun&recursive=true";
        let before = get(&router, &token, &format!("{query}&{TIMELINE}")).await?;
        assert_eq!(before.status, StatusCode::OK, "{}", before.body);
        assert!(listed(&before.body)?.is_empty(), "a match without a row is not returned");
        assert_eq!(strings(&before.body, "walkedStarts"), ["photos/big", "videos"]);
        assert_eq!(
            strings(&before.body, "cutOffStarts"),
            ["photos/big", "videos"],
            "the walk counts the folders and files of `videos` too"
        );
        assert!(month_counts(&months(&router, &token, query).await?.body).is_empty());

        aggregate(&state, "photos").await?;
        aggregate(&state, "videos").await?;
        let after = get(&router, &token, &format!("{query}&{TIMELINE}")).await?;
        assert!(strings(&after.body, "walkedStarts").is_empty());
        assert_eq!(strings(&after.body, "cutOffStarts"), ["photos/big"]);
        assert_eq!(listed(&after.body)?.len(), 3 + 1, "the matches of each start up to the limit");
        let dates = after.body["items"]
            .as_array()
            .ok_or("entries")?
            .iter()
            .map(|entry| entry["timelineDate"].as_str().unwrap_or_default().to_string())
            .collect::<Vec<_>>();
        assert!(dates.windows(2).all(|pair| pair[0] >= pair[1]), "newest first: {dates:?}");
        assert_eq!(walk(&router, &token, query, 1).await?, listed(&after.body)?);
        let counted = months(&router, &token, query).await?;
        assert_eq!(counted.status, StatusCode::OK, "{}", counted.body);
        assert_eq!(month_counts(&counted.body), [("2024-03".to_string(), 4)]);
        // Without a condition there is no limit, no walk, and nothing to name.
        let plain = get(&router, &token, &format!("start=photos/big&start=videos&recursive=true&{TIMELINE}")).await?;
        assert_eq!(listed(&plain.body)?.len(), 9);
        assert!(strings(&plain.body, "cutOffStarts").is_empty() && strings(&plain.body, "walkedStarts").is_empty());

        // A name condition over the whole Scope keeps the images that have it, in the order of the rows.
        let named = get(&router, &token, &format!("{BOTH}&query=png&{TIMELINE}")).await?;
        let images = get(&router, &token, &format!("{BOTH}&kind=image&{TIMELINE}")).await?;
        assert_eq!(listed(&named.body)?.len(), 3 + 1, "the matches of `photos`, cut at the limit of 3, and of `videos`");
        assert_eq!(strings(&named.body, "cutOffStarts"), ["photos"]);
        assert_eq!(listed(&images.body)?.len(), 4 + 6);

        // A rating condition keeps the rated matches, and a write of ratings changes what the next read returns.
        let headers = &[("authorization", token.as_str())];
        for (library, path) in [("photos", "/photos/p1.png"), ("videos", "/v2.png")] {
            let saved = send(
                &router,
                http::Method::PUT,
                &format!("/api/v1/libraries/{library}/entries/rating?path={path}"),
                headers,
                Some(json!({"rating": "good"})),
            )
            .await?;
            assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
        }
        let good = get(&router, &token, &format!("{BOTH}&rating=good&{TIMELINE}")).await?;
        assert_eq!(listed(&good.body)?, ["photos:/photos/p1.png", "videos:/v2.png"]);
        assert_eq!(
            month_counts(&months(&router, &token, &format!("{BOTH}&rating=good")).await?.body),
            [("2024-03".to_string(), 1), ("2023-12".to_string(), 1)]
        );
        send(
            &router,
            http::Method::PUT,
            "/api/v1/libraries/photos/entries/rating?path=/top.png",
            headers,
            Some(json!({"rating": "good"})),
        )
        .await?;
        let more = get(&router, &token, &format!("{BOTH}&rating=good&{TIMELINE}")).await?;
        assert_eq!(listed(&more.body)?, ["photos:/photos/p1.png", "photos:/top.png", "videos:/v2.png"]);
        Ok(())
    }

    #[tokio::test]
    async fn the_months_equal_the_pages_over_every_scope_and_filter_test() -> TestResult {
        let fixture = aggregated_fixture().await?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let mut checked = 0;
        for starts in [
            "start=photos&start=videos",
            "start=videos",
            "start=photos/photos",
            "start=photos/photos&start=videos/x",
            "start=photos/photos/outer.7z&start=videos",
            "start=photos/photos/a.7z",
        ] {
            for choices in ["recursive=true", "recursive=true&insideArchives=true", "recursive=false"] {
                for kinds in ["", "&kind=image", "&kind=video", "&kind=image&kind=video"] {
                    for query in ["", "&query=png", "&query=x"] {
                        let filter = format!("{starts}&{choices}{kinds}{query}");
                        let seen = walk(&router, &token, &filter, 3).await?;
                        let pages = get(&router, &token, &format!("{filter}&{TIMELINE}")).await?;
                        let mut counted = Vec::<(String, u64)>::new();
                        for entry in pages.body["items"].as_array().ok_or("entries")? {
                            let month = entry["timelineDate"].as_str().unwrap_or_default()[..7].to_string();
                            match counted.last_mut() {
                                Some((last, count)) if *last == month => *count += 1,
                                _ => counted.push((month, 1)),
                            }
                        }
                        let response = months(&router, &token, &filter).await?;
                        assert_eq!(response.status, StatusCode::OK, "{filter}: {}", response.body);
                        assert_eq!(month_counts(&response.body), counted, "{filter}");
                        assert_eq!(seen.len(), counted.iter().map(|(_, count)| *count as usize).sum::<usize>(), "{filter}");
                        checked += seen.len();
                    }
                }
            }
        }
        assert!(checked > 100, "the readings are not all empty");
        let unauthenticated = send(&router, http::Method::POST, &format!("{ENTRIES}/months"), &[], Some(body_of(BOTH))).await?;
        assert_eq!(unauthenticated.status, StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    async fn timeline_date_order_rejects_what_it_does_not_accept_test() -> TestResult {
        let fixture = aggregated_fixture().await?;
        let router = router(&fixture);
        let token = login(&router).await?;
        let cursor = "2024-03-15T12:00:00photos/photos/p1.png";
        for query in [
            "order=asc",
            "offset=0",
            "offset=5",
            "locate=photos/photos/p1.png",
            "foldersFirst=true",
            "foldersFirst=false",
            &format!("after={cursor}&before={cursor}"),
            &format!("after={cursor}&month=2024-02"),
            "month=2024-02&around=photos/photos/p1.png",
            "after=2024-03",
            "after=nonsense",
            "before=2024-03-15T12:00:00nolibrary",
            "month=2024-13",
            "month=2024-2",
            "month=202402",
            "around=/photos/p1.png",
            "start=",
        ] {
            let response = get(&router, &token, &format!("{BOTH}&sort=timeline_date&{query}")).await?;
            assert_eq!(response.status, StatusCode::BAD_REQUEST, "{query}: {}", response.body);
        }
        // An Entry that is a Library's root is not one of the sequence.
        let root = get(&router, &token, &format!("{BOTH}&{TIMELINE}&around=nolibrary")).await?;
        assert_eq!(root.status, StatusCode::NOT_FOUND);
        // The anchors belong to timeline-date order only.
        for query in [
            format!("after={cursor}"),
            format!("before={cursor}"),
            "month=2024-02".to_string(),
            "around=photos/photos/p1.png".to_string(),
        ] {
            for sort in ["", "&sort=name", "&sort=modified_at"] {
                let response = get(&router, &token, &format!("{BOTH}{sort}&{query}")).await?;
                assert_eq!(response.status, StatusCode::BAD_REQUEST, "{sort} {query}");
            }
        }
        // An order that descends is accepted, and so is a sort that is not a timeline.
        let descending = get(&router, &token, &format!("{BOTH}&{TIMELINE}&order=desc")).await?;
        assert_eq!(listed(&descending.body)?, OUTSIDE_ORDER);
        // An offset page in timeline-date order, and a cursor page in another order, are refused.
        for spec in [
            format!("{BOTH}&{TIMELINE}&pageKind=offset"),
            format!("{BOTH}&sort=name&pageKind=cursor"),
            format!("{BOTH}&{TIMELINE}&includeHidden=true"),
        ] {
            let response = get(&router, &token, &spec).await?;
            assert_eq!(response.status, StatusCode::BAD_REQUEST, "{spec}: {}", response.body);
        }
        // A listing cut from snapshots answers as before, with `totalCount` and without the cursors.
        let named = get(&router, &token, &format!("{BOTH}&sort=name&limit=100")).await?;
        assert_eq!(named.status, StatusCode::OK);
        assert!(named.body["page"]["totalCount"].is_number() && named.body["page"]["previous"].is_null() && named.body["items"][0]["timelineDate"].is_null());
        Ok(())
    }
}
