//! Log levels and environment-aware level configuration.

use serde::{Deserialize, Serialize};

/// Severity level of a log record (ordered: Trace < Debug < … < Error).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LogLevel {
    /// Very verbose tracing.
    Trace,
    /// Debug diagnostics.
    Debug,
    /// Informational.
    Info,
    /// Warnings.
    Warn,
    /// Errors.
    Error,
}

impl LogLevel {
    /// Uppercase label used in structured output.
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    /// Parses a level from a string (case-insensitive). Returns `None` if
    /// unrecognized.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "TRACE" => Some(LogLevel::Trace),
            "DEBUG" => Some(LogLevel::Debug),
            "INFO" => Some(LogLevel::Info),
            "WARN" | "WARNING" => Some(LogLevel::Warn),
            "ERROR" => Some(LogLevel::Error),
            _ => None,
        }
    }
}

/// Deployment environment, which determines the default log level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Environment {
    /// Local development — verbose by default.
    Development,
    /// Production — concise by default.
    Production,
}

/// Resolved logging level configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelConfig {
    /// The minimum level that will be emitted.
    pub min_level: LogLevel,
}

impl LevelConfig {
    /// Default level for an environment: DEBUG for dev, INFO for production.
    pub fn for_environment(env: Environment) -> Self {
        let min_level = match env {
            Environment::Development => LogLevel::Debug,
            Environment::Production => LogLevel::Info,
        };
        Self { min_level }
    }

    /// Resolves config from an explicit override string (e.g. a `LOG_LEVEL`
    /// env var value) falling back to the environment default.
    pub fn resolve(env: Environment, override_level: Option<&str>) -> Self {
        match override_level.and_then(LogLevel::parse) {
            Some(min_level) => Self { min_level },
            None => Self::for_environment(env),
        }
    }

    /// Whether a record at `level` should be emitted.
    pub fn is_enabled(&self, level: LogLevel) -> bool {
        level >= self.min_level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_ordering() {
        assert!(LogLevel::Error > LogLevel::Warn);
        assert!(LogLevel::Trace < LogLevel::Info);
    }

    #[test]
    fn parse_is_case_insensitive() {
        assert_eq!(LogLevel::parse("info"), Some(LogLevel::Info));
        assert_eq!(LogLevel::parse("WARNING"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::parse("nonsense"), None);
    }

    #[test]
    fn environment_defaults() {
        assert_eq!(
            LevelConfig::for_environment(Environment::Production).min_level,
            LogLevel::Info
        );
        assert_eq!(
            LevelConfig::for_environment(Environment::Development).min_level,
            LogLevel::Debug
        );
    }

    #[test]
    fn override_takes_precedence() {
        let cfg = LevelConfig::resolve(Environment::Production, Some("trace"));
        assert_eq!(cfg.min_level, LogLevel::Trace);
        // Invalid override falls back to env default.
        let cfg2 = LevelConfig::resolve(Environment::Production, Some("bogus"));
        assert_eq!(cfg2.min_level, LogLevel::Info);
    }

    #[test]
    fn filtering() {
        let cfg = LevelConfig::for_environment(Environment::Production); // INFO
        assert!(!cfg.is_enabled(LogLevel::Debug));
        assert!(cfg.is_enabled(LogLevel::Info));
        assert!(cfg.is_enabled(LogLevel::Error));
    }
}
