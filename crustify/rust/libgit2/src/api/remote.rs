//! Safe wrappers for libgit2 remote APIs.

use core::ffi::CStr;
use core::marker::PhantomData;
use core::ptr::NonNull;

use crate::api::buffer::GitBufMut;
use crate::remote::GitPushUpdateRef;
use crate::util::net::Direction;

/// Wraps: git_url_resolve_cb
/// Safe callable surface for the deprecated remote URL resolver.
pub trait GitUrlResolveCallback {
    /// Writes the resolved URL to `output`.
    ///
    /// Return zero on success, `GIT_PASSTHROUGH`, or another libgit2 error.
    fn resolve(&mut self, output: &mut GitBufMut<'_>, url: &CStr, direction: Direction) -> i32;
}

impl<F> GitUrlResolveCallback for F
where
    F: FnMut(&mut GitBufMut<'_>, &CStr, Direction) -> i32,
{
    fn resolve(&mut self, output: &mut GitBufMut<'_>, url: &CStr, direction: Direction) -> i32 {
        self(output, url, direction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closures_implement_the_url_resolver_surface() {
        fn accepts<C: GitUrlResolveCallback>(_callback: C) {}
        accepts(|_: &mut GitBufMut<'_>, _: &CStr, _: Direction| 0);
    }
}

/// A borrowed pointer-array view supplied to push negotiation callbacks.
#[derive(Clone, Copy)]
pub struct GitPushUpdates<'a> {
    ptr: NonNull<*const crate::ffi::git_push_update>,
    len: usize,
    _borrow: PhantomData<GitPushUpdateRef<'a>>,
}

impl<'a> GitPushUpdates<'a> {
    /// Constructs the transient view used by a callback trampoline.
    ///
    /// # Safety
    /// `ptr` must address `len` readable pointers to live push updates for
    /// `'a`; every element must be non-null.
    #[allow(dead_code)]
    pub(crate) unsafe fn from_raw(
        ptr: *const *const crate::ffi::git_push_update,
        len: usize,
    ) -> Option<Self> {
        let ptr = if len == 0 && ptr.is_null() {
            NonNull::dangling()
        } else {
            NonNull::new(ptr.cast_mut())?
        };
        Some(Self {
            ptr,
            len,
            _borrow: PhantomData,
        })
    }

    /// Returns the number of updates.
    #[must_use]
    pub const fn len(self) -> usize {
        self.len
    }

    /// Returns whether there are no updates.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Borrows one update by position.
    #[must_use]
    pub fn get(self, index: usize) -> Option<GitPushUpdateRef<'a>> {
        if index >= self.len {
            return None;
        }
        // SAFETY: construction guarantees a readable `len`-element pointer
        // array, and the bounds check selects one initialized element.
        let update = unsafe { self.ptr.as_ptr().add(index).read() };
        // SAFETY: construction requires each element to name a live update for
        // `'a`; a null element is defensively rejected.
        unsafe { GitPushUpdateRef::from_ptr(update.cast_mut()) }
    }
}

/// Wraps: git_push_negotiation
/// Safe callable surface for the transient update array negotiated by a push.
pub trait GitPushNegotiationCallback {
    /// Returns zero to proceed or a nonzero libgit2 status to cancel.
    fn call(&mut self, updates: GitPushUpdates<'_>) -> i32;
}

impl<F> GitPushNegotiationCallback for F
where
    F: for<'a> FnMut(GitPushUpdates<'a>) -> i32,
{
    fn call(&mut self, updates: GitPushUpdates<'_>) -> i32 {
        self(updates)
    }
}

#[cfg(test)]
mod push_negotiation_tests {
    use core::mem::MaybeUninit;

    use super::*;

    #[test]
    fn pointer_array_view_checks_bounds_without_forming_a_slice() {
        let storage = Box::new(MaybeUninit::<crate::ffi::git_push_update>::zeroed());
        let update = Box::into_raw(storage).cast::<crate::ffi::git_push_update>();
        let pointers = [update.cast_const()];
        // SAFETY: the local array and opaque allocation remain live for this
        // view, and its sole element is non-null.
        let updates = unsafe { GitPushUpdates::from_raw(pointers.as_ptr(), 1) }.unwrap();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates.get(0).unwrap().as_ptr(), update.cast_const());
        assert!(updates.get(1).is_none());
        let mut callback = |values: GitPushUpdates<'_>| values.len() as i32;
        assert_eq!(GitPushNegotiationCallback::call(&mut callback, updates), 1);
        // SAFETY: no handle survives and this recovers the original allocation.
        drop(unsafe { Box::from_raw(update.cast::<MaybeUninit<crate::ffi::git_push_update>>()) });
    }
}
