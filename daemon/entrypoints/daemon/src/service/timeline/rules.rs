use chrono::format::{Fixed, Item, ParseErrorKind, Parsed, StrftimeItems, parse};
use chrono::{NaiveDateTime, NaiveTime};
use glob::{MatchOptions, Pattern};
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::{TimelineDate, TimelineZone};
use crate::{config::LibraryConfig, prelude::*};

const NAME_CAPTURE: &str = "date";

const GLOB_OPTIONS: MatchOptions = MatchOptions {
    case_sensitive: true,
    require_literal_separator: true,
    require_literal_leading_dot: false,
};

/// One date detection rule, written as `[[timeline.date_rules]]` in TOML.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DateRuleConfig {
    /// The id of the Library the rule applies to. Omit it to apply the rule to every Library.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library: Option<String>,
    /// A glob matched against the Library-relative path. The leading `/` may be omitted.
    pub path: String,
    /// A regular expression partially matched against the file name including its extension. It must contain one `(?P<date>...)`.
    pub name: String,
    /// A chrono strftime string that reads the text captured by `date`. A date alone is enough; a missing time becomes 00:00:00.
    pub format: String,
    /// The time zone the `format` value is in. When omitted, the value is kept without a time zone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
}

struct DateRule {
    library: Option<String>,
    path: Pattern,
    name: Regex,
    format: Vec<Item<'static>>,
    zone: Option<TimelineZone>,
}

/// The ordered `[[timeline.date_rules]]`, which read timeline dates from file names.
pub struct DateRules {
    rules: Vec<DateRule>,
    fingerprint: String,
}

impl DateRules {
    pub fn new(configs: &[DateRuleConfig], libraries: &[LibraryConfig]) -> Result<Self> {
        let mut rules = Vec::with_capacity(configs.len());
        for (index, config) in configs.iter().enumerate() {
            rules.push(Self::compile(index, config, libraries)?);
        }
        let fingerprint = serde_json::to_string(configs).map_err(|error| Error::from_error(error, ErrorKind::UnexpectedError))?;
        Ok(Self { rules, fingerprint })
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// Use the first value, in rule order, whose Library, path, and file name match and whose text parses as a date and time.
    pub fn date(&self, display: &TimelineZone, library: &str, path: &str, file_name: &str) -> Option<TimelineDate> {
        let path = path.trim_start_matches('/');
        for rule in &self.rules {
            if rule.library.as_deref().is_some_and(|candidate| candidate != library) {
                continue;
            }
            if !rule.path.matches_with(path, GLOB_OPTIONS) {
                continue;
            }
            let Some(captured) = rule.name.captures(file_name).and_then(|captures| captures.name(NAME_CAPTURE)) else {
                continue;
            };
            let Some(value) = Self::parse_value(&rule.format, captured.as_str()) else {
                continue;
            };
            if let Some(date) = TimelineDate::from_file_name(display, rule.zone.as_ref(), value) {
                return Some(date);
            }
        }
        None
    }

    fn compile(index: usize, config: &DateRuleConfig, libraries: &[LibraryConfig]) -> Result<DateRule> {
        let invalid = |field: &str, message: String| Error::new(ErrorKind::InvalidFormat).with_message(format!("invalid [timeline] date_rules[{index}].{field}: {message}"));

        if let Some(library) = &config.library
            && !libraries.iter().any(|candidate| candidate.id == *library)
        {
            return Err(invalid("library", format!("unknown library id: {library:?}")));
        }

        let path_text = config.path.trim_start_matches('/');
        if path_text.is_empty() {
            return Err(invalid("path", "must not be empty".to_string()));
        }
        let path = Pattern::new(path_text).map_err(|error| invalid("path", error.to_string()))?;

        let name = Regex::new(&config.name).map_err(|error| invalid("name", error.to_string()))?;
        if !name.capture_names().flatten().any(|capture| capture == NAME_CAPTURE) {
            return Err(invalid("name", format!("must contain a named capture (?P<{NAME_CAPTURE}>...)")));
        }

        let format = StrftimeItems::new(&config.format).parse_to_owned().map_err(|error| invalid("format", error.to_string()))?;
        if format.iter().any(|item| matches!(item, Item::Fixed(fixed) if Self::has_offset(fixed))) {
            return Err(invalid("format", "must not contain a time zone or offset; use time_zone for that".to_string()));
        }

        let zone = match &config.time_zone {
            Some(name) => Some(TimelineZone::new(Some(name)).map_err(|error| invalid("time_zone", error.message().unwrap_or_default().to_string()))?),
            None => None,
        };

        Ok(DateRule {
            library: config.library.clone(),
            path,
            name,
            format,
            zone,
        })
    }

    fn has_offset(fixed: &Fixed) -> bool {
        matches!(
            fixed,
            Fixed::TimezoneName
                | Fixed::TimezoneOffsetColon
                | Fixed::TimezoneOffsetDoubleColon
                | Fixed::TimezoneOffsetTripleColon
                | Fixed::TimezoneOffsetColonZ
                | Fixed::TimezoneOffset
                | Fixed::TimezoneOffsetZ
                | Fixed::RFC2822
                | Fixed::RFC3339
        )
    }

    fn parse_value(format: &[Item<'static>], text: &str) -> Option<NaiveDateTime> {
        let mut parsed = Parsed::new();
        parse(&mut parsed, text, format.iter()).ok()?;
        let date = parsed.to_naive_date().ok()?;
        let time = match parsed.to_naive_time() {
            Ok(time) => time,
            Err(error) if error.kind() == ParseErrorKind::NotEnough => NaiveTime::MIN,
            Err(_) => return None,
        };
        Some(date.and_time(time))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library(id: &str) -> LibraryConfig {
        LibraryConfig {
            id: id.to_string(),
            name: id.to_string(),
            path: "/library".into(),
            readonly: false,
        }
    }

    fn rule(path: &str, name: &str, format: &str) -> DateRuleConfig {
        DateRuleConfig {
            library: None,
            path: path.to_string(),
            name: name.to_string(),
            format: format.to_string(),
            time_zone: None,
        }
    }

    fn rules(configs: &[DateRuleConfig]) -> DateRules {
        DateRules::new(configs, &[library("contents")]).unwrap()
    }

    #[test]
    fn filename_dates_follow_the_first_successful_rule_test() -> testresult::TestResult {
        let zone = TimelineZone::new(Some("Asia/Tokyo"))?;
        let rules = rules(&[
            rule("**/WhatsApp/**", r"IMG-(?P<date>\d{8}_\d{6})", "%Y%m%d_%H%M%S"),
            rule("**", r"(?P<date>\d{4}-\d{2}-\d{2})", "%Y-%m-%d"),
        ]);

        let date = rules.date(&zone, "contents", "/WhatsApp/IMG-20240501_123456.jpg", "IMG-20240501_123456.jpg").unwrap();
        assert_eq!(
            (date.text(), date.source),
            ("2024-05-01T12:34:56".to_string(), crate::service::timeline::TimelineDateSource::FileName)
        );
        assert_eq!(date.instant, None, "a rule without time_zone keeps the naive value");

        let date = rules.date(&zone, "contents", "/other/2024-05-01.jpg", "2024-05-01.jpg").unwrap();
        assert_eq!(date.text(), "2024-05-01T00:00:00", "a date-only format fills 00:00:00");
        assert_eq!(rules.date(&zone, "contents", "/other/none.jpg", "none.jpg"), None);
        Ok(())
    }

    #[test]
    fn broken_value_falls_through_to_the_next_rule_test() -> testresult::TestResult {
        let zone = TimelineZone::new(Some("UTC"))?;
        let rules = rules(&[rule("**", r"IMG-(?P<date>\d{8})", "%Y%m%d%H%M%S"), rule("**", r"IMG-(?P<date>\d{8})", "%Y%m%d")]);
        let date = rules.date(&zone, "contents", "/IMG-20240501.jpg", "IMG-20240501.jpg").unwrap();
        assert_eq!(date.text(), "2024-05-01T00:00:00");
        Ok(())
    }

    #[test]
    fn rule_time_zone_becomes_an_instant_test() -> testresult::TestResult {
        let display = TimelineZone::new(Some("UTC"))?;
        let tokyo = TimelineZone::new(Some("Asia/Tokyo"))?;
        let mut config = rule("**", r"IMG-(?P<date>\d{14})", "%Y%m%d%H%M%S");
        config.time_zone = Some("Asia/Tokyo".to_string());
        let rules = rules(&[config]);

        let date = rules.date(&display, "contents", "/IMG-20240501123456.jpg", "IMG-20240501123456.jpg").unwrap();
        assert_eq!(date.text(), "2024-05-01T03:34:56", "displayed in the UTC [timeline] time_zone");
        assert_eq!(date.instant.unwrap().to_string(), "2024-05-01 03:34:56 UTC");
        assert_eq!(date.in_zone(&tokyo).text(), "2024-05-01T12:34:56");
        Ok(())
    }

    #[test]
    fn paths_are_matched_without_a_leading_slash_test() -> testresult::TestResult {
        let zone = TimelineZone::new(Some("UTC"))?;
        let rules = rules(&[rule("**/nested/**", r"(?P<date>\d{8})", "%Y%m%d"), rule("a.7z/**", r"(?P<date>\d{8})", "%Y%m%d")]);

        assert!(rules.date(&zone, "contents", "/x/nested/20240501.jpg", "20240501.jpg").is_some());
        assert!(rules.date(&zone, "contents", "/a.7z/inner/20240501.jpg", "20240501.jpg").is_some());
        assert!(rules.date(&zone, "contents", "/a.7z/20240501.jpg", "20240501.jpg").is_some(), "** matches no directory too");
        Ok(())
    }

    #[test]
    fn library_filters_and_validation_are_reported_test() {
        let mut config = rule("**", r"(?P<date>\d{8})", "%Y%m%d");
        config.library = Some("other".to_string());
        let error = DateRules::new(&[config.clone()], &[library("contents")]).err().unwrap();
        assert!(error.message().unwrap_or_default().contains("date_rules[0].library"));

        config.library = Some("contents".to_string());
        config.name = r"IMG-(\d{8})".to_string();
        let error = DateRules::new(&[config], &[library("contents")]).err().unwrap();
        assert!(error.message().unwrap_or_default().contains("(?P<date>"));

        let error = DateRules::new(&[rule("**", r"(?P<date>\d{8})", "%Y%m%d%z")], &[library("contents")]).err().unwrap();
        assert!(error.message().unwrap_or_default().contains("time zone"));

        let error = DateRules::new(&[rule("**", r"(?P<date>\d{8})", "%Y%m%d")], &[library("contents")]).err();
        assert!(error.is_none());

        let mut invalid_zone = rule("**", r"(?P<date>\d{8})", "%Y%m%d");
        invalid_zone.time_zone = Some("Not/AZone".to_string());
        let error = DateRules::new(&[invalid_zone], &[library("contents")]).err().unwrap();
        assert!(error.message().unwrap_or_default().contains("time_zone"));
    }

    #[test]
    fn fingerprint_tracks_the_rule_list_test() {
        let first = rules(&[rule("**", r"(?P<date>\d{8})", "%Y%m%d")]);
        let same = rules(&[rule("**", r"(?P<date>\d{8})", "%Y%m%d")]);
        let changed = rules(&[rule("**", r"(?P<date>\d{8})", "%Y%m%d"), rule("**", r"(?P<date>\d{6})", "%Y%m")]);
        assert_eq!(first.fingerprint(), same.fingerprint());
        assert_ne!(first.fingerprint(), changed.fingerprint());
        assert_eq!(rules(&[]).fingerprint(), "[]");
    }
}
