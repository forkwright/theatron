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
    use tracing_subscriber::{EnvFilter, layer::SubscriberExt};

    let dir = config.resolve_log_dir()?;
    std::fs::create_dir_all(&dir).context(CreateDirSnafu { path: dir.clone() })?;

    let file_appender = tracing_appender::rolling::RollingFileAppender::new(
        tracing_appender::rolling::Rotation::DAILY,
        &dir,
        format!("{}.log", config.app_name),
    );
    let (writer, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(config.level.to_string()));

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_writer(writer));

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
}
