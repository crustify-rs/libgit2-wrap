//! Safe Rust wrappers for libgit2.

/// Raw libgit2 bindings.
#[allow(unused_imports)]
pub mod ffi {
    pub use libgit2_sys::*;
}
