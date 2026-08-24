//! Safe wrappers for libgit2 trace APIs.

use core::ffi::CStr;

use crate::trace::GitTraceLevel;

/// Wraps: git_trace_cb
/// Safe callable surface for a transient trace message.
pub trait GitTraceCallback {
    /// Receives one trace message at `level`.
    fn call(&mut self, level: GitTraceLevel, message: &CStr);
}

impl<F> GitTraceCallback for F
where
    F: FnMut(GitTraceLevel, &CStr),
{
    fn call(&mut self, level: GitTraceLevel, message: &CStr) {
        self(level, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closures_implement_the_trace_callback_surface() {
        fn accepts<C: GitTraceCallback>(_callback: C) {}
        accepts(|_: GitTraceLevel, _: &CStr| {});
    }
}
