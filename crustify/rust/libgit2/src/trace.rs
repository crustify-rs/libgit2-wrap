//! Safe wrappers for libgit2 trace APIs.

use crate::ffi;

/// Wraps: git_trace_level_t
/// A checked tracing threshold understood by libgit2.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GitTraceLevel {
    /// Disable tracing.
    None = ffi::git_trace_level_t_GIT_TRACE_NONE,
    /// Severe errors that may affect execution.
    Fatal = ffi::git_trace_level_t_GIT_TRACE_FATAL,
    /// Errors that do not affect execution.
    Error = ffi::git_trace_level_t_GIT_TRACE_ERROR,
    /// Warnings about abnormal data or behavior.
    Warn = ffi::git_trace_level_t_GIT_TRACE_WARN,
    /// Informational execution messages.
    Info = ffi::git_trace_level_t_GIT_TRACE_INFO,
    /// Detailed debugging data.
    Debug = ffi::git_trace_level_t_GIT_TRACE_DEBUG,
    /// Exceptionally detailed debugging data.
    Trace = ffi::git_trace_level_t_GIT_TRACE_TRACE,
}

/// A raw trace level not published by libgit2.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitTraceLevel(ffi::git_trace_level_t);

impl InvalidGitTraceLevel {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_trace_level_t {
        self.0
    }
}

impl From<GitTraceLevel> for ffi::git_trace_level_t {
    fn from(level: GitTraceLevel) -> Self {
        level as Self
    }
}

impl TryFrom<ffi::git_trace_level_t> for GitTraceLevel {
    type Error = InvalidGitTraceLevel;

    fn try_from(level: ffi::git_trace_level_t) -> Result<Self, InvalidGitTraceLevel> {
        match level {
            ffi::git_trace_level_t_GIT_TRACE_NONE => Ok(Self::None),
            ffi::git_trace_level_t_GIT_TRACE_FATAL => Ok(Self::Fatal),
            ffi::git_trace_level_t_GIT_TRACE_ERROR => Ok(Self::Error),
            ffi::git_trace_level_t_GIT_TRACE_WARN => Ok(Self::Warn),
            ffi::git_trace_level_t_GIT_TRACE_INFO => Ok(Self::Info),
            ffi::git_trace_level_t_GIT_TRACE_DEBUG => Ok(Self::Debug),
            ffi::git_trace_level_t_GIT_TRACE_TRACE => Ok(Self::Trace),
            value => Err(InvalidGitTraceLevel(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn published_trace_levels_round_trip_in_threshold_order() {
        let levels = [
            GitTraceLevel::None,
            GitTraceLevel::Fatal,
            GitTraceLevel::Error,
            GitTraceLevel::Warn,
            GitTraceLevel::Info,
            GitTraceLevel::Debug,
            GitTraceLevel::Trace,
        ];

        for (raw, level) in levels.into_iter().enumerate() {
            assert_eq!(ffi::git_trace_level_t::from(level), raw as u32);
            assert_eq!(GitTraceLevel::try_from(raw as u32), Ok(level));
        }
    }

    #[test]
    fn invalid_trace_levels_are_rejected() {
        let raw = ffi::git_trace_level_t_GIT_TRACE_TRACE + 1;
        assert_eq!(GitTraceLevel::try_from(raw).unwrap_err().value(), raw);
    }

    #[test]
    fn trace_level_matches_the_c_abi_scalar() {
        assert_eq!(
            size_of::<GitTraceLevel>(),
            size_of::<ffi::git_trace_level_t>()
        );
        assert_eq!(
            align_of::<GitTraceLevel>(),
            align_of::<ffi::git_trace_level_t>()
        );
    }
}
