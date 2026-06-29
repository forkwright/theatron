//! Operator-tier settings store backed by a TOML file with atomic
//! writes.
//!
//! [`Settings::open`] resolves the per-user config dir
//! (`~/.config/<app>/` on Linux, equivalent on macOS / Windows via the
//! [`dirs`] crate), creates `<app>/` if missing, and points at
//! `settings.toml` inside.
//!
//! Writes go through [`tempfile::NamedTempFile`] in the same
//! directory then [`persist`] (rename) onto the target path so a
//! crash mid-write cannot leave a half-flushed `settings.toml`.
//!
//! Reads parse the entire TOML document, deserialize the requested
//! key, and discard the rest. This is fine for an operator-tier
//! KV store (small documents, infrequent reads).
//!
//! [`dirs`]: https://docs.rs/dirs
//! [`persist`]: tempfile::NamedTempFile::persist

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use serde::{Serialize, de::DeserializeOwned};
use snafu::{OptionExt, ResultExt, Snafu};

/// Errors from the settings subsystem.
// kanon:ignore RUST/no-debug-derive-on-public-types -- variants carry filesystem paths and io::Error; no PII, credentials, or secret material.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
pub enum SettingsError {
    /// The platform user-config-dir lookup failed (no `$HOME`, no
    /// `%APPDATA%`, etc.).
    #[snafu(display("could not determine user config directory"))]
    NoConfigDir,

    /// Failed to create the `<app>/` directory inside the user
    /// config dir.
    #[snafu(display("failed to create config directory {}: {source}", path.display()))]
    CreateDir {
        /// Path that failed to create.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// Failed to read the settings file.
    #[snafu(display("failed to read settings file {}: {source}", path.display()))]
    ReadFile {
        /// Path that failed to read.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// Failed to write the settings file (tempfile creation, rename,
    /// or fsync).
    #[snafu(display("failed to write settings file {}: {source}", path.display()))]
    WriteFile {
        /// Path that failed to write.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// Failed to atomically promote the tempfile to the target path.
    #[snafu(display("failed to persist tempfile to {}: {source}", path.display()))]
    PersistFile {
        /// Target path.
        path: PathBuf,
        /// Underlying persist error.
        source: tempfile::PersistError,
    },

    /// Failed to parse on-disk TOML.
    #[snafu(display("failed to parse settings TOML: {source}"))]
    ParseToml {
        /// Underlying TOML deserialize error.
        source: toml::de::Error,
    },

    /// Failed to serialize a value for storage.
    #[snafu(display("failed to serialize settings TOML: {source}"))]
    SerializeToml {
        /// Underlying TOML serialize error.
        source: toml::ser::Error,
    },

    /// Value at the requested key did not match the requested type.
    #[snafu(display("failed to deserialize settings value at key '{lookup_key}': {source}"))]
    DeserializeValue {
        /// Settings key that failed to deserialize.
        lookup_key: String, // kanon:ignore RUST/plain-string-secret -- settings KV key name, not credential material
        /// Underlying deserialize error.
        source: toml::de::Error,
    },
}

impl SettingsError {
    /// Return the filesystem path embedded in this error, if the
    /// variant carries one.
    ///
    /// Returns `Some(&Path)` for filesystem-touching variants
    /// ([`Self::CreateDir`], [`Self::ReadFile`], [`Self::WriteFile`],
    /// [`Self::PersistFile`]) and `None` for the rest. Useful for
    /// consumer code that wants to log the affected path without
    /// destructuring per variant.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::CreateDir { path, .. }
            | Self::ReadFile { path, .. }
            | Self::WriteFile { path, .. }
            | Self::PersistFile { path, .. } => Some(path),
            Self::NoConfigDir
            | Self::ParseToml { .. }
            | Self::SerializeToml { .. }
            | Self::DeserializeValue { .. } => None,
        }
    }

    /// Return the settings key embedded in this error, if the
    /// variant carries one.
    ///
    /// Returns `Some(&str)` only for [`Self::DeserializeValue`]
    /// (the only variant that knows which key was being read).
    /// Useful for consumer code surfacing "couldn't read setting
    /// 'theme'" diagnostics.
    #[must_use]
    pub fn lookup_key(&self) -> Option<&str> {
        match self {
            Self::DeserializeValue { lookup_key, .. } => Some(lookup_key),
            _ => None,
        }
    }
}

/// Operator-tier KV store. One instance per app; cloned handles and
/// separate handles for the same path coordinate writes through a
/// per-file advisory lock.
///
/// [`get`]: Settings::get
/// [`set`]: Settings::set
#[derive(Debug, Clone)]
pub struct Settings {
    file: PathBuf,
}

struct WriteLock {
    _file: std::fs::File,
}

impl Settings {
    /// Open (or create) the settings store for `app_name`. Resolves
    /// the user config dir via [`dirs::config_dir`], creates
    /// `<config>/<app_name>/` if missing, and points at
    /// `settings.toml` inside.
    ///
    /// # Errors
    ///
    /// - [`SettingsError::NoConfigDir`] if the platform doesn't
    ///   expose a user config dir.
    /// - [`SettingsError::CreateDir`] if the directory can't be
    ///   created.
    pub fn open(app_name: &str) -> Result<Self, SettingsError> {
        let base = dirs::config_dir().context(NoConfigDirSnafu)?;
        let app_dir = base.join(app_name);
        Self::open_at(app_dir)
    }

    /// Open (or create) a settings store rooted at an explicit
    /// directory. Used by tests with [`tempfile::tempdir`] to avoid
    /// touching the real user config dir.
    ///
    /// # Errors
    ///
    /// [`SettingsError::CreateDir`] if `dir` does not exist and
    /// can't be created.
    pub fn open_at(dir: impl AsRef<Path>) -> Result<Self, SettingsError> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir).context(CreateDirSnafu { path: dir })?;
        Ok(Self {
            file: dir.join("settings.toml"),
        })
    }

    /// Path to the on-disk settings file. Stable for the lifetime of
    /// `self`; useful for diagnostics and tests.
    #[must_use]
    pub fn file(&self) -> &Path {
        &self.file
    }

    fn lock_file_path(&self) -> PathBuf {
        let parent = self.file.parent().unwrap_or_else(|| Path::new("."));
        let mut file_name = OsString::from(".");
        file_name.push(
            self.file
                .file_name()
                .unwrap_or_else(|| OsStr::new("settings.toml")),
        );
        file_name.push(".lock");
        parent.join(file_name)
    }

    fn acquire_write_lock(&self) -> Result<WriteLock, SettingsError> {
        let lock_path = self.lock_file_path();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .context(WriteFileSnafu {
                path: lock_path.clone(),
            })?;
        lock_file_exclusive(&file).context(WriteFileSnafu { path: lock_path })?;
        Ok(WriteLock { _file: file })
    }

    fn read_doc(&self) -> Result<toml::Table, SettingsError> {
        let text = match std::fs::read_to_string(&self.file) {
            Ok(text) => text,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Ok(toml::Table::new());
            }
            Err(source) => {
                return Err(source).context(ReadFileSnafu {
                    path: self.file.clone(),
                });
            }
        };
        if text.trim().is_empty() {
            return Ok(toml::Table::new());
        }
        toml::from_str::<toml::Table>(&text).context(ParseTomlSnafu)
    }

    fn write_doc(&self, doc: &toml::Table) -> Result<(), SettingsError> {
        let text = toml::to_string_pretty(doc).context(SerializeTomlSnafu)?;
        let parent = self.file.parent().unwrap_or_else(|| Path::new("."));
        let mut tmp = tempfile::NamedTempFile::new_in(parent).context(WriteFileSnafu {
            path: self.file.clone(),
        })?;
        std::io::Write::write_all(&mut tmp, text.as_bytes()).context(WriteFileSnafu {
            path: self.file.clone(),
        })?;
        // fsync the tempfile contents before rename so a power loss
        // between rename and write-back can't surface a truncated
        // file. as_file() exposes the underlying File for sync.
        tmp.as_file().sync_all().context(WriteFileSnafu {
            path: self.file.clone(),
        })?;
        tmp.persist(&self.file).context(PersistFileSnafu {
            path: self.file.clone(),
        })?;
        Ok(())
    }

    /// Read a value at `key`. Returns `Ok(None)` if the key is
    /// absent.
    ///
    /// # Errors
    ///
    /// [`SettingsError::ReadFile`], [`SettingsError::ParseToml`],
    /// [`SettingsError::DeserializeValue`].
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, SettingsError> {
        let doc = self.read_doc()?;
        let Some(value) = doc.get(key) else {
            return Ok(None);
        };
        let parsed =
            T::deserialize(value.clone()).context(DeserializeValueSnafu { lookup_key: key })?;
        Ok(Some(parsed))
    }

    /// Whether `key` is present in the settings file.
    ///
    /// Cheaper than [`get`](Self::get) when the consumer only
    /// needs to know presence (e.g. "has the user set a value
    /// yet?"). Skips the [`DeserializeOwned`] cost of [`get`](Self::get);
    /// reports presence regardless of the value's TOML type.
    ///
    /// # Errors
    ///
    /// [`SettingsError::ReadFile`], [`SettingsError::ParseToml`].
    /// Cannot return [`SettingsError::DeserializeValue`] since no
    /// value-deserialization happens.
    pub fn contains(&self, key: &str) -> Result<bool, SettingsError> {
        let doc = self.read_doc()?;
        Ok(doc.get(key).is_some())
    }

    /// Every top-level key currently present in the settings file,
    /// in TOML document order.
    ///
    /// Useful for migration code (enumerate everything stored, drop
    /// or rename keys whose schema changed), debug UIs (show "what
    /// is in my settings file?"), and consumer-side config
    /// validation (warn about unrecognised keys).
    ///
    /// Returns an empty vector when the settings file is missing or
    /// empty — symmetric with [`get`](Self::get) returning `None`
    /// in those cases.
    ///
    /// Only enumerates **top-level** keys; nested table values
    /// (e.g. `[ui]` sections) appear as a single key whose value is
    /// the table. Consumers wanting nested enumeration should call
    /// [`get`](Self::get) on the top-level key and recurse on the
    /// returned structure.
    ///
    /// # Errors
    ///
    /// [`SettingsError::ReadFile`], [`SettingsError::ParseToml`].
    /// Cannot return [`SettingsError::DeserializeValue`] since no
    /// value-deserialization happens.
    pub fn keys(&self) -> Result<Vec<String>, SettingsError> {
        let doc = self.read_doc()?;
        Ok(doc.keys().cloned().collect())
    }

    /// Write `value` at `key`. Atomic via tempfile + rename; a
    /// crash mid-write leaves the previous on-disk state intact.
    ///
    /// # Errors
    ///
    /// [`SettingsError::ReadFile`], [`SettingsError::WriteFile`],
    /// [`SettingsError::PersistFile`], [`SettingsError::SerializeToml`].
    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<(), SettingsError> {
        let _lock = self.acquire_write_lock()?;
        let mut doc = self.read_doc()?;
        let serialized = toml::Value::try_from(value).context(SerializeTomlSnafu)?;
        doc.insert(key.to_string(), serialized);
        self.write_doc(&doc)?;
        Ok(())
    }

    /// Remove `key` from the settings file.
    ///
    /// Returns `Ok(true)` if the key was present and removed,
    /// `Ok(false)` if the key was already absent. Idempotent:
    /// removing an absent key is not an error.
    ///
    /// Atomic via tempfile + rename, like [`set`](Self::set); a
    /// crash mid-write leaves the previous on-disk state intact.
    /// Skips the write entirely when the key is absent (no I/O
    /// cost, no settings-file mtime bump).
    ///
    /// Symmetric with [`set`](Self::set) and rounds out the CRUD
    /// surface alongside [`get`](Self::get) / [`contains`](Self::contains)
    /// / [`keys`](Self::keys).
    ///
    /// # Errors
    ///
    /// [`SettingsError::ReadFile`], [`SettingsError::ParseToml`],
    /// [`SettingsError::WriteFile`], [`SettingsError::PersistFile`],
    /// [`SettingsError::SerializeToml`].
    pub fn remove(&self, key: &str) -> Result<bool, SettingsError> {
        let _lock = self.acquire_write_lock()?;
        let mut doc = self.read_doc()?;
        if doc.remove(key).is_none() {
            return Ok(false);
        }
        self.write_doc(&doc)?;
        Ok(true)
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;

#[cfg(unix)]
fn lock_file_exclusive(file: &std::fs::File) -> std::io::Result<()> {
    use std::os::{fd::AsRawFd, raw::c_int};

    const LOCK_EX: c_int = 2;

    unsafe extern "C" {
        fn flock(fd: c_int, operation: c_int) -> c_int;
    }

    loop {
        // SAFETY: `file.as_raw_fd()` is a valid open file descriptor for the
        // duration of this call, and `LOCK_EX` is the documented flock
        // operation for a blocking exclusive advisory lock.
        if unsafe { flock(file.as_raw_fd(), LOCK_EX) } == 0 {
            return Ok(());
        }
        let source = std::io::Error::last_os_error();
        if source.kind() != std::io::ErrorKind::Interrupted {
            return Err(source);
        }
    }
}

#[cfg(windows)]
fn lock_file_exclusive(file: &std::fs::File) -> std::io::Result<()> {
    use std::{ffi::c_void, os::windows::io::AsRawHandle};

    type Bool = i32;
    type Dword = u32;
    type Handle = *mut c_void;

    const LOCKFILE_EXCLUSIVE_LOCK: Dword = 0x0000_0002;

    #[repr(C)]
    struct Overlapped {
        internal: usize,
        internal_high: usize,
        offset: Dword,
        offset_high: Dword,
        h_event: Handle,
    }

    unsafe extern "system" {
        fn LockFileEx(
            h_file: Handle,
            dw_flags: Dword,
            dw_reserved: Dword,
            n_number_of_bytes_to_lock_low: Dword,
            n_number_of_bytes_to_lock_high: Dword,
            lp_overlapped: *mut Overlapped,
        ) -> Bool;
    }

    let mut overlapped = Overlapped {
        internal: 0,
        internal_high: 0,
        offset: 0,
        offset_high: 0,
        h_event: std::ptr::null_mut(),
    };
    // SAFETY: `file.as_raw_handle()` is valid while `file` is open, the
    // OVERLAPPED value points to initialized stack storage for the duration of
    // the blocking call, and the lock range covers the whole lock file.
    let ok = unsafe {
        LockFileEx(
            file.as_raw_handle().cast::<c_void>(),
            LOCKFILE_EXCLUSIVE_LOCK,
            0,
            Dword::MAX,
            Dword::MAX,
            &mut overlapped,
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn lock_file_exclusive(_file: &std::fs::File) -> std::io::Result<()> {
    Err(std::io::Error::other(
        "settings write locks are unsupported on this platform",
    ))
}
