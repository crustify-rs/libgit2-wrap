//! Safe wrappers for libgit2 trace APIs.

use core::ffi::CStr;

use crate::trace::GitTraceLevel;

/// Wraps: git_trace_cb
/// Safe callable surface for a transient trace message.
///
/// `git_trace_cb` carries no `void *payload`: libgit2 stores one process-global
/// callback and invokes it with nothing but the level and the message. A
/// callable that captured state would therefore have nowhere to be reached
/// from, so this surface is a plain `fn` pointer and a callback that needs
/// state reaches it through its own `static`.
///
/// `message` is the formatted trace line, borrowed only for the invocation.
pub type GitTraceCallback = fn(GitTraceLevel, &CStr);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_functions_implement_the_trace_callback_surface() {
        fn observer(_: GitTraceLevel, _: &CStr) {}
        let callback: GitTraceCallback = observer;
        callback(GitTraceLevel::Info, c"message");
    }
}
