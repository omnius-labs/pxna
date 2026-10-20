use std::path::Path;

use crate::prelude::*;

/// 既存 destination を置換しない rename の結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameOutcome {
    Renamed,
    AlreadyExists,
}

/// destination が既に存在する場合に、それを置換せず `AlreadyExists` を返す rename。
///
/// `Fail`、`Skip`、`Rename` の 3 つの conflict policy は、いずれも「存在しないことを確認してから
/// 公開する」という不可分な操作を必要とする。存在確認と rename を別々に行うと、その間に
/// destination が作られた場合に既存 file を黙って上書きする。
///
/// OS の primitive を使えない環境（古い kernel、no-replace rename に非対応の filesystem）では、
/// 存在確認 + rename へ退避する。この経路でだけ競合の窓が残る。
// omnius-lint:allow(free-fn) rename操作を呼出し側とcfg moduleの間のOS境界として公開する
pub fn rename_no_replace(src: &Path, dst: &Path) -> Result<RenameOutcome> {
    match sys::rename_no_replace(src, dst) {
        Some(r) => r,
        None => rename_no_replace_fallback(src, dst),
    }
}

/// rename が別 filesystem をまたいだために失敗したかどうか。
///
/// 判定できたときだけ copy 経由の move へ退避する。
// omnius-lint:allow(free-fn) OS error codeの判定をrename境界へ閉じる
pub fn is_cross_device(e: &Error) -> bool {
    let Some(source) = std::error::Error::source(e) else {
        return false;
    };
    let Some(io) = source.downcast_ref::<std::io::Error>() else {
        return false;
    };
    io.raw_os_error() == Some(sys::CROSS_DEVICE_ERROR)
}

// omnius-lint:allow(free-fn) OS API非対応時のrename fallbackをplatform境界で共有する
fn rename_no_replace_fallback(src: &Path, dst: &Path) -> Result<RenameOutcome> {
    match std::fs::symlink_metadata(dst) {
        Ok(_) => return Ok(RenameOutcome::AlreadyExists),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    std::fs::rename(src, dst)?;
    Ok(RenameOutcome::Renamed)
}

#[cfg(unix)]
mod sys {
    use std::path::Path;

    use rustix::{
        fs::{CWD, RenameFlags, renameat_with},
        io::Errno,
    };

    use super::RenameOutcome;
    use crate::prelude::*;

    pub const CROSS_DEVICE_ERROR: i32 = Errno::XDEV.raw_os_error();

    /// `None` は「この環境では OS の primitive を使えない」を意味し、呼び出し側が退避する。
    // omnius-lint:allow(free-fn) Unixのrename system callをcfg moduleのOS境界として公開する
    pub fn rename_no_replace(src: &Path, dst: &Path) -> Option<Result<RenameOutcome>> {
        match renameat_with(CWD, src, CWD, dst, RenameFlags::NOREPLACE) {
            Ok(()) => Some(Ok(RenameOutcome::Renamed)),
            Err(Errno::EXIST) => Some(Ok(RenameOutcome::AlreadyExists)),
            // 古い kernel、no-replace rename に非対応の filesystem、古い macOS。
            Err(Errno::NOSYS) | Err(Errno::INVAL) | Err(Errno::NOTSUP) => None,
            Err(e) => Some(Err(Error::from_error(std::io::Error::from(e), ErrorKind::IoError).with_message("rename failed"))),
        }
    }
}

#[cfg(windows)]
mod sys {
    use std::{os::windows::ffi::OsStrExt as _, path::Path};

    use windows_sys::Win32::{
        Foundation::{ERROR_ALREADY_EXISTS, ERROR_FILE_EXISTS, ERROR_NOT_SAME_DEVICE},
        Storage::FileSystem::MoveFileExW,
    };

    use super::RenameOutcome;
    use crate::prelude::*;

    pub const CROSS_DEVICE_ERROR: i32 = ERROR_NOT_SAME_DEVICE as i32;

    // omnius-lint:allow(free-fn) Windows FFIの文字列変換をcfg moduleのOS境界へ閉じる
    fn to_wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
    }

    /// `MoveFileExW` は `MOVEFILE_REPLACE_EXISTING` を渡さなければ既存 destination を置換しない。
    // omnius-lint:allow(free-fn) Windowsのrename APIをcfg moduleのOS境界として公開する
    pub fn rename_no_replace(src: &Path, dst: &Path) -> Option<Result<RenameOutcome>> {
        let src = to_wide(src);
        let dst = to_wide(dst);

        // SAFETY: どちらも NUL 終端された有効な UTF-16 の buffer である。
        let ok = unsafe { MoveFileExW(src.as_ptr(), dst.as_ptr(), 0) };
        if ok != 0 {
            return Some(Ok(RenameOutcome::Renamed));
        }

        let e = std::io::Error::last_os_error();
        match e.raw_os_error().map(|n| n as u32) {
            Some(ERROR_ALREADY_EXISTS) | Some(ERROR_FILE_EXISTS) => Some(Ok(RenameOutcome::AlreadyExists)),
            _ => Some(Err(Error::from_error(e, ErrorKind::IoError).with_message("rename failed"))),
        }
    }
}

#[cfg(not(any(unix, windows)))]
mod sys {
    use std::path::Path;

    use super::RenameOutcome;
    use crate::prelude::*;

    pub const CROSS_DEVICE_ERROR: i32 = -1;

    // omnius-lint:allow(free-fn) 未対応OSのrename fallback契約をcfg moduleの境界として公開する
    pub fn rename_no_replace(_src: &Path, _dst: &Path) -> Option<Result<RenameOutcome>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    #[test]
    fn renames_when_destination_is_absent_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let src = dir.path().join("src");
        let dst = dir.path().join("dst");
        std::fs::write(&src, b"a")?;

        assert_eq!(rename_no_replace(&src, &dst)?, RenameOutcome::Renamed);
        assert!(!src.exists());
        assert_eq!(std::fs::read(&dst)?, b"a");

        Ok(())
    }

    #[test]
    fn reports_existing_destination_without_replacing_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let src = dir.path().join("src");
        let dst = dir.path().join("dst");
        std::fs::write(&src, b"a")?;
        std::fs::write(&dst, b"b")?;

        assert_eq!(rename_no_replace(&src, &dst)?, RenameOutcome::AlreadyExists);
        assert_eq!(std::fs::read(&dst)?, b"b", "既存 destination を置換しない");
        assert!(src.exists(), "source を残す");

        Ok(())
    }

    #[test]
    fn fallback_reports_existing_destination_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let src = dir.path().join("src");
        let dst = dir.path().join("dst");
        std::fs::write(&src, b"a")?;
        std::fs::write(&dst, b"b")?;

        assert_eq!(rename_no_replace_fallback(&src, &dst)?, RenameOutcome::AlreadyExists);
        assert_eq!(std::fs::read(&dst)?, b"b");

        Ok(())
    }
}
