//! Tracing-subscriber init with daily-rotated file appender.
//!
//! [`init`] wires a [`tracing_subscriber::Registry`] with a
//! [`tracing_appender::rolling::RollingFileAppender`] (daily rotation)
//! plus an env-filter (`RUST_LOG` honored if set, otherwise the
//! configured [`LogConfig::level`]).
//!
//! Returns the [`tracing_appender::non_blocking::WorkerGuard`] —
//! callers MUST hold this for the program lifetime; dropping it
//! flushes the worker thread and stops accepting events.
//!
//! `init` itself is process-global state and not unit-testable.
//! [`LogConfig::resolve_log_dir`] is pure and is covered.

use std::path::PathBuf;

#[cfg(not(test))]
use snafu::ResultExt;
use snafu::{OptionExt, Snafu};

/// Errors from logging init.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
pub enum LoggingError {
    /// The platform user-state-dir lookup failed and no `log_dir`
    /// was provided in [`LogConfig`].
    #[snafu(display("could not determine user state directory for logs"))]
    NoStateDir,

    /// Failed to create the log directory.
    #[snafu(display("failed to create log directory {}: {source}", path.display()))]
    CreateDir {
        /// Path that failed to create.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// Failed to install the global tracing subscriber (typically
    /// because one is already installed).
    #[snafu(display("failed to install tracing subscriber: {source}"))]
    SetGlobalDefault {
        /// Underlying tracing error.
        source: tracing::dispatcher::SetGlobalDefaultError,
    },
}

/// Logging configuration.
///
/// `log_dir` defaults to `<state>/<app_name>/logs/` where `<state>` is
/// [`dirs::state_dir`] on Linux (typically `~/.local/state`) and
/// [`dirs::data_local_dir`] on macOS / Windows. Override by setting
/// [`LogConfig::log_dir`].
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// App name — segments the log directory and prefixes log files.
    pub app_name: String,
    /// Default log level if `RUST_LOG` is unset.
    pub level: tracing::Level,
    /// Optional override for the log directory. If `None`, resolved
    /// via [`LogConfig::resolve_log_dir`].
    pub log_dir: Option<PathBuf>,
}

impl LogConfig {
    /// Construct a config for `app_name` at the given default level.
    /// `log_dir` is left as `None` (auto-resolved at init time).
    #[must_use]
    pub fn new(app_name: impl Into<String>, level: tracing::Level) -> Self {
        Self {
            app_name: app_name.into(),
            level,
            log_dir: None,
        }
    }

    /// Override the log directory.
    #[must_use]
    pub fn with_log_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.log_dir = Some(dir.into());
        self
    }

    /// Resolve the effective log directory. Honors `log_dir` if set;
    /// otherwise picks `<state>/<app_name>/logs/`.
    ///
    /// # Errors
    ///
    /// [`LoggingError::NoStateDir`] if the platform can't expose a
    /// state dir AND the caller didn't provide one.
    pub fn resolve_log_dir(&self) -> Result<PathBuf, LoggingError> {
        if let Some(dir) = &self.log_dir {
            return Ok(dir.clone());
        }
        // dirs::state_dir() is Linux-only (XDG); fall back to
        // data_local_dir() on macOS / Windows so we get a sensible
        // per-user writable path everywhere.
        let base = dirs::state_dir()
            .or_else(dirs::data_local_dir)
            .context(NoStateDirSnafu)?;
        Ok(base.join(&self.app_name).join("logs"))
    }
}

/// Initialize global tracing with file rotation and an env-filter.
///
/// File output only. For also-emit-to-stderr behaviour (typical for a
/// `--verbose` flag, dev runs, or always-loud daemons) call
/// [`init_with_stderr`] instead.
///
/// The returned [`tracing_appender::non_blocking::WorkerGuard`] MUST
/// be held for the program's lifetime — dropping it flushes the
/// background writer and stops accepting log events.
///
/// # Errors
///
/// - [`LoggingError::NoStateDir`] (platform state dir lookup failed
///   and `log_dir` not set).
/// - [`LoggingError::CreateDir`] (couldn't create the log directory).
/// - [`LoggingError::SetGlobalDefault`] (a subscriber is already
///   installed).
#[cfg(not(test))]
pub fn init(
    config: LogConfig,
) -> Result<tracing_appender::non_blocking::WorkerGuard, LoggingError> {
    init_with_stderr(config, false)
}

/// Initialize global tracing with file rotation, env-filter, and an
/// optional stderr layer.
///
/// When `also_to_stderr` is `true`, log events are written to *both*
/// the daily-rotated file *and* stderr. Both layers share the same
/// env-filter (the configured [`LogConfig::level`], or `RUST_LOG` if
/// set in the environment).
///
/// Typical usage: a desktop app reading a `--verbose` CLI flag or
/// the presence of `RUST_LOG` to opt callers into an always-on
/// stderr layer for development. Production deployments leave
/// `also_to_stderr = false` and rely on file output.
///
/// The returned [`tracing_appender::non_blocking::WorkerGuard`] MUST
/// be held for the program's lifetime — dropping it flushes the
/// background writer and stops accepting log events.
///
/// # Errors
///
/// Same set as [`init`]:
///
/// - [`LoggingError::NoStateDir`] (platform state dir lookup failed
///   and `log_dir` not set).
/// - [`LoggingError::CreateDir`] (couldn't create the log directory).
/// - [`LoggingError::SetGlobalDefault`] (a subscriber is already
///   installed).
#[cfg(not(test))]
pub fn init_with_stderr(
    config: LogConfig,
    also_to_stderr: bool,
) -> Result<tracing_appender::non_blocking::WorkerGuard, LoggingError> {
    use tracing_subscriber::{EnvFilter, layer::SubscriberExt};

    let dir = config.resolve_log_dir()?;
    std::fs::create_dir_all(&dir).context(CreateDirSnafu { path: dir.clone() })?;

    let LogConfig {
        app_name, level, ..
    } = config;

    let file_appender = tracing_appender::rolling::RollingFileAppender::new(
        tracing_appender::rolling::Rotation::DAILY,
        &dir,
        format!("{app_name}.log"),
    );
    let (writer, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level.to_string()));

    // tracing_subscriber's `Option<L>: Layer` blanket impl makes the
    // stderr layer no-op when `None`. Keeps both branches a single
    // statically-typed subscriber.
    let stderr_layer =
        also_to_stderr.then(|| tracing_subscriber::fmt::layer().with_writer(std::io::stderr));

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_writer(writer))
        .with(stderr_layer);

    tracing::subscriber::set_global_default(subscriber).context(SetGlobalDefaultSnafu)?;
    Ok(guard)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_constructor_defaults() {
        let cfg = LogConfig::new("myapp", tracing::Level::INFO);
        assert_eq!(cfg.app_name, "myapp");
        assert_eq!(cfg.level, tracing::Level::INFO);
        assert!(cfg.log_dir.is_none());
    }

    #[test]
    fn with_log_dir_override() {
        let custom = PathBuf::from("/tmp/explicit/logs");
        let cfg = LogConfig::new("app", tracing::Level::DEBUG).with_log_dir(&custom);
        assert_eq!(cfg.log_dir, Some(custom.clone()));
        let resolved = cfg.resolve_log_dir().unwrap();
        assert_eq!(resolved, custom);
    }

    #[test]
    fn resolve_default_path_contains_app_segment() {
        let cfg = LogConfig::new("bathron-test-app", tracing::Level::INFO);
        // On any supported platform either state_dir() or
        // data_local_dir() returns Some(_); the resolved path must
        // contain both the app_name and a "logs" leaf.
        let resolved = cfg
            .resolve_log_dir()
            .expect("platform must expose a state dir");
        let s = resolved.to_string_lossy();
        assert!(s.contains("bathron-test-app"), "got {s}");
        assert!(s.ends_with("logs"), "got {s}");
    }

    #[test]
    fn explicit_log_dir_skips_state_lookup() {
        // Even if state_dir() were unavailable, an explicit log_dir
        // must short-circuit. We verify by setting an absurd path
        // and confirming resolve returns it verbatim.
        let weird = PathBuf::from("/dev/null/not-real");
        let cfg = LogConfig::new("x", tracing::Level::WARN).with_log_dir(&weird);
        assert_eq!(cfg.resolve_log_dir().unwrap(), weird);
    }

    #[test]
    fn config_constructor_accepts_string_via_into() {
        // app_name: impl Into<String> covers both &str and String.
        let cfg_str = LogConfig::new("from-str", tracing::Level::TRACE);
        let cfg_string = LogConfig::new(String::from("from-str"), tracing::Level::TRACE);
        assert_eq!(cfg_str.app_name, cfg_string.app_name);
    }

    #[test]
    fn with_log_dir_accepts_pathbuf_and_str() {
        // with_log_dir: impl Into<PathBuf> covers both PathBuf and &Path.
        let cfg_buf =
            LogConfig::new("a", tracing::Level::INFO).with_log_dir(PathBuf::from("/tmp/x"));
        let cfg_str =
            LogConfig::new("a", tracing::Level::INFO).with_log_dir(std::path::Path::new("/tmp/x"));
        assert_eq!(cfg_buf.log_dir, cfg_str.log_dir);
    }

    #[test]
    fn config_clone_preserves_fields() {
        let original = LogConfig::new("myapp", tracing::Level::DEBUG)
            .with_log_dir(PathBuf::from("/var/log/myapp"));
        let cloned = original.clone();
        assert_eq!(cloned.app_name, original.app_name);
        assert_eq!(cloned.level, original.level);
        assert_eq!(cloned.log_dir, original.log_dir);
    }

    #[test]
    fn config_debug_format_includes_field_values() {
        let cfg = LogConfig::new("debug-test", tracing::Level::WARN);
        let formatted = format!("{cfg:?}");
        // tracing::Level's Debug renders as Level(Warn); both
        // app_name and level appear in the rendered LogConfig debug.
        assert!(formatted.contains("debug-test"), "got {formatted}");
        assert!(formatted.contains("Warn"), "got {formatted}");
    }

    #[test]
    fn resolve_log_dir_accepts_temp_dir() {
        // resolve_log_dir with an explicit temp dir override returns
        // the temp dir verbatim, regardless of state_dir() availability.
        let tmp = tempfile::TempDir::new().expect("tempdir creation");
        let cfg = LogConfig::new("temp-app", tracing::Level::ERROR).with_log_dir(tmp.path());
        let resolved = cfg.resolve_log_dir().expect("explicit dir must resolve");
        assert_eq!(resolved, tmp.path());
    }

    #[test]
    fn resolve_log_dir_default_path_is_absolute() {
        let cfg = LogConfig::new("absolute-test", tracing::Level::INFO);
        let resolved = cfg
            .resolve_log_dir()
            .expect("platform must expose a state dir");
        assert!(
            resolved.is_absolute(),
            "default log_dir must be absolute, got {}",
            resolved.display()
        );
    }

    #[test]
    fn resolve_log_dir_with_explicit_dir_skips_app_segment() {
        // When log_dir is set, resolve returns it verbatim — does NOT
        // append the app_name or a "logs" leaf. The override is total.
        let custom = PathBuf::from("/var/log/custom-no-app-segment");
        let cfg = LogConfig::new("ignored-app", tracing::Level::INFO).with_log_dir(&custom);
        let resolved = cfg.resolve_log_dir().unwrap();
        assert_eq!(resolved, custom);
        assert!(!resolved.to_string_lossy().contains("ignored-app"));
        assert!(!resolved.ends_with("logs"));
    }

    #[test]
    fn no_state_dir_error_displays_message() {
        let err = LoggingError::NoStateDir;
        assert_eq!(
            err.to_string(),
            "could not determine user state directory for logs"
        );
    }

    #[test]
    fn create_dir_error_displays_path_and_source() {
        let path = PathBuf::from("/tmp/some-non-creatable-path");
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err = LoggingError::CreateDir {
            path: path.clone(),
            source: io_err,
        };
        let display = err.to_string();
        assert!(
            display.contains("/tmp/some-non-creatable-path"),
            "got {display}"
        );
        assert!(
            display.contains("failed to create log directory"),
            "got {display}"
        );
    }

    #[test]
    fn logging_error_is_send_sync() {
        // Snafu-derived errors should be both Send + Sync so they
        // cross thread / await boundaries cleanly. Compile-time check.
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<LoggingError>();
    }

    #[test]
    fn logging_error_implements_std_error() {
        // Snafu-derived errors should impl std::error::Error so they
        // compose with `?` into anyhow / boxed-error chains.
        fn assert_error<T: std::error::Error>() {}
        assert_error::<LoggingError>();
    }
}
