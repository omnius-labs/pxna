use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// 利用者が Entry に付ける評価。評価のない状態は、この型の `Option` の `None` で表す。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Rating {
    Good,
    Bad,
}

impl Rating {
    /// 保存する文字列。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Good => "good",
            Self::Bad => "bad",
        }
    }

    /// 保存した文字列から読む。
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "good" => Ok(Self::Good),
            "bad" => Ok(Self::Bad),
            other => Err(Error::new(ErrorKind::InvalidFormat).with_message(format!("unknown rating: {other}"))),
        }
    }
}

/// 利用者が Entry に付けるタグの語。
///
/// 前後の空白を除いた 1 文字以上 64 文字以下で、制御文字を含まない。大文字と小文字は区別する。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TagName(String);

impl TagName {
    pub const MAX_CHARS: usize = 64;

    pub fn parse(value: &str) -> Result<Self> {
        let value = value.trim();
        if value.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("a tag must not be empty"));
        }
        if value.chars().count() > Self::MAX_CHARS {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("a tag must be at most {} characters", Self::MAX_CHARS)));
        }
        if value.chars().any(char::is_control) {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("a tag must not contain control characters"));
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    #[test]
    fn a_rating_round_trips_through_its_stored_text_test() -> TestResult {
        for rating in [Rating::Good, Rating::Bad] {
            assert_eq!(Rating::parse(rating.as_str())?, rating);
        }
        assert_eq!(*Rating::parse("great").unwrap_err().kind(), ErrorKind::InvalidFormat);
        Ok(())
    }

    #[test]
    fn a_tag_is_trimmed_and_checked_test() -> TestResult {
        assert_eq!(TagName::parse("  旅行 2026 ")?.as_str(), "旅行 2026");
        assert_ne!(TagName::parse("Cat")?, TagName::parse("cat")?);
        for invalid in ["", "   ", "a\nb", "a\u{0}b"] {
            assert_eq!(*TagName::parse(invalid).unwrap_err().kind(), ErrorKind::InvalidInput, "{invalid:?}");
        }
        assert!(TagName::parse(&"あ".repeat(TagName::MAX_CHARS)).is_ok());
        assert_eq!(*TagName::parse(&"あ".repeat(TagName::MAX_CHARS + 1)).unwrap_err().kind(), ErrorKind::InvalidInput);
        Ok(())
    }
}
