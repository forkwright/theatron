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

use std::path::{Path, PathBuf};

use serde::{Serialize, de::DeserializeOwned};
use snafu::{OptionExt, ResultExt, Snafu};

/// Errors from the settings subsystem.
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

    fn read_doc(&self) -> Result<toml::Table, SettingsError> {
        if !self.file.exists() {
            return Ok(toml::Table::new());
        }
        let text = std::fs::read_to_string(&self.file).context(ReadFileSnafu {
            path: self.file.clone(),
        })?;
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

    /// Write `value` at `key`. Atomic via tempfile + rename; a
    /// crash mid-write leaves the previous on-disk state intact.
    ///
    /// # Errors
    ///
    /// [`SettingsError::ReadFile`], [`SettingsError::WriteFile`],
    /// [`SettingsError::PersistFile`], [`SettingsError::SerializeToml`].
    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<(), SettingsError> {
        let mut doc = self.read_doc()?;
        let serialized = toml::Value::try_from(value).context(SerializeTomlSnafu)?;
        doc.insert(key.to_string(), serialized);
        self.write_doc(&doc)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[test]
    fn open_at_creates_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("a").join("b");
        let s = Settings::open_at(&nested).unwrap();
        assert!(nested.exists());
        assert_eq!(s.file(), nested.join("settings.toml"));
    }

    #[test]
    fn round_trip_string() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Settings::open_at(tmp.path()).unwrap();
        s.set("greeting", &"hello".to_string()).unwrap();
        let got: Option<String> = s.get("greeting").unwrap();
        assert_eq!(got, Some("hello".to_string()));
    }

    #[test]
    fn missing_key_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Settings::open_at(tmp.path()).unwrap();
        let got: Option<String> = s.get("nope").unwrap();
        assert_eq!(got, None);
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct WindowState {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        maximized: bool,
    }

    #[test]
    fn round_trip_struct() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Settings::open_at(tmp.path()).unwrap();
        let want = WindowState {
            x: 100,
            y: 200,
            width: 1280,
            height: 720,
            maximized: false,
        };
        s.set("window", &want).unwrap();
        let got: WindowState = s.get("window").unwrap().unwrap();
        assert_eq!(got, want);
    }

    #[test]
    fn second_write_overrides_first() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Settings::open_at(tmp.path()).unwrap();
        s.set("k", &"A".to_string()).unwrap();
        s.set("k", &"B".to_string()).unwrap();
        let got: Option<String> = s.get("k").unwrap();
        assert_eq!(got, Some("B".to_string()));
    }

    #[test]
    fn multiple_keys_coexist() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Settings::open_at(tmp.path()).unwrap();
        s.set("a", &1_i64).unwrap();
        s.set("b", &"two".to_string()).unwrap();
        s.set("c", &true).unwrap();
        assert_eq!(s.get::<i64>("a").unwrap(), Some(1));
        assert_eq!(s.get::<String>("b").unwrap(), Some("two".to_string()));
        assert_eq!(s.get::<bool>("c").unwrap(), Some(true));
    }

    #[test]
    fn atomic_write_no_partial_file() {
        // After a successful set(), the file must contain a fully-
        // parseable TOML document — never a half-written truncate.
        // We can't simulate a crash mid-write, but we can verify the
        // post-condition: every set() leaves a parseable file.
        let tmp = tempfile::tempdir().unwrap();
        let s = Settings::open_at(tmp.path()).unwrap();
        for i in 0..16 {
            s.set("counter", &i).unwrap();
            let raw = std::fs::read_to_string(s.file()).unwrap();
            // Must parse cleanly after every write.
            let _: toml::Table = toml::from_str(&raw).unwrap();
        }
        assert_eq!(s.get::<i64>("counter").unwrap(), Some(15));
    }

    #[test]
    fn persists_across_settings_handles() {
        let tmp = tempfile::tempdir().unwrap();
        {
            let s = Settings::open_at(tmp.path()).unwrap();
            s.set("k", &"v".to_string()).unwrap();
        }
        let s2 = Settings::open_at(tmp.path()).unwrap();
        assert_eq!(s2.get::<String>("k").unwrap(), Some("v".to_string()));
    }
}
