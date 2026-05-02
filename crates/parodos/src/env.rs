//! Minimal environment-variable abstraction for parodos.
//!
//! Mirrors the [`var`](Env::var) surface of `koina::system::Environment` so
//! parodos can detect terminal capabilities (COLORTERM, TERM, COLORFGBG,
//! TMUX, etc.) without depending on aletheia. Production code uses
//! [`RealEnv`]; tests can pass any [`Env`] implementation to inject
//! deterministic values.
//!
//! # Why a parodos-local trait
//!
//! Parodos is a TUI substrate that ships in theatron, downstream of aletheia.
//! It cannot depend on `koina::system` without inverting the layering
//! (theatron → aletheia is forbidden — theatron is upstream of aletheia).
//! The trait is small and the standard-library backing is the same.

/// Read environment variables.
pub trait Env: Send + Sync {
    /// Return the value of environment variable `name`, or `None` if unset
    /// or holds non-UTF-8 bytes.
    #[must_use]
    fn var(&self, name: &str) -> Option<String>;
}

/// Production [`Env`] backed by `std::env::var`.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealEnv;

impl Env for RealEnv {
    fn var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// In-memory [`Env`] for tests.
    struct TestEnv {
        vars: HashMap<String, String>,
    }

    impl Env for TestEnv {
        fn var(&self, name: &str) -> Option<String> {
            self.vars.get(name).cloned()
        }
    }

    #[test]
    fn real_env_returns_some_for_path() {
        // PATH is set on every supported platform.
        assert!(RealEnv.var("PATH").is_some());
    }

    #[test]
    fn real_env_returns_none_for_missing() {
        assert!(
            RealEnv
                .var("PARODOS_TEST_VAR_THAT_SHOULD_NOT_EXIST")
                .is_none()
        );
    }

    #[test]
    fn injected_env_serves_value_for_known_key() {
        let env = TestEnv {
            vars: [("COLORTERM".to_owned(), "truecolor".to_owned())]
                .into_iter()
                .collect(),
        };
        assert_eq!(env.var("COLORTERM"), Some("truecolor".to_owned()));
        assert_eq!(env.var("MISSING"), None);
    }
}
