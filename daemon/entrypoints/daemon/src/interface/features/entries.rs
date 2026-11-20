use std::collections::HashMap;

use async_trait::async_trait;
use axum_extra::extract::CookieJar;
use headers::Host;
use http::Method;

use omnius_pxna_interface::{apis, models};

use omnius_core_base::error::OmniError as _;

use crate::{
    error::{ApiError, ApiStatus, Error, ErrorKind},
    interface::{ApiServer, session::Claims},
    service::listing::{ListOptions, MarkSelection, Scope, ScopeEntry, ScopeListing, ScopeStart},
};

#[allow(unused_variables)]
#[async_trait]
impl apis::entries::Entries for ApiServer {
    type Claims = Claims;

    async fn list_entries(
        &self,
        method: &Method,
        host: &Host,
        cookies: &CookieJar,
        claims: &Self::Claims,
        query_params: &models::ListEntriesQueryParams,
    ) -> std::result::Result<apis::entries::ListEntriesResponse, ()> {
        let listing = async {
            let options = ListOptions::from_entries(query_params)?;
            let scope = Scope::parse(&query_params.start, options.recursive(), options.inside_archives())?;
            // `locate` names an Entry in the form of a start, which is how it is parsed.
            let located = match &query_params.locate {
                Some(locate) => {
                    if query_params.offset.is_some() {
                        return Err(Error::new(ErrorKind::InvalidInput).with_message("locate and offset cannot be given together"));
                    }
                    Some(ScopeStart::parse(locate)?)
                }
                None => None,
            };
            let selections = self.select(&options, &scope, &claims.user.id).await?;
            match located {
                Some(entry) => {
                    self.state
                        .library
                        .list_entries_located(&scope, &selections, &options, entry.library_id(), &entry.path().absolute())
                        .await
                }
                None => self.state.library.list_entries(&scope, &selections, &options).await,
            }
        }
        .await;
        Ok(match listing {
            Ok(listing) => apis::entries::ListEntriesResponse::Status200_TheEntriesOfTheScope(listing.into()),
            Err(error) => ApiError::from(error).into(),
        })
    }
}

impl ApiServer {
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

impl From<ScopeListing> for models::EntryListing {
    fn from(listing: ScopeListing) -> Self {
        let mut response = models::EntryListing::new(
            listing.entries.into_iter().map(models::ScopedEntry::from).collect(),
            listing.cut_off_starts,
            listing.walked_starts,
            listing.missing_starts,
        );
        response.total_count = Some(listing.total_count);
        response.offset = Some(listing.offset);
        response.snapshot = Some(listing.snapshot);
        response
    }
}

impl From<ScopeEntry> for models::ScopedEntry {
    fn from(scoped: ScopeEntry) -> Self {
        models::ScopedEntry::new(scoped.library_id, scoped.entry)
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
        Ok(body["entries"]
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
            .collect())
    }

    fn strings(body: &Value, field: &str) -> Vec<String> {
        body[field]
            .as_array()
            .map(|values| values.iter().map(|value| value.as_str().unwrap_or_default().to_string()).collect())
            .unwrap_or_default()
    }

    async fn get(router: &axum::Router, token: &str, query: &str) -> TestResult<HttpResponse> {
        send(router, http::Method::GET, &format!("{ENTRIES}?{query}"), &[("authorization", token)], None).await
    }

    /// Runs a full aggregation of `library` and waits for it, which makes its name index usable.
    async fn aggregate(state: &DaemonState, library: &str) -> TestResult {
        state.names.verified().await;
        state.timeline.refresh(library).await?;
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
        assert_eq!((&all.body["totalCount"], &all.body["offset"]), (&json!(6), &json!(0)));
        assert!(
            all.body["snapshot"].as_str().is_some_and(|snapshot| snapshot.contains(',')),
            "the ids of the two starts are joined"
        );
        for field in ["cutOffStarts", "walkedStarts", "missingStarts"] {
            assert_eq!(all.body[field], json!([]), "{field}");
        }

        let reversed = get(&router, &token, "start=videos/x&start=photos/x&recursive=true&sort=name&limit=100").await?;
        assert_eq!(&listed(&reversed.body)?[2..4], ["videos:/x/same.png", "photos:/x/same.png"]);

        for limit in [1, 2, 4] {
            let mut joined = Vec::new();
            let mut snapshots = Vec::new();
            for offset in (0..6).step_by(limit) {
                let page = get(&router, &token, &format!("{query}&limit={limit}&offset={offset}")).await?;
                assert_eq!(page.body["offset"], offset);
                assert_eq!(page.body["totalCount"], 6);
                joined.extend(listed(&page.body)?);
                snapshots.push(page.body["snapshot"].clone());
            }
            assert_eq!(joined, expected, "limit {limit}");
            assert!(snapshots.iter().all(|snapshot| *snapshot == all.body["snapshot"]));
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
        assert_eq!(base.body["totalCount"], 3);

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
            assert_eq!(response.body["totalCount"], direct.body["totalCount"], "{starts}");
            assert_eq!(listed(&response.body)?, listed(&direct.body)?, "{starts}");
        }
        // Without recursion only the same start is dropped: the direct children of the root and of `x` are both listed, `x` once.
        let flat = get(&router, &token, "start=photos/x&start=photos/x&start=photos&limit=100").await?;
        assert_eq!(
            listed(&flat.body)?,
            ["photos:/x/a.png", "photos:/photos", "photos:/x/same.png", "photos:/x", "photos:/x/z.txt"]
        );
        assert_eq!(flat.body["totalCount"], 5);

        // A start inside an archive is covered by a recursive start above it only with the insides, which the index answers once it is usable.
        aggregate(&state, "photos").await?;
        let root = get(&router, &token, "start=photos&recursive=true&limit=1000").await?;
        let with_archive = get(&router, &token, "start=photos&start=photos/photos/a.7z&recursive=true&limit=1000").await?;
        let inside_only = get(&router, &token, "start=photos/photos/a.7z&recursive=true&limit=1000").await?;
        assert_eq!(
            with_archive.body["totalCount"].as_u64(),
            Some(root.body["totalCount"].as_u64().unwrap_or_default() + inside_only.body["totalCount"].as_u64().unwrap_or_default()),
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
            covered.body["totalCount"], root_inside.body["totalCount"],
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
        assert_eq!(response.body["totalCount"], 3);

        // The starts that are missing are named on every page.
        let later = get(&router, &token, "start=photos/gone&start=photos/x&recursive=true&limit=1&offset=2").await?;
        assert_eq!(strings(&later.body, "missingStarts"), ["photos/gone"]);

        // With every start missing the result is empty and every start is named.
        let none = get(&router, &token, "start=photos/gone&start=nolibrary&start=photos/photos/notes.txt&recursive=true").await?;
        assert_eq!(none.status, StatusCode::OK, "{}", none.body);
        assert_eq!(listed(&none.body)?, Vec::<String>::new());
        assert_eq!((&none.body["totalCount"], &none.body["offset"]), (&json!(0), &json!(0)));
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

        // The folder listing API keeps its answer for the same path.
        let folder = send(
            &router,
            http::Method::GET,
            "/api/v1/libraries/photos/folders/entries?path=/photos/notes.txt/x",
            &[("authorization", token.as_str())],
            None,
        )
        .await?;
        assert_eq!(folder.status, StatusCode::BAD_REQUEST, "{}", folder.body);
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
        assert_eq!(before.body["totalCount"], 4);
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
            assert_eq!(located.body["offset"], offset, "{target}");
            let page = get(&router, &token, &format!("{query}&offset={offset}")).await?;
            assert_eq!(located.body["entries"], page.body["entries"], "{target}");
            assert_eq!(located.body["totalCount"], page.body["totalCount"]);
            assert_eq!(located.body["snapshot"], page.body["snapshot"]);
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
            "start=photos/a%5Cb".to_string(),
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
        let unauthenticated = send(&router, http::Method::GET, &format!("{ENTRIES}?start=photos"), &[], None).await?;
        assert_eq!(unauthenticated.status, StatusCode::UNAUTHORIZED);
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
                &format!("/api/v1/libraries/{library}/rating"),
                headers,
                Some(json!({"path": path, "rating": "good"})),
            )
            .await?;
            assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
        }
        let query = "start=photos/x&start=videos/x&recursive=true&limit=100";
        let good = get(&router, &token, &format!("{query}&rating=good")).await?;
        assert_eq!(listed(&good.body)?, ["photos:/x/a.png", "videos:/x/b.png"]);
        let unrated = get(&router, &token, &format!("{query}&rating=none")).await?;
        assert_eq!(listed(&unrated.body)?, ["videos:/x/b.mp4", "photos:/x/same.png", "videos:/x/same.png"]);
        let before = good.body["snapshot"].clone();
        send(
            &router,
            http::Method::PUT,
            "/api/v1/libraries/photos/rating",
            headers,
            Some(json!({"path": "/x/same.png", "rating": "good"})),
        )
        .await?;
        let after = get(&router, &token, &format!("{query}&rating=good")).await?;
        assert_ne!(after.body["snapshot"], before, "a rating write changes the id");
        assert_eq!(after.body["totalCount"], 3);
        let none = get(&router, &token, &format!("{query}&rating=good&kind=folder")).await?;
        assert_eq!(none.body["totalCount"], 0);
        Ok(())
    }
}
