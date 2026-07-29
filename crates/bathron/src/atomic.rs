//! Durable whole-file replacement.
//!
//! [`write_atomic`] replaces a file's contents so that a concurrent reader
//! sees either the old file or the complete new one, and so that the
//! replacement survives a power loss rather than only a process crash.
//!
//! The sequence is tempfile in the target's own directory → `write_all` →
//! `sync_all` on the tempfile → `persist` (rename) → `fsync` of the parent
//! directory. The first three cover the new file's contents; the last covers
//! the rename, which is a directory-metadata operation and is not carried by
//! the tempfile's own fsync.
//!
//! Every step matters for a different failure, and stopping early is not a
//! weaker version of the same guarantee — it is a different one. Omitting the
//! tempfile leaves a torn file after a crash. Omitting the directory fsync
//! leaves the old contents after a power loss, with no error at the time and
//! nothing in a log to attribute it to.

use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use snafu::{ResultExt, Snafu};

/// Errors from [`write_atomic`].
// kanon:ignore RUST/no-debug-derive-on-public-types -- variants carry filesystem paths and io::Error; no PII, credentials, or secret material.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
pub enum AtomicWriteError {
    /// Creating, writing, or fsyncing the temporary file failed.
    #[snafu(display("failed to write {}: {source}", path.display()))]
    Write {
        /// Target path the write was destined for.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// Applying the requested unix permission mode failed.
    #[snafu(display("failed to set mode on the replacement for {}: {source}", path.display()))]
    SetMode {
        /// Target path the write was destined for.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// Renaming the temporary file onto the target failed.
    ///
    /// WHY the source is `tempfile`'s own error rather than [`std::io::Error`]:
    /// `settings::SettingsError::PersistFile` already exposes it, and narrowing
    /// it here would either change that variant's public type or force a
    /// persist failure to surface as a different variant.
    #[snafu(display("failed to persist {}: {source}", path.display()))]
    Persist {
        /// Target path that could not be replaced.
        path: PathBuf,
        /// Underlying rename error, carrying the temporary file back.
        source: tempfile::PersistError,
    },

    /// Flushing the containing directory after the rename failed.
    #[snafu(display("failed to fsync the directory holding {}: {source}", path.display()))]
    SyncDir {
        /// Target path whose parent could not be flushed.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },
}

/// Replace `path`'s contents with `bytes`, durably.
///
/// `mode` is a unix permission mode applied to the replacement *before* the
/// rename, so the target is never briefly visible at the default mode. It is
/// ignored on non-unix platforms.
///
/// The temporary file is created in `path`'s own directory, because a rename
/// is only atomic within one filesystem. The directory must already exist;
/// this function does not create it, and does not set its mode — a caller
/// writing secret material owns that decision and the mode its parent needs.
///
/// # Errors
///
/// [`AtomicWriteError::Write`] if the temporary file cannot be created,
/// written, or flushed; [`AtomicWriteError::SetMode`] if `mode` cannot be
/// applied; [`AtomicWriteError::Persist`] if the rename fails;
/// [`AtomicWriteError::SyncDir`] if the containing directory cannot be
/// flushed afterwards.
pub fn write_atomic(path: &Path, bytes: &[u8], mode: Option<u32>) -> Result<(), AtomicWriteError> {
    let parent = target_dir(path);

    let mut tmp = tempfile::NamedTempFile::new_in(parent).context(WriteSnafu { path })?;
    tmp.write_all(bytes).context(WriteSnafu { path })?;
    tmp.as_file().sync_all().context(WriteSnafu { path })?;

    apply_mode(tmp.as_file(), mode).context(SetModeSnafu { path })?;

    tmp.persist(path).context(PersistSnafu { path })?;

    sync_parent_dir(parent).context(SyncDirSnafu { path })?;

    Ok(())
}

/// The directory the temporary file must be created in for the rename to
/// stay within one filesystem.
///
/// WARNING: `Path::parent` of a bare filename is `Some("")`, not `None`, and
/// an empty path is not a directory anything can be created in. Both that
/// case and the `None` case resolve to the current directory.
fn target_dir(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

/// Apply a unix permission mode to the not-yet-renamed replacement.
#[cfg(unix)]
fn apply_mode(file: &File, mode: Option<u32>) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;

    let Some(mode) = mode else {
        return Ok(());
    };
    file.set_permissions(std::fs::Permissions::from_mode(mode))
}

/// No-op: unix permission modes have no meaning on this platform.
#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "signature must match the unix arm, which can fail"
)]
fn apply_mode(_file: &File, _mode: Option<u32>) -> std::io::Result<()> {
    Ok(())
}

/// Flush the directory entry the rename created.
///
/// WHY: `sync_all` on the tempfile flushes the new contents; the rename that
/// makes them reachable is a change to the *directory*, and is durable only
/// once the directory itself is flushed. Without this, a power loss shortly
/// after the rename can leave the old file in place — a write that silently
/// did not happen, rather than a corrupted one.
#[cfg(unix)]
fn sync_parent_dir(parent: &Path) -> std::io::Result<()> {
    File::open(parent).and_then(|dir| dir.sync_all())
}

/// No-op: Windows cannot open a directory as a file, so there is nothing to
/// flush and the rename's durability is the filesystem's business.
#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "signature must match the unix arm, which can fail"
)]
fn sync_parent_dir(_parent: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
#[path = "atomic_tests.rs"]
mod tests;
