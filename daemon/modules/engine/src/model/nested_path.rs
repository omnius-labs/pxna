use std::{
    borrow::Borrow,
    fmt,
    ops::{Deref, DerefMut},
    path::Path,
    str::FromStr,
};

const DELIMITER: char = '\0';

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NestedPathBuf {
    inner: String,
}

impl NestedPathBuf {
    pub fn new() -> NestedPathBuf {
        NestedPathBuf { inner: String::new() }
    }

    pub fn push<P: AsRef<NestedPath>>(&mut self, path: P) -> Result<(), NestedPathError> {
        if !self.inner.is_empty() && self.inner.ends_with(DELIMITER) {
            return Err(NestedPathError::EmptySegment);
        }

        if !self.inner.is_empty() {
            self.inner.push(DELIMITER);
        }
        self.inner.push_str(path.as_ref().as_str());
        Ok(())
    }

    pub fn push_str(&mut self, path: &str) -> Result<(), NestedPathError> {
        self.push(NestedPath::new(path)?)
    }

    /// 最後の segment が指す archive file の内側へ入り、archive の root を指す path にする。
    pub fn push_archive_root(&mut self) -> Result<(), NestedPathError> {
        self.push_str("")
    }

    pub(crate) fn compose(outer: &[String], driver_path: &str) -> Result<Self, NestedPathError> {
        let mut buf = Self::new();
        for segment in outer {
            buf.push_str(segment)?;
        }
        buf.push_str(driver_path)?;
        Ok(buf)
    }

    pub fn pop(&mut self) -> bool {
        let i = self.inner.rfind(DELIMITER).unwrap_or_default();
        if i == 0 {
            return false;
        }
        self.inner.truncate(i);
        true
    }

    pub fn rank(&self) -> usize {
        self.inner.chars().filter(|&c| c == DELIMITER).count()
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            inner: self.inner.split(DELIMITER),
        }
    }
}

#[repr(transparent)]
#[derive(PartialEq, Eq)]
pub struct NestedPath {
    inner: str,
}

impl NestedPath {
    pub fn new(s: &str) -> Result<&NestedPath, NestedPathError> {
        Self::validate(s)?;
        Ok(Self::from_inner(s))
    }

    pub fn to_nested_path_buf(&self) -> NestedPathBuf {
        let s = self.inner.to_string();
        NestedPathBuf { inner: s }
    }

    pub fn as_str(&self) -> &str {
        &self.inner
    }

    fn from_inner(inner: &str) -> &NestedPath {
        // SAFETY: `NestedPath` is `repr(transparent)` over `str`, so both references have the
        // same layout and lifetime. Callers keep the validated string unchanged.
        unsafe { &*(inner as *const str as *const NestedPath) }
    }

    fn from_inner_mut(inner: &mut str) -> &mut NestedPath {
        // SAFETY: `NestedPath` is `repr(transparent)` over `str`, so both references have the
        // same layout and lifetime. `NestedPath` exposes no operation that can invalidate UTF-8.
        unsafe { &mut *(inner as *mut str as *mut NestedPath) }
    }

    fn validate(s: &str) -> Result<(), NestedPathError> {
        if s.is_empty() {
            return Ok(());
        }

        let mut segments = s.split(DELIMITER).peekable();
        while let Some(segment) = segments.next() {
            if segment.is_empty() && segments.peek().is_some() {
                return Err(NestedPathError::EmptySegment);
            }
        }

        Ok(())
    }

    pub fn rank(&self) -> usize {
        self.inner.chars().filter(|&c| c == DELIMITER).count()
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            inner: self.inner.split(DELIMITER),
        }
    }

    pub(crate) fn outer_segments(&self) -> Vec<String> {
        let mut segments: Vec<String> = self.iter().map(str::to_string).collect();
        segments.pop();
        segments
    }

    pub(crate) fn parent(&self) -> Option<String> {
        self.iter()
            .last()
            .and_then(|segment| Path::new(segment).parent())
            .and_then(|parent| parent.to_str())
            .map(str::to_string)
    }

    pub fn file_name(&self) -> Option<&str> {
        self.iter().last().and_then(|segment| Path::new(segment).file_name().and_then(|name| name.to_str()))
    }

    pub fn file_stem(&self) -> Option<&str> {
        self.iter().last().and_then(|segment| Path::new(segment).file_stem().and_then(|stem| stem.to_str()))
    }

    pub fn extension(&self) -> Option<&str> {
        self.iter().last().and_then(|segment| Path::new(segment).extension().and_then(|ext| ext.to_str()))
    }
}

// ---- Iter ----

pub struct Iter<'a> {
    inner: std::str::Split<'a, char>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl<'a> IntoIterator for &'a NestedPathBuf {
    type Item = &'a str;
    type IntoIter = Iter<'a>;
    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a NestedPath {
    type Item = &'a str;
    type IntoIter = Iter<'a>;
    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

// ---- NestedPathBuf ----

impl TryFrom<&Path> for NestedPathBuf {
    type Error = NestedPathError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let path = path.to_str().ok_or(NestedPathError::NonUtf8)?;
        path.parse()
    }
}

impl FromStr for NestedPathBuf {
    type Err = NestedPathError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        NestedPath::validate(s)?;
        Ok(NestedPathBuf { inner: s.to_string() })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedPathError {
    NonUtf8,
    EmptySegment,
}

impl fmt::Display for NestedPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonUtf8 => f.write_str("path is not valid UTF-8"),
            Self::EmptySegment => f.write_str("nested path contains an empty segment"),
        }
    }
}

impl std::error::Error for NestedPathError {}

impl From<NestedPathBuf> for String {
    fn from(buf: NestedPathBuf) -> String {
        buf.inner
    }
}

impl Deref for NestedPathBuf {
    type Target = NestedPath;
    fn deref(&self) -> &NestedPath {
        NestedPath::from_inner(&self.inner)
    }
}

impl DerefMut for NestedPathBuf {
    fn deref_mut(&mut self) -> &mut NestedPath {
        NestedPath::from_inner_mut(&mut self.inner)
    }
}

impl Borrow<NestedPath> for NestedPathBuf {
    fn borrow(&self) -> &NestedPath {
        self.deref()
    }
}

impl Default for NestedPathBuf {
    fn default() -> Self {
        NestedPathBuf::new()
    }
}

impl fmt::Debug for NestedPathBuf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner)?;
        Ok(())
    }
}

impl fmt::Display for NestedPathBuf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner)?;
        Ok(())
    }
}

// ---- NestedPath ----

impl AsRef<NestedPath> for NestedPath {
    fn as_ref(&self) -> &NestedPath {
        self
    }
}

impl AsRef<NestedPath> for NestedPathBuf {
    fn as_ref(&self) -> &NestedPath {
        self
    }
}

impl fmt::Debug for NestedPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.inner)?;
        Ok(())
    }
}

impl fmt::Display for NestedPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.inner)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use testresult::TestResult;

    use super::*;

    #[tokio::test]
    async fn push_and_pop_test() -> TestResult {
        // push test
        let mut buf = NestedPathBuf::new();
        buf.push_str("aaaa/bbbb")?;
        buf.push_str("1111/2222")?;
        buf.push_str("xxxx/yyyy")?;
        assert_eq!(buf.iter().collect::<Vec<_>>(), vec!["aaaa/bbbb", "1111/2222", "xxxx/yyyy"]);

        // pop test
        assert!(buf.pop());
        assert_eq!(buf.iter().collect::<Vec<_>>(), vec!["aaaa/bbbb", "1111/2222"]);

        assert!(buf.pop());
        assert_eq!(buf.iter().collect::<Vec<_>>(), vec!["aaaa/bbbb"]);

        assert!(!buf.pop());
        assert_eq!(buf.iter().collect::<Vec<_>>(), vec!["aaaa/bbbb"]);

        Ok(())
    }

    #[test]
    fn validates_nested_path_segments_test() {
        assert!("outer\0inner".parse::<NestedPathBuf>().is_ok());
        assert!("outer\0".parse::<NestedPathBuf>().is_ok());
        assert_eq!("\0inner".parse::<NestedPathBuf>().unwrap_err(), NestedPathError::EmptySegment);
        assert_eq!("outer\0\0inner".parse::<NestedPathBuf>().unwrap_err(), NestedPathError::EmptySegment);

        let mut path = "outer\0".parse::<NestedPathBuf>().unwrap();
        assert_eq!(path.push_str("inner").unwrap_err(), NestedPathError::EmptySegment);
        assert_eq!(path.as_str(), "outer\0");
    }

    #[test]
    fn pushes_archive_root_test() -> TestResult {
        let mut path = NestedPathBuf::new();
        path.push_str("outer.7z")?;
        path.push_archive_root()?;
        assert_eq!(path.as_str(), "outer.7z\0");
        assert_eq!(path.rank(), 1);
        assert_eq!(path.push_archive_root().unwrap_err(), NestedPathError::EmptySegment);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn rejects_non_utf8_os_path_test() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        let path = Path::new(OsStr::from_bytes(b"invalid-\xFF"));
        assert_eq!(NestedPathBuf::try_from(path).unwrap_err(), NestedPathError::NonUtf8);
    }
}
