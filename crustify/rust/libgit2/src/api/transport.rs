//! Safe wrappers for libgit2 transport APIs.

use crate::remote::GitRemoteMut;
use crate::sys::transport::GitTransportWithRemote;

/// Wraps: git_transport_cb
/// Safe callable surface for a factory that transfers one new transport to
/// the remote that will own and use it.
pub trait GitTransportCallback {
    /// Creates a transport whose lifetime remains coupled to `owner`.
    fn call<'remote>(
        &mut self,
        owner: GitRemoteMut<'remote>,
    ) -> Result<GitTransportWithRemote<'remote>, i32>;
}

impl<F> GitTransportCallback for F
where
    F: for<'remote> FnMut(GitRemoteMut<'remote>) -> Result<GitTransportWithRemote<'remote>, i32>,
{
    fn call<'remote>(
        &mut self,
        owner: GitRemoteMut<'remote>,
    ) -> Result<GitTransportWithRemote<'remote>, i32> {
        self(owner)
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    fn accepts_callback<C: GitTransportCallback>(_callback: &mut C) {}

    #[test]
    fn callback_trait_accepts_lifetime_coupled_factories() {
        fn factory<'remote>(
            _owner: GitRemoteMut<'remote>,
        ) -> Result<GitTransportWithRemote<'remote>, i32> {
            Err(-1)
        }
        accepts_callback(&mut factory);
    }
}
