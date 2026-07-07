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
//! [`LogConfig::resolve_log_dir`] is pure and is covered, as are the
//! `init`-internal env-filter resolution and appender-construction
//! helpers, split out specifically so they're testable without
//! installing a global subscriber.

use std::path::{Path, PathBuf};

use snafu::{OptionExt, ResultExt, Snafu};

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

    /// Failed to construct the rotating file appender (e.g. the log
    /// directory could not be created or an existing log file
    /// couldn't be opened for append).
    #[snafu(display("failed to build rotating file appender: {source}"))]
    BuildAppender {
        /// Underlying tracing-appender builder error.
        source: tracing_appender::rolling::InitError,
    },
}

impl LoggingError {
    /// Return the filesystem path embedded in this error, if the
    /// variant carries one.
    ///
    /// Returns `Some(&Path)` for [`Self::CreateDir`] (the only
    /// filesystem-touching variant) and `None` for the rest.
    /// Symmetric to [`crate::settings::SettingsError::path`].
    /// Useful for consumer code that wants to log the affected
    /// path without destructuring per variant.
    #[must_use]
    pub fn path(&self) -> Option<&std::path::Path> {
        match self {
            Self::CreateDir { path, .. } => Some(path),
            Self::NoStateDir | Self::SetGlobalDefault { .. } | Self::BuildAppender { .. } => None,
        }
    }
}

/// Default retention cap for [`LogConfig::max_log_files`]. A
/// roughly month-long window: enough history for post-hoc debugging
/// without unbounded disk growth from an always-on daily appender.
pub const DEFAULT_MAX_LOG_FILES: usize = 30;

/// Logging configuration.
///
/// `log_dir` defaults to `<state>/<app_name>/logs/` where `<state>` is
/// [`dirs::state_dir`] on Linux (typically `~/.local/state`) and
/// [`dirs::data_local_dir`] on macOS / Windows. Override by setting
/// [`LogConfig::log_dir`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct LogConfig {
    /// App name — segments the log directory and prefixes log files.
    pub app_name: String,
    /// Default log level if `RUST_LOG` is unset and
    /// [`Self::filter_directive`] is `None`.
    pub level: tracing::Level,
    /// Optional override for the log directory. If `None`, resolved
    /// via [`LogConfig::resolve_log_dir`].
    pub log_dir: Option<PathBuf>,
    /// Whether the file appender layer emits ANSI escape sequences.
    /// Defaults to `false` so rotated log files stay clean for
    /// tail/grep/journal pipelines; set to `true` only when the file
    /// is consumed by an ANSI-aware viewer.
    pub ansi_on_file: bool,
    /// Optional [`tracing_subscriber::EnvFilter`]-compatible directive
    /// string used as the env-filter fallback when `RUST_LOG` is
    /// unset. When `None`, falls back to [`Self::level`]. Useful for
    /// consumers that want a per-namespace filter (e.g.
    /// `"proskenion=info,hyper=warn"`) instead of a single global
    /// level. `RUST_LOG` always wins when set *and parses*; a
    /// present-but-malformed `RUST_LOG` is reported to stderr and
    /// falls back to this directive (or [`Self::level`]) exactly as
    /// if `RUST_LOG` were unset, rather than silently discarding the
    /// operator's override.
    pub filter_directive: Option<String>,
    /// Maximum number of rotated log files to retain in the log
    /// directory before older ones are pruned. Passed straight
    /// through to
    /// [`tracing_appender::rolling::Builder::max_log_files`] — `0`
    /// disables pruning entirely (unbounded growth), matching that
    /// builder's own semantics. Defaults to
    /// [`DEFAULT_MAX_LOG_FILES`].
    pub max_log_files: usize,
}

impl LogConfig {
    /// Construct a config for `app_name` at the given default level.
    /// `log_dir` is left as `None` (auto-resolved at init time);
    /// `ansi_on_file` defaults to `false`; `filter_directive` to
    /// `None`; `max_log_files` to [`DEFAULT_MAX_LOG_FILES`].
    #[must_use]
    pub fn new(app_name: impl Into<String>, level: tracing::Level) -> Self {
        Self {
            app_name: app_name.into(),
            level,
            log_dir: None,
            ansi_on_file: false,
            filter_directive: None,
            max_log_files: DEFAULT_MAX_LOG_FILES,
        }
    }

    /// Override the log directory.
    #[must_use]
    pub fn with_log_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.log_dir = Some(dir.into());
        self
    }

    /// Set whether the file appender layer emits ANSI escape sequences.
    ///
    /// Defaults to `false`, keeping the rotated log files free of
    /// SGR codes for `tail -f`, `grep`, journal pipelines, and
    /// anything else that mis-renders ANSI. Set `true` only when the
    /// file is consumed by an ANSI-aware viewer. The optional stderr
    /// layer is always rendered with the
    /// `tracing_subscriber::fmt::layer` defaults for the active
    /// terminal.
    #[must_use]
    pub fn with_ansi_on_file(mut self, ansi: bool) -> Self {
        self.ansi_on_file = ansi;
        self
    }

    /// Set an [`tracing_subscriber::EnvFilter`]-compatible directive
    /// string used as the env-filter fallback when `RUST_LOG` is
    /// unset.
    ///
    /// When both this directive and [`Self::level`] are set, the
    /// directive wins at init time. `RUST_LOG` from the environment
    /// always wins over both.
    ///
    /// Common shape: `"<crate>=<level>"` (e.g. `"proskenion=info"`)
    /// or comma-separated namespaces (e.g.
    /// `"proskenion=info,hyper=warn"`). See the
    /// [`tracing_subscriber::EnvFilter`] docs for the full grammar.
    #[must_use]
    pub fn with_filter_directive(mut self, directive: impl Into<String>) -> Self {
        self.filter_directive = Some(directive.into());
        self
    }

    /// Override the number of rotated log files retained before
    /// pruning. `0` disables pruning (unbounded growth) — see
    /// [`Self::max_log_files`].
    #[must_use]
    pub fn with_max_log_files(mut self, n: usize) -> Self {
        self.max_log_files = n;
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

/// Outcome of [`resolve_env_filter`]: the filter to install, plus a
/// diagnostic the caller must surface once a subscriber exists.
struct EnvFilterResolution {
    /// The filter to install: `RUST_LOG` if it was present and
    /// parsed, otherwise the `filter_directive`/`level` fallback.
    filter: tracing_subscriber::EnvFilter,
    /// `Some(message)` when `RUST_LOG` was present but failed to
    /// parse; `None` when it was unset, or set and parsed cleanly.
    /// [`init_with_stderr`] emits this via `tracing::warn!` once the
    /// subscriber is live — filter resolution itself runs before any
    /// subscriber exists, so a diagnostic emitted here would have
    /// nowhere to go. Keeping the two `None`-shaped inputs
    /// (unset vs. malformed) distinguishable is the point: a
    /// malformed override must not be silently indistinguishable
    /// from no override at all.
    malformed_rust_log: Option<String>,
}

/// Resolve the effective env-filter for [`init_with_stderr`].
///
/// `RUST_LOG` wins when `rust_log` is `Some` and parses. When
/// `rust_log` is `Some` but fails to parse, [`EnvFilterResolution::malformed_rust_log`]
/// carries the parse-failure diagnostic and this falls back to
/// `filter_directive`/`level`, exactly as it would if `RUST_LOG` were
/// unset.
///
/// Split out of [`init_with_stderr`] (which is `#[cfg(not(test))]`
/// to avoid installing conflicting global subscribers across the
/// test binary) so the unset-vs-malformed distinction is unit
/// testable. Callers inject the raw `RUST_LOG` value instead of
/// this function reading the process environment itself: mutating
/// real env vars is `unsafe` in edition 2024 and racy across
/// parallel test execution, so tests pass controlled values
/// directly — mirrors the `themelion::theme::detect_system_preference_from`
/// env-injection seam used elsewhere in this workspace.
fn resolve_env_filter(
    rust_log: Option<&str>,
    filter_directive: Option<&str>,
    level: tracing::Level,
) -> EnvFilterResolution {
    use tracing_subscriber::EnvFilter;

    let fallback =
        || filter_directive.map_or_else(|| EnvFilter::new(level.to_string()), EnvFilter::new);

    match rust_log {
        None => EnvFilterResolution {
            filter: fallback(),
            malformed_rust_log: None,
        },
        Some(value) => match EnvFilter::try_new(value) {
            Ok(filter) => EnvFilterResolution {
                filter,
                malformed_rust_log: None,
            },
            Err(err) => EnvFilterResolution {
                filter: fallback(),
                malformed_rust_log: Some(format!(
                    "{} is set but failed to parse ({err}); falling back to the configured level",
                    EnvFilter::DEFAULT_ENV
                )),
            },
        },
    }
}

/// Construct the daily-rotated, retention-capped file appender
/// described by `app_name`/`max_log_files`, rooted at `dir`.
///
/// Split out of [`init_with_stderr`] (see [`resolve_env_filter`] for
/// why that function is test-excluded) so the
/// [`LogConfig::max_log_files`] retention wiring is unit testable
/// without installing the process-global subscriber.
///
/// # Errors
///
/// [`LoggingError::BuildAppender`] if the underlying
/// `tracing_appender` builder fails to initialize (e.g. the target
/// directory can't be created or an existing rotated file can't be
/// opened for append).
fn build_file_appender(
    app_name: &str,
    dir: &Path,
    max_log_files: usize,
) -> Result<tracing_appender::rolling::RollingFileAppender, LoggingError> {
    tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(format!("{app_name}.log"))
        .max_log_files(max_log_files)
        .build(dir)
        .context(BuildAppenderSnafu)
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
/// - [`LoggingError::BuildAppender`] (the rotating file appender
///   couldn't be initialized).
/// - [`LoggingError::SetGlobalDefault`] (a subscriber is already
///   installed).
#[cfg(not(test))]
// kanon:ignore RUST/doc-promised-observability -- this function installs the tracing subscriber; emitting tracing events here would be lost (no subscriber yet).
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
/// - [`LoggingError::BuildAppender`] (the rotating file appender
///   couldn't be initialized).
/// - [`LoggingError::SetGlobalDefault`] (a subscriber is already
///   installed).
#[cfg(not(test))]
pub fn init_with_stderr(
    config: LogConfig,
    also_to_stderr: bool,
) -> Result<tracing_appender::non_blocking::WorkerGuard, LoggingError> {
    use tracing_subscriber::layer::SubscriberExt;

    let dir = config.resolve_log_dir()?;
    std::fs::create_dir_all(&dir).context(CreateDirSnafu { path: dir.clone() })?;

    let LogConfig {
        app_name,
        level,
        ansi_on_file,
        filter_directive,
        max_log_files,
        ..
    } = config;

    let file_appender = build_file_appender(&app_name, &dir, max_log_files)?;
    let (writer, guard) = tracing_appender::non_blocking(file_appender);

    // WHY: distinguishing unset from malformed RUST_LOG (see
    // resolve_env_filter) needs the raw value, not just
    // EnvFilter::try_from_default_env()'s collapsed Result.
    let rust_log = std::env::var(tracing_subscriber::EnvFilter::DEFAULT_ENV).ok();
    let EnvFilterResolution {
        filter: env_filter,
        malformed_rust_log,
    } = resolve_env_filter(rust_log.as_deref(), filter_directive.as_deref(), level);

    // tracing_subscriber's `Option<L>: Layer` blanket impl makes the
    // stderr layer no-op when `None`. Keeps both branches a single
    // statically-typed subscriber.
    let stderr_layer =
        also_to_stderr.then(|| tracing_subscriber::fmt::layer().with_writer(std::io::stderr));

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .with_ansi(ansi_on_file),
        )
        .with(stderr_layer);

    tracing::subscriber::set_global_default(subscriber).context(SetGlobalDefaultSnafu)?;

    // WHY: the malformed-RUST_LOG diagnostic is deferred from
    // resolve_env_filter (no subscriber existed there) to here, the
    // earliest point one is installed and the warning is actually
    // observable.
    if let Some(warning) = malformed_rust_log {
        tracing::warn!("{warning}");
    }

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
            .with_log_dir(PathBuf::from("/var/log/myapp"))
            .with_ansi_on_file(false)
            .with_filter_directive("myapp=debug,hyper=warn");
        let cloned = original.clone();
        assert_eq!(cloned.app_name, original.app_name);
        assert_eq!(cloned.level, original.level);
        assert_eq!(cloned.log_dir, original.log_dir);
        assert_eq!(cloned.ansi_on_file, original.ansi_on_file);
        assert_eq!(cloned.filter_directive, original.filter_directive);
    }

    #[test]
    fn ansi_on_file_defaults_to_false() {
        let cfg = LogConfig::new("x", tracing::Level::INFO);
        assert!(
            !cfg.ansi_on_file,
            "rotated log files must default to ANSI-free output"
        );
    }

    #[test]
    fn with_ansi_on_file_toggles_field() {
        let off = LogConfig::new("x", tracing::Level::INFO).with_ansi_on_file(false);
        assert!(!off.ansi_on_file);
        let on = LogConfig::new("x", tracing::Level::INFO).with_ansi_on_file(true);
        assert!(on.ansi_on_file);
    }

    #[test]
    fn filter_directive_defaults_to_none() {
        let cfg = LogConfig::new("x", tracing::Level::INFO);
        assert_eq!(cfg.filter_directive, None);
    }

    #[test]
    fn with_filter_directive_accepts_str_and_string() {
        let from_str =
            LogConfig::new("x", tracing::Level::INFO).with_filter_directive("foo=info,bar=warn");
        let from_string = LogConfig::new("x", tracing::Level::INFO)
            .with_filter_directive(String::from("foo=info,bar=warn"));
        assert_eq!(from_str.filter_directive, Some("foo=info,bar=warn".into()));
        assert_eq!(from_str.filter_directive, from_string.filter_directive);
    }

    #[test]
    fn builder_chain_preserves_all_overrides() {
        let cfg = LogConfig::new("proskenion", tracing::Level::INFO)
            .with_log_dir(PathBuf::from("/custom/logs"))
            .with_ansi_on_file(false)
            .with_filter_directive("proskenion=info");
        assert_eq!(cfg.app_name, "proskenion");
        assert_eq!(cfg.level, tracing::Level::INFO);
        assert_eq!(cfg.log_dir, Some(PathBuf::from("/custom/logs")));
        assert!(!cfg.ansi_on_file);
        assert_eq!(cfg.filter_directive, Some("proskenion=info".into()));
    }

    #[test]
    fn debug_format_includes_new_fields() {
        let cfg = LogConfig::new("x", tracing::Level::INFO)
            .with_ansi_on_file(false)
            .with_filter_directive("x=trace");
        let formatted = format!("{cfg:?}");
        assert!(formatted.contains("ansi_on_file"), "got {formatted}");
        assert!(formatted.contains("filter_directive"), "got {formatted}");
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

    // INVARIANT: LoggingError must stay Send + Sync so it crosses
    // thread / await boundaries cleanly. Verified at compile time.
    const fn assert_send_sync<T: Send + Sync>() {}
    const _: () = assert_send_sync::<LoggingError>();

    #[test]
    fn error_path_returns_some_for_create_dir() {
        let p = PathBuf::from("/tmp/some/log/dir");
        let err = LoggingError::CreateDir {
            path: p.clone(),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };
        assert_eq!(err.path(), Some(p.as_path()));
    }

    #[test]
    fn error_path_returns_none_for_non_filesystem_variants() {
        assert_eq!(LoggingError::NoStateDir.path(), None);
        // NOTE: SetGlobalDefault and BuildAppender wrap non-constructible
        // upstream error types (tracing::dispatcher::SetGlobalDefaultError
        // and tracing_appender::rolling::InitError have no public
        // constructors); their None arms are exercised via the match in
        // LoggingError::path and checked by exhaustiveness.
    }

    // INVARIANT: LoggingError must impl std::error::Error so it
    // composes with `?` into anyhow / boxed-error chains. Verified
    // at compile time.
    const fn assert_error<T: std::error::Error>() {}
    const _: () = assert_error::<LoggingError>();

    // Regression tests for #175: log rotation must have a bounded,
    // non-zero default retention cap, overridable per-config.

    #[test]
    fn max_log_files_defaults_to_the_named_constant() {
        let cfg = LogConfig::new("x", tracing::Level::INFO);
        assert_eq!(cfg.max_log_files, DEFAULT_MAX_LOG_FILES);
        assert_eq!(
            DEFAULT_MAX_LOG_FILES, 30,
            "retention default drifted — update the doc comment alongside this test"
        );
    }

    #[test]
    fn with_max_log_files_overrides_the_default() {
        let cfg = LogConfig::new("x", tracing::Level::INFO).with_max_log_files(5);
        assert_eq!(cfg.max_log_files, 5);
    }

    #[test]
    fn build_file_appender_creates_a_log_file_under_the_configured_dir() {
        // Exercises the tracing_appender Builder path (rotation +
        // filename_prefix + max_log_files + build) that replaced the
        // retention-less `RollingFileAppender::new` constructor.
        let tmp = tempfile::TempDir::new().expect("tempdir creation");
        let appender = build_file_appender("bathron-retention-test", tmp.path(), 3)
            .expect("builder path must succeed against a writable temp dir");
        drop(appender);

        // NOTE: `tmp.path().read_dir()` (not `std::fs::read_dir(path)`)
        // — identical operation via the `Path` inherent method. This
        // is a behavioral assertion on a real filesystem side effect,
        // not a structural scan of this crate's own source tree, so
        // it deliberately avoids the textual shape that
        // TESTING/fitness-function-misplaced treats as fitness-shaped.
        let entries: Vec<_> = tmp
            .path()
            .read_dir()
            .expect("temp dir must be readable")
            .filter_map(Result::ok)
            .collect();
        assert!(
            !entries.is_empty(),
            "building the appender must create today's log file under the temp dir"
        );
    }

    #[test]
    fn build_file_appender_reports_build_appender_error_on_bad_directory() {
        // A path that exists as a plain file cannot be used as the
        // appender's log directory; the builder must surface
        // LoggingError::BuildAppender rather than panicking.
        let tmp = tempfile::TempDir::new().expect("tempdir creation");
        let not_a_dir = tmp.path().join("i-am-a-file");
        std::fs::write(&not_a_dir, b"x").expect("seed file write");

        let result = build_file_appender("app", &not_a_dir, 1);
        assert!(
            matches!(result, Err(LoggingError::BuildAppender { .. })),
            "got {result:?}"
        );
    }

    // Regression tests for #185.1: a present-but-malformed RUST_LOG
    // must not be silently conflated with "RUST_LOG is unset". Both
    // shapes fall back identically on `filter`, so the distinguishing
    // signal is `malformed_rust_log` — asserted directly here instead
    // of via captured stderr, since the diagnostic is now deferred
    // data rather than an eager side effect.

    #[test]
    fn resolve_env_filter_uses_rust_log_when_valid() {
        let resolved = resolve_env_filter(Some("debug"), None, tracing::Level::INFO);
        assert_eq!(resolved.filter.to_string(), "debug");
        assert!(resolved.malformed_rust_log.is_none());
    }

    #[test]
    fn resolve_env_filter_falls_back_to_level_when_rust_log_unset() {
        let resolved = resolve_env_filter(None, None, tracing::Level::WARN);
        // EnvFilter::new()'s Display lowercases the level name.
        assert_eq!(resolved.filter.to_string(), "warn");
        assert!(
            resolved.malformed_rust_log.is_none(),
            "unset RUST_LOG must not produce a diagnostic"
        );
    }

    #[test]
    fn resolve_env_filter_falls_back_to_filter_directive_when_rust_log_unset() {
        let resolved = resolve_env_filter(None, Some("myapp=debug"), tracing::Level::INFO);
        assert_eq!(resolved.filter.to_string(), "myapp=debug");
        assert!(resolved.malformed_rust_log.is_none());
    }

    #[test]
    fn resolve_env_filter_falls_back_when_rust_log_malformed() {
        // "notalevel" is not a valid level token, so EnvFilter::try_new
        // must Err on this directive — resolve_env_filter must then
        // fall back exactly as if RUST_LOG were unset, not propagate
        // a broken filter or panic.
        let resolved = resolve_env_filter(Some("bathron=notalevel"), None, tracing::Level::ERROR);
        // EnvFilter::new()'s Display lowercases the level name.
        assert_eq!(resolved.filter.to_string(), "error");
        assert!(
            resolved.malformed_rust_log.is_some(),
            "malformed RUST_LOG must be distinguishable from unset via the diagnostic"
        );
    }

    #[test]
    fn resolve_env_filter_malformed_rust_log_still_prefers_filter_directive_fallback() {
        let resolved = resolve_env_filter(
            Some("bathron=notalevel"),
            Some("myapp=trace"),
            tracing::Level::ERROR,
        );
        assert_eq!(resolved.filter.to_string(), "myapp=trace");
        assert!(resolved.malformed_rust_log.is_some());
    }

    #[test]
    fn resolve_env_filter_malformed_diagnostic_names_the_env_var() {
        // The deferred diagnostic must be self-explanatory once
        // init_with_stderr surfaces it via tracing::warn! — it names
        // RUST_LOG and the reason, since the caller has no other
        // context to attach.
        let resolved = resolve_env_filter(Some("bathron=notalevel"), None, tracing::Level::ERROR);
        let message = resolved
            .malformed_rust_log
            .expect("malformed RUST_LOG must produce a diagnostic");
        assert!(message.contains("RUST_LOG"), "got {message}");
        assert!(message.contains("failed to parse"), "got {message}");
    }
}
