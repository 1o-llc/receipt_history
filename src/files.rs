// Copyright © 2026 1o. All rights reserved.

//! Files written whole: to a sibling temporary file, then a rename, so a
//! reader never sees half a file.

use crate::refusal::{Refusal, Result};
use std::path::Path;

fn write_err(path: &Path, e: impl std::fmt::Display) -> crate::Error {
    Refusal::Write.refuse(format!("{}: {e}", path.display()))
}

/// Writes `bytes` to `path` whole: to a sibling temporary file, then a rename,
/// so a reader never sees half a file.
pub fn write_whole(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| write_err(dir, e))?;
    }
    let tmp = path.with_file_name(format!(".{}.{}.tmp", path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), std::process::id()));
    std::fs::write(&tmp, bytes).map_err(|e| write_err(&tmp, e))?;
    std::fs::rename(&tmp, path).map_err(|e| write_err(path, e))
}
