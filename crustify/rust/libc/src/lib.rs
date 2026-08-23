//! Safe wrappers for the campaign's libc dependency surface.

/// Raw libc bindings used by this campaign.
#[allow(unused_imports)]
pub mod ffi {
    pub use libc_sys::*;
}
