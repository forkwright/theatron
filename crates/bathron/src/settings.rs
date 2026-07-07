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
//! Mutations ([`Settings::set`], [`Settings::remove`]) hold an
//! exclusive advisory lock on a sibling `settings.toml.lock` file for
//! the duration of their read-modify-write cycle, so concurrent
//! writers — other threads or other processes sharing the same path —
//! serialize instead of silently overwriting each other's keys.
//! Reads take no lock: the rename-based write means a reader always
//! sees a complete document.
//!
//! Reads parse the entire TOML document, deserialize the requested
//! key, and discard the rest. This is fine for an operator-tier
//! KV store (small documents, infrequent reads).
//!
//! [`dirs`]: https://docs.rs/dirs
//! [`persist`]: tempfile::NamedTempFile::persist

use std::path::{Path, PathBuf};

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

    /// Failed to open or acquire the advisory write lock guarding
    /// the settings file's read-modify-write cycle.
    #[snafu(display("failed to lock settings file via {}: {source}", path.display()))]
    LockFile {
        /// Lock-file path.
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
    /// [`Self::LockFile`], [`Self::PersistFile`]) and `None` for the
    /// rest. Useful for consumer code that wants to log the affected
    /// path without destructuring per variant.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::CreateDir { path, .. }
            | Self::ReadFile { path, .. }
            | Self::WriteFile { path, .. }
            | Self::LockFile { path, .. }
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

/// Operator-tier KV store. One instance per app; cheap to clone the
/// underlying path if needed (each [`get`]/[`set`] re-reads the file).
///
/// [`get`]: Settings::get
/// [`set`]: Settings::set
#[derive(Debug, Clone)]
pub struct Settings {
    file: PathBuf,
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

    /// Open the sibling lock file that serializes mutations.
    ///
    /// The lock file is separate from `settings.toml` because
    /// [`write_doc`](Self::write_doc) replaces the settings file by
    /// rename — a lock held on the settings file's inode would be
    /// orphaned by the very write it guards, letting the next writer
    /// proceed concurrently. The lock file is never renamed, so its
    /// inode is stable across writes.
    fn write_lock(&self) -> Result<fd_lock::RwLock<std::fs::File>, SettingsError> {
        let path = self.file.with_extension("toml.lock");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .context(LockFileSnafu { path })?;
        Ok(fd_lock::RwLock::new(file))
    }

    fn read_doc(&self) -> Result<toml::Table, SettingsError> {
        let text = match std::fs::read_to_string(&self.file) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(toml::Table::new()),
            Err(e) => {
                return Err(e).context(ReadFileSnafu {
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
    /// The read-modify-write cycle holds an exclusive advisory lock
    /// on a sibling `settings.toml.lock` file, so concurrent writers
    /// (threads or processes) serialize instead of silently losing
    /// each other's keys.
    ///
    /// # Errors
    ///
    /// [`SettingsError::ReadFile`], [`SettingsError::LockFile`],
    /// [`SettingsError::WriteFile`], [`SettingsError::PersistFile`],
    /// [`SettingsError::SerializeToml`].
    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<(), SettingsError> {
        let mut lock = self.write_lock()?;
        let _guard = lock.write().context(LockFileSnafu {
            path: self.file.with_extension("toml.lock"),
        })?;
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
    /// Holds the same exclusive advisory lock as [`set`](Self::set)
    /// across the read-modify-write cycle. Skips the write when the
    /// key is absent (no settings-file mtime bump).
    ///
    /// Symmetric with [`set`](Self::set) and rounds out the CRUD
    /// surface alongside [`get`](Self::get) / [`contains`](Self::contains)
    /// / [`keys`](Self::keys).
    ///
    /// # Errors
    ///
    /// [`SettingsError::ReadFile`], [`SettingsError::ParseToml`],
    /// [`SettingsError::LockFile`], [`SettingsError::WriteFile`],
    /// [`SettingsError::PersistFile`], [`SettingsError::SerializeToml`].
    pub fn remove(&self, key: &str) -> Result<bool, SettingsError> {
        let mut lock = self.write_lock()?;
        let _guard = lock.write().context(LockFileSnafu {
            path: self.file.with_extension("toml.lock"),
        })?;
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
