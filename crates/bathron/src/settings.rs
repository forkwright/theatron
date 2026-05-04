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

    /// Whether `key` is present in the settings file.
    ///
    /// Cheaper than [`get`](Self::get) when the consumer only
    /// needs to know presence (e.g. "has the user set a value
    /// yet?"). Skips the [`DeserializeOwned`] cost of [`get`];
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
        let mut doc = self.read_doc()?;
        let serialized = toml::Value::try_from(value).context(SerializeTomlSnafu)?;
        doc.insert(key.to_string(), serialized);
        self.write_doc(&doc)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

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

    #[test]
    fn open_at_resolves_file_to_settings_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Settings::open_at(tmp.path()).unwrap();
        assert_eq!(s.file(), tmp.path().join("settings.toml"));
    }

    #[test]
    fn get_returns_none_when_file_is_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Settings::open_at(tmp.path()).unwrap();
        let got: Option<String> = s.get("any_key").unwrap();
        assert_eq!(got, None);
    }

    #[test]
    fn get_returns_none_when_file_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Settings::open_at(tmp.path()).unwrap();
        std::fs::write(s.file(), "").unwrap();
        let got: Option<String> = s.get("any_key").unwrap();
        assert_eq!(got, None);
    }

    #[test]
    fn round_trip_array_of_strings() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Settings::open_at(tmp.path()).unwrap();
        let want = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];
        s.set("tags", &want).unwrap();
        let got: Vec<String> = s.get("tags").unwrap().unwrap();
        assert_eq!(got, want);
    }

    #[test]
    fn round_trip_float() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Settings::open_at(tmp.path()).unwrap();
        s.set("factor", &2.5_f64).unwrap();
        let got: f64 = s.get("factor").unwrap().unwrap();
        assert!((got - 2.5).abs() < f64::EPSILON);
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Inner {
        value: i64,
        flag: bool,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Outer {
        name: String,
        inner: Inner,
    }

    #[test]
    fn round_trip_nested_table() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Settings::open_at(tmp.path()).unwrap();
        let want = Outer {
            name: "example".to_string(),
            inner: Inner {
                value: 42,
                flag: true,
            },
        };
        s.set("config", &want).unwrap();
        let got: Outer = s.get("config").unwrap().unwrap();
        assert_eq!(got, want);
    }

    #[test]
    fn set_is_idempotent_for_same_value() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Settings::open_at(tmp.path()).unwrap();
        s.set("k", &"v".to_string()).unwrap();
        let first = std::fs::read_to_string(s.file()).unwrap();
        s.set("k", &"v".to_string()).unwrap();
        let second = std::fs::read_to_string(s.file()).unwrap();
        assert_eq!(first, second);
        assert_eq!(s.get::<String>("k").unwrap(), Some("v".to_string()));
    }

    #[test]
    fn deserialize_value_fails_when_type_mismatches() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Settings::open_at(tmp.path()).unwrap();
        s.set("key", &"not_a_number".to_string()).unwrap();
        let result: Result<Option<i64>, SettingsError> = s.get("key");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failed to deserialize settings value at key 'key'"));
    }

    #[test]
    fn parse_toml_fails_when_document_is_malformed() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Settings::open_at(tmp.path()).unwrap();
        std::fs::write(s.file(), "this is not valid toml [[[").unwrap();
        let result: Result<Option<String>, SettingsError> = s.get("key");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failed to parse settings TOML"));
    }

    #[test]
    fn read_file_fails_when_path_is_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let s = Settings::open_at(tmp.path()).unwrap();
        std::fs::create_dir(s.file()).unwrap();
        let result: Result<Option<String>, SettingsError> = s.get("key");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failed to read settings file"));
    }

    #[test]
    fn create_dir_fails_when_final_path_component_is_file() {
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("a_file");
        std::fs::write(&file_path, "x").unwrap();
        let nested = file_path.join("sub");
        let result = Settings::open_at(&nested);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failed to create config directory"));
    }

    #[test]
    fn write_fails_when_app_dir_is_removed() {
        let tmp = tempfile::tempdir().unwrap();
        let app_dir = tmp.path().join("app");
        let s = Settings::open_at(&app_dir).unwrap();
        std::fs::remove_dir_all(&app_dir).unwrap();
        let result = s.set("k", &"v".to_string());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failed to write settings file"));
    }

    #[test]
    fn cascade_falls_back_through_manual_layering() {
        let system_tmp = tempfile::tempdir().unwrap();
        let user_tmp = tempfile::tempdir().unwrap();
        let app_tmp = tempfile::tempdir().unwrap();

        let system = Settings::open_at(system_tmp.path()).unwrap();
        let user = Settings::open_at(user_tmp.path()).unwrap();
        let app = Settings::open_at(app_tmp.path()).unwrap();

        system.set("theme", &"system_default".to_string()).unwrap();
        user.set("theme", &"user_override".to_string()).unwrap();
        user.set("font_size", &12_i64).unwrap();
        app.set("font_size", &14_i64).unwrap();

        let theme = app
            .get::<String>("theme")
            .unwrap()
            .or_else(|| user.get::<String>("theme").unwrap())
            .or_else(|| system.get::<String>("theme").unwrap());
        assert_eq!(theme, Some("user_override".to_string()));

        let font_size = app
            .get::<i64>("font_size")
            .unwrap()
            .or_else(|| user.get::<i64>("font_size").unwrap())
            .or_else(|| system.get::<i64>("font_size").unwrap());
        assert_eq!(font_size, Some(14));

        let missing = app
            .get::<String>("missing")
            .unwrap()
            .or_else(|| user.get::<String>("missing").unwrap())
            .or_else(|| system.get::<String>("missing").unwrap());
        assert_eq!(missing, None);
    }

    #[test]
    fn contains_returns_true_for_existing_key() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = Settings::open_at(tmp.path()).unwrap();
        settings.set("theme", &"dark").unwrap();
        assert!(settings.contains("theme").unwrap());
    }

    #[test]
    fn contains_returns_false_for_missing_key() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = Settings::open_at(tmp.path()).unwrap();
        assert!(!settings.contains("nonexistent").unwrap());
    }

    #[test]
    fn contains_returns_false_when_file_is_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = Settings::open_at(tmp.path()).unwrap();
        // No set() call — file doesn't exist on disk yet.
        assert!(!settings.contains("any_key").unwrap());
    }

    #[test]
    fn contains_succeeds_regardless_of_value_type() {
        // contains() doesn't deserialize, so type coercion errors
        // that would surface in get::<T>() don't surface here.
        let tmp = tempfile::tempdir().unwrap();
        let settings = Settings::open_at(tmp.path()).unwrap();
        settings.set("count", &42i64).unwrap();
        // get::<String>("count") would fail with DeserializeValue;
        // contains("count") just reports presence.
        assert!(settings.contains("count").unwrap());
        assert!(settings.get::<String>("count").is_err());
    }

    #[test]
    fn contains_returns_true_after_idempotent_set() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = Settings::open_at(tmp.path()).unwrap();
        settings.set("theme", &"dark").unwrap();
        settings.set("theme", &"dark").unwrap(); // re-set same value
        assert!(settings.contains("theme").unwrap());
    }

    #[test]
    fn keys_returns_empty_when_file_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = Settings::open_at(tmp.path()).unwrap();
        let keys = settings.keys().unwrap();
        assert!(keys.is_empty(), "missing file → empty keys, got {keys:?}");
    }

    #[test]
    fn keys_lists_every_top_level_key() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = Settings::open_at(tmp.path()).unwrap();
        settings.set("theme", &"dark").unwrap();
        settings.set("verbose", &true).unwrap();
        settings.set("retries", &3_i64).unwrap();
        let mut keys = settings.keys().unwrap();
        keys.sort();
        assert_eq!(keys, vec!["retries", "theme", "verbose"]);
    }

    #[test]
    fn keys_round_trips_with_contains() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = Settings::open_at(tmp.path()).unwrap();
        settings.set("a", &1_i64).unwrap();
        settings.set("b", &2_i64).unwrap();
        for key in settings.keys().unwrap() {
            assert!(
                settings.contains(&key).unwrap(),
                "every enumerated key should be contained: {key}"
            );
        }
    }

    #[test]
    fn keys_does_not_include_values() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = Settings::open_at(tmp.path()).unwrap();
        settings.set("greeting", &"hello world").unwrap();
        let keys = settings.keys().unwrap();
        assert_eq!(keys, vec!["greeting"]);
        // The value "hello world" must not leak into the keys list.
        assert!(!keys.iter().any(|k| k.contains("hello")));
    }

    #[test]
    fn error_path_returns_some_for_filesystem_variants() {
        let p = PathBuf::from("/tmp/some/path");
        let create_dir = SettingsError::CreateDir {
            path: p.clone(),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "x"),
        };
        let read_file = SettingsError::ReadFile {
            path: p.clone(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "x"),
        };
        let write_file = SettingsError::WriteFile {
            path: p.clone(),
            source: std::io::Error::other("x"),
        };
        assert_eq!(create_dir.path(), Some(p.as_path()));
        assert_eq!(read_file.path(), Some(p.as_path()));
        assert_eq!(write_file.path(), Some(p.as_path()));
    }

    #[test]
    fn error_path_returns_none_for_non_filesystem_variants() {
        let parse_toml = SettingsError::ParseToml {
            source: toml::from_str::<toml::Value>("not = =")
                .expect_err("malformed TOML must error"),
        };
        assert_eq!(SettingsError::NoConfigDir.path(), None);
        assert_eq!(parse_toml.path(), None);
    }

    #[test]
    fn error_lookup_key_returns_some_only_for_deserialize_value() {
        let dv = SettingsError::DeserializeValue {
            lookup_key: "theme".to_string(),
            source: toml::from_str::<i64>("not_a_number").err().unwrap(),
        };
        assert_eq!(dv.lookup_key(), Some("theme"));
        assert_eq!(SettingsError::NoConfigDir.lookup_key(), None);

        let read_file = SettingsError::ReadFile {
            path: PathBuf::from("/x"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "x"),
        };
        assert_eq!(read_file.lookup_key(), None);
    }
}
