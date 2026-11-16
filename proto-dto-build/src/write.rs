// SPDX-License-Identifier: AGPL-3.0-or-later

//! Writing generated files without touching unchanged ones.

use std::path::Path;

use crate::Error;

/// Writes `contents` to `path` only when the file is missing or differs.
///
/// Unchanged files keep their mtime, so `flutter_rust_bridge_codegen`, cargo
/// and IDE file watchers are not retriggered by every build. Missing parent
/// directories are created. Returns `true` when the file was written.
///
/// # Errors
///
/// [`Error::Io`] when the directory or the file cannot be written.
pub fn write_if_changed(path: &Path, contents: &str) -> Result<bool, Error> {
    if std::fs::read(path).is_ok_and(|existing| existing == contents.as_bytes()) {
        return Ok(false);
    }
    let io = |source| Error::Io {
        path: path.to_owned(),
        source,
    };
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(io)?;
    }
    std::fs::write(path, contents).map_err(io)?;
    Ok(true)
}
