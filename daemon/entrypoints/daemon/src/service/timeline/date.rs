use chrono::{DateTime, Datelike as _, NaiveDate, NaiveDateTime, Timelike as _, Utc};

use crate::prelude::*;

/// 並びの日時をどの値から得たか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineDateSource {
    Exif,
    CreationTime,
    Modified,
    ArchiveModified,
    FileName,
}

impl TimelineDateSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exif => "exif",
            Self::CreationTime => "creationTime",
            Self::Modified => "modified",
            Self::ArchiveModified => "archiveModified",
            Self::FileName => "fileName",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "exif" => Ok(Self::Exif),
            "creationTime" => Ok(Self::CreationTime),
            "modified" => Ok(Self::Modified),
            "archiveModified" => Ok(Self::ArchiveModified),
            "fileName" => Ok(Self::FileName),
            _ => Err(Error::new(ErrorKind::InvalidFormat).with_message(format!("unknown timeline date source: {value}"))),
        }
    }
}

/// タイムラインで Entry を並べる、タイムゾーンを持たない日時と、その出どころ。
///
/// UTC の瞬間から得た日時は、タイムゾーンを変えたときに直せるよう、元の瞬間も持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineDate {
    pub value: NaiveDateTime,
    pub source: TimelineDateSource,
    pub instant: Option<DateTime<Utc>>,
}

impl TimelineDate {
    /// 索引と API で使う形。年が 4 桁の範囲では、文字列の順と日時の順が一致する。
    pub const FORMAT: &'static str = "%Y-%m-%dT%H:%M:%S";

    /// Use the first value out of the file name, the contents, the Entry's modification time, and the containing archive's modification time.
    pub fn choose(zone: &TimelineZone, file_name: Option<Self>, content: Option<Self>, modified: Option<DateTime<Utc>>, archive_modified: Option<DateTime<Utc>>) -> Option<Self> {
        file_name
            .or(content)
            .or_else(|| modified.and_then(|time| Self::from_instant(zone, time, TimelineDateSource::Modified)))
            .or_else(|| archive_modified.and_then(|time| Self::from_instant(zone, time, TimelineDateSource::ArchiveModified)))
    }

    /// A value read by a file name rule. A rule time zone makes the value an instant in that zone.
    pub fn from_file_name(display: &TimelineZone, rule_zone: Option<&TimelineZone>, value: NaiveDateTime) -> Option<Self> {
        match rule_zone {
            Some(rule_zone) => rule_zone.to_utc(value).and_then(|time| Self::from_instant(display, time, TimelineDateSource::FileName)),
            None => Self::in_range(value).then_some(Self {
                value,
                source: TimelineDateSource::FileName,
                instant: None,
            }),
        }
    }

    /// Exif の撮影日時（`YYYY-MM-DDTHH:MM:SS`）から作る。読めない値と、4 桁に収まらない年は `None` にする。
    pub fn from_exif(taken_at: &str) -> Option<Self> {
        let value = NaiveDateTime::parse_from_str(taken_at, Self::FORMAT).ok()?;
        Self::in_range(value).then_some(Self {
            value,
            source: TimelineDateSource::Exif,
            instant: None,
        })
    }

    pub fn from_creation_time(zone: &TimelineZone, time: DateTime<Utc>) -> Option<Self> {
        Self::from_instant(zone, time, TimelineDateSource::CreationTime)
    }

    fn from_instant(zone: &TimelineZone, time: DateTime<Utc>, source: TimelineDateSource) -> Option<Self> {
        let value = zone.local(time);
        Self::in_range(value).then_some(Self {
            value,
            source,
            instant: Some(time),
        })
    }

    /// 索引から読んだ値で作る。
    pub fn parse(value: &str, source: &str, instant: Option<i64>) -> Result<Self> {
        let value =
            NaiveDateTime::parse_from_str(value, Self::FORMAT).map_err(|error| Error::from_error(error, ErrorKind::InvalidFormat).with_message("unreadable timeline date"))?;
        let instant = instant.map(DateTime::from_timestamp_nanos);
        Ok(Self {
            value,
            source: TimelineDateSource::parse(source)?,
            instant,
        })
    }

    /// 同じ瞬間を、別のタイムゾーンで直した値にする。瞬間を持たない値はそのまま返す。
    pub fn in_zone(self, zone: &TimelineZone) -> Self {
        match self.instant {
            Some(time) => Self { value: zone.local(time), ..self },
            None => self,
        }
    }

    pub fn text(&self) -> String {
        self.value.format(Self::FORMAT).to_string()
    }

    /// 索引に置く元の瞬間（UNIX 時刻のナノ秒）。
    pub fn instant_nanos(&self) -> Option<i64> {
        self.instant.and_then(|time| time.timestamp_nanos_opt())
    }

    fn in_range(value: NaiveDateTime) -> bool {
        (1000..=9999).contains(&value.year())
    }
}

/// 更新日時と `creation_time` を、並びの日時へ直すタイムゾーン。
#[derive(Debug, Clone)]
pub struct TimelineZone {
    zone: jiff::tz::TimeZone,
    name: String,
}

impl TimelineZone {
    /// IANA の名前で指定する。省略したときは環境変数 `TZ` か OS の設定から得て、どちらからも得られなければ UTC にする。
    pub fn new(name: Option<&str>) -> Result<Self> {
        let zone = match name {
            Some(name) => jiff::tz::TimeZone::get(name).map_err(|error| Error::from_error(error, ErrorKind::InvalidFormat).with_message(format!("unknown time zone: {name}")))?,
            None => jiff::tz::TimeZone::try_system().unwrap_or(jiff::tz::TimeZone::UTC),
        };
        let name = zone.iana_name().map(str::to_string).unwrap_or_else(|| name.unwrap_or("UTC").to_string());
        Ok(Self { zone, name })
    }

    /// 索引に記録する名前。
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn local(&self, time: DateTime<Utc>) -> NaiveDateTime {
        let Ok(timestamp) = jiff::Timestamp::new(time.timestamp(), time.timestamp_subsec_nanos() as i32) else {
            return time.naive_utc();
        };
        let local = self.zone.to_datetime(timestamp);
        NaiveDate::from_ymd_opt(i32::from(local.year()), local.month() as u32, local.day() as u32)
            .and_then(|date| date.and_hms_opt(local.hour() as u32, local.minute() as u32, local.second() as u32))
            .unwrap_or_else(|| time.naive_utc())
    }

    /// Turn a naive time in this zone into a UTC instant. A nonexistent or ambiguous time is resolved by jiff's default.
    pub fn to_utc(&self, value: NaiveDateTime) -> Option<DateTime<Utc>> {
        let civil = jiff::civil::DateTime::new(
            i16::try_from(value.year()).ok()?,
            i8::try_from(value.month()).ok()?,
            i8::try_from(value.day()).ok()?,
            i8::try_from(value.hour()).ok()?,
            i8::try_from(value.minute()).ok()?,
            i8::try_from(value.second()).ok()?,
            i32::try_from(value.nanosecond()).ok()?,
        )
        .ok()?;
        let timestamp = self.zone.to_zoned(civil).ok()?.timestamp();
        DateTime::from_timestamp(timestamp.as_second(), u32::try_from(timestamp.subsec_nanosecond()).ok()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(text: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(text).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn choose_uses_the_first_available_value_test() -> testresult::TestResult {
        let zone = TimelineZone::new(Some("UTC"))?;
        let exif = TimelineDate::from_exif("2024-05-01T12:34:56");
        let modified = Some(utc("2025-01-02T03:04:05Z"));
        let archive = Some(utc("2026-01-01T00:00:00Z"));

        let chosen = TimelineDate::choose(&zone, exif, None, modified, archive).unwrap();
        assert_eq!((chosen.text(), chosen.source), ("2024-05-01T12:34:56".to_string(), TimelineDateSource::Exif));
        let chosen = TimelineDate::choose(&zone, None, None, modified, archive).unwrap();
        assert_eq!((chosen.text(), chosen.source), ("2025-01-02T03:04:05".to_string(), TimelineDateSource::Modified));
        let chosen = TimelineDate::choose(&zone, None, None, None, archive).unwrap();
        assert_eq!((chosen.text(), chosen.source), ("2026-01-01T00:00:00".to_string(), TimelineDateSource::ArchiveModified));
        assert_eq!(TimelineDate::choose(&zone, None, None, None, None), None);

        let file_name = TimelineDate::from_file_name(&zone, None, NaiveDate::from_ymd_opt(2024, 4, 3).unwrap().and_hms_opt(2, 1, 0).unwrap()).unwrap();
        let chosen = TimelineDate::choose(&zone, Some(file_name), exif, modified, archive).unwrap();
        assert_eq!((chosen.text(), chosen.source), ("2024-04-03T02:01:00".to_string(), TimelineDateSource::FileName));
        Ok(())
    }

    #[test]
    fn unreadable_exif_values_are_ignored_test() {
        assert_eq!(TimelineDate::from_exif("0000-00-00T00:00:00"), None);
        assert_eq!(TimelineDate::from_exif("2024:05:01 12:34:56"), None);
        assert_eq!(TimelineDate::from_exif("0999-01-01T00:00:00"), None);
    }

    #[test]
    fn instants_are_converted_to_the_configured_zone_test() -> testresult::TestResult {
        let tokyo = TimelineZone::new(Some("Asia/Tokyo"))?;
        let date = TimelineDate::from_creation_time(&tokyo, utc("2024-12-31T20:00:00Z")).unwrap();
        assert_eq!(date.text(), "2025-01-01T05:00:00", "日付と年をまたいで 9 時間を足す");
        assert_eq!(date.source, TimelineDateSource::CreationTime);
        assert_eq!(tokyo.name(), "Asia/Tokyo");

        let utc_zone = TimelineZone::new(Some("UTC"))?;
        assert_eq!(date.in_zone(&utc_zone).text(), "2024-12-31T20:00:00", "元の瞬間から直す");
        let exif = TimelineDate::from_exif("2024-05-01T12:34:56").unwrap();
        assert_eq!(exif.in_zone(&tokyo), exif, "Exif の値は直さない");
        Ok(())
    }

    #[test]
    fn index_values_round_trip_test() -> testresult::TestResult {
        let zone = TimelineZone::new(Some("Europe/Paris"))?;
        let date = TimelineDate::from_creation_time(&zone, utc("2024-07-01T10:00:00.5Z")).unwrap();
        let parsed = TimelineDate::parse(&date.text(), date.source.as_str(), date.instant_nanos())?;
        assert_eq!(parsed, date);
        assert!(TimelineZone::new(Some("Not/AZone")).is_err());
        Ok(())
    }
}
