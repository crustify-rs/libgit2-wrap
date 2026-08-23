//! Safe wrappers for libgit2 transport APIs.

/// Wraps: git_transport_message_cb
/// Safe callable surface for length-delimited transport messages.
pub trait GitTransportMessageCallback {
    /// Receives transient message bytes, which need not be NUL-terminated.
    fn call(&mut self, message: &[u8]) -> i32;
}

impl<F> GitTransportMessageCallback for F
where
    F: FnMut(&[u8]) -> i32,
{
    fn call(&mut self, message: &[u8]) -> i32 {
        self(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_callback_preserves_non_nul_bytes() {
        let mut callback = |message: &[u8]| message.len() as i32;
        assert_eq!(GitTransportMessageCallback::call(&mut callback, b"a\0b"), 3);
    }
}
