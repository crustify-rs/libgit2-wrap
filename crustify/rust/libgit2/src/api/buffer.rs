//! Safe wrappers for libgit2 buffer APIs.

use core::ptr::NonNull;

use ffibox::{CLenDropped, CSlice, CSliceMut, CVal, CVec};

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_buf
    /// Layout-compatible buffer header used for inline and C-owned storage.
    GitBuf,
    GitBufRef,
    GitBufMut,
    ffi::git_buf
);

// `git_buf_dispose` releases the buffer fields without freeing the header.
ffibox::impl_cvalued!(GitBuf, ffi::git_buf, ffi::git_buf_dispose);

/// Deleter for an allocated `git_buf` byte run that has been detached from its
/// header.
///
/// Libgit2 resolves its process-global allocator when this strategy is
/// dropped. The installed `gfree` must therefore remain compatible with the
/// allocation until then.
pub struct GitBufBytesFree;

// SAFETY: `c_drop_len` dispatches to libgit2's configured `gfree`; callers may
// construct the corresponding `CVec` only from a uniquely owned allocation
// made by the compatible configured allocator.
unsafe impl CLenDropped for GitBufBytesFree {
    unsafe fn c_drop_len(ptr: *mut u8, _byte_len: usize) {
        // SAFETY: the `CLenDropped` contract guarantees unique ownership and
        // allocator compatibility; libgit2's free routine accepts the erased
        // allocation pointer and does not require its length.
        unsafe { ffi::crustify_git__free(ptr.cast()) }
    }
}

/// Owned bytes detached from a [`GitBuf`].
///
/// The final byte is the buffer's required NUL terminator; preceding bytes may
/// contain arbitrary data, including interior NUL bytes.
pub type GitBufAllocation = CVec<u8, GitBufBytesFree>;

impl GitBuf {
    /// Construct an empty owned buffer header whose fields will be disposed on
    /// drop.
    #[must_use]
    pub fn new() -> CVal<Self> {
        CVal::new(Self::zeroed())
    }
}

impl GitBufRef<'_> {
    /// Wraps: git_buf.size
    /// Number of initialized content bytes, excluding the trailing NUL.
    #[inline]
    #[must_use]
    pub fn size(&self) -> usize {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).size).read() }
    }

    /// Wraps: git_buf.ptr
    /// Borrow the initialized content bytes without exposing the owned pointer.
    ///
    /// `None` denotes the valid null representation of an empty buffer. A
    /// non-null empty sentinel is represented by an empty `CSlice`.
    #[must_use]
    pub fn contents(&self) -> Option<CSlice<'_, u8>> {
        let header = self.as_ptr();
        // SAFETY: both fields are initialized members of the live header.
        let (ptr, size) = unsafe {
            (
                core::ptr::addr_of!((*header).ptr).read().cast::<u8>(),
                core::ptr::addr_of!((*header).size).read(),
            )
        };
        let ptr = NonNull::new(ptr)?;
        // SAFETY: a valid `git_buf` guarantees `size` initialized content bytes
        // at its non-null pointer. The view is tied to this handle borrow.
        Some(unsafe { CSlice::from_raw_parts(ptr, size) })
    }

    /// Wraps: git_buf.reserved
    /// Allocation capacity recorded by libgit2.
    #[inline]
    #[must_use]
    pub fn reserved(&self) -> usize {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).reserved).read() }
    }
}

impl GitBufMut<'_> {
    /// Borrow the initialized content bytes exclusively.
    #[must_use]
    pub fn contents_mut(&mut self) -> Option<CSliceMut<'_, u8>> {
        let header = self.as_mut_ptr();
        // SAFETY: both fields are initialized members of the exclusively
        // borrowed live header.
        let (ptr, size) = unsafe {
            (
                core::ptr::addr_of!((*header).ptr).read().cast::<u8>(),
                core::ptr::addr_of!((*header).size).read(),
            )
        };
        let ptr = NonNull::new(ptr)?;
        // SAFETY: a valid `git_buf` guarantees `size` initialized bytes at the
        // pointer, and the mutable handle provides exclusive access for the
        // returned view's lifetime.
        Some(unsafe { CSliceMut::from_raw_parts(ptr, size) })
    }

    /// Shrink the initialized contents and move the trailing NUL terminator.
    /// Returns `false` if `new_size` would grow the buffer or the header is not
    /// a valid non-empty representation.
    pub fn truncate(&mut self, new_size: usize) -> bool {
        let header = self.as_mut_ptr();
        // SAFETY: reads initialized scalar fields through the exclusive handle.
        let (ptr, old_size) = unsafe {
            (
                core::ptr::addr_of!((*header).ptr).read(),
                core::ptr::addr_of!((*header).size).read(),
            )
        };

        if new_size > old_size {
            return false;
        }
        if new_size == old_size {
            return true;
        }
        if ptr.is_null() {
            return false;
        }

        // SAFETY: `new_size < old_size`; a valid buffer has at least
        // `old_size + 1` writable bytes, and this exclusive handle permits both
        // writes without forming a reference to C-visible storage.
        unsafe {
            ptr.cast::<u8>().add(new_size).write(0);
            core::ptr::addr_of_mut!((*header).size).write(new_size);
        }
        true
    }

    /// Detach the allocated byte run, leaving this header empty.
    ///
    /// The returned allocation includes the trailing NUL. Static empty
    /// sentinels and null empty buffers have no detachable allocation.
    #[must_use]
    pub fn take_allocation(&mut self) -> Option<GitBufAllocation> {
        let header = self.as_mut_ptr();
        // SAFETY: reads initialized fields through the exclusive handle.
        let (ptr, reserved, size) = unsafe {
            (
                core::ptr::addr_of!((*header).ptr).read().cast::<u8>(),
                core::ptr::addr_of!((*header).reserved).read(),
                core::ptr::addr_of!((*header).size).read(),
            )
        };
        let count = size.checked_add(1)?;
        if ptr.is_null() || reserved == 0 || count > reserved {
            return None;
        }

        // SAFETY: the exclusive handle permits resetting all ownership fields;
        // null/zero is the public `GIT_BUF_INIT` representation.
        unsafe {
            core::ptr::addr_of_mut!((*header).ptr).write(core::ptr::null_mut());
            core::ptr::addr_of_mut!((*header).reserved).write(0);
            core::ptr::addr_of_mut!((*header).size).write(0);
        }

        // SAFETY: the valid allocated-buffer representation proves `count`
        // initialized bytes (content plus NUL), unique ownership moved from the
        // reset header, and compatibility with libgit2's configured allocator.
        unsafe { GitBufAllocation::from_raw_parts(ptr, count) }
    }

    /// Replace the current byte run with a detached libgit2 allocation.
    ///
    /// The replacement must be non-empty and end in NUL. On validation
    /// failure, ownership is returned unchanged in `Err`.
    pub fn replace_allocation(
        &mut self,
        allocation: GitBufAllocation,
    ) -> Result<(), GitBufAllocation> {
        if allocation.as_slice().last() != Some(&0) {
            return Err(allocation);
        }

        let header = self.as_mut_ptr();
        // SAFETY: this live valid header is exclusively borrowed. The C
        // disposer releases its current owned field, preserves the header, and
        // resets all fields before the replacement is installed.
        unsafe { ffi::git_buf_dispose(header) };

        let (ptr, count) = allocation.into_raw_parts();
        // SAFETY: the validated non-empty allocation has `count - 1` content
        // bytes followed by NUL. These writes install its unique ownership in
        // the exclusively borrowed, already-empty header.
        unsafe {
            core::ptr::addr_of_mut!((*header).ptr).write(ptr.cast());
            core::ptr::addr_of_mut!((*header).reserved).write(count);
            core::ptr::addr_of_mut!((*header).size).write(count - 1);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn wrapper_preserves_the_c_buffer_layout() {
        assert_eq!(size_of::<GitBuf>(), size_of::<ffi::git_buf>());
        assert_eq!(align_of::<GitBuf>(), align_of::<ffi::git_buf>());
    }

    #[test]
    fn empty_owned_buffer_has_the_public_init_state() {
        let buffer = GitBuf::new();
        let view = buffer.as_ref();
        assert_eq!(view.size(), 0);
        assert_eq!(view.reserved(), 0);
        assert!(view.contents().is_none());
    }

    #[test]
    fn allocation_can_move_through_the_header_without_double_free() {
        // SAFETY: libgit2 initialization is refcounted and the successful call
        // is balanced by shutdown after all configured allocations are freed.
        let init_count = unsafe { ffi::git_libgit2_init() };
        assert!(init_count > 0);

        // SAFETY: libgit2 is initialized; a non-null result uniquely owns four
        // writable bytes from its configured allocator.
        let raw = unsafe { ffi::crustify_git__malloc(4) }.cast::<u8>();
        assert!(!raw.is_null());
        // SAFETY: `raw` addresses the four writable bytes just allocated.
        unsafe { core::ptr::copy_nonoverlapping(c"abc".as_ptr().cast(), raw, 4) };
        // SAFETY: the four bytes are initialized, uniquely owned, and paired
        // with the libgit2 configured-allocator strategy.
        let allocation = unsafe { GitBufAllocation::from_raw_parts(raw, 4) }.unwrap();

        let mut buffer = GitBuf::new();
        buffer
            .as_mut()
            .replace_allocation(allocation)
            .expect("the replacement ends in NUL");

        let shared = buffer.as_ref();
        let contents = shared.contents().unwrap();
        let mut copied = [0; 3];
        assert!(contents.copy_to_slice(&mut copied));
        assert_eq!(&copied, b"abc");

        let detached = {
            let mut view = buffer.as_mut();
            {
                let mut bytes = view.contents_mut().unwrap();
                assert!(bytes.set_elem(1, b'Z'));
            }
            assert!(view.truncate(2));

            let detached = view.take_allocation().unwrap();
            assert_eq!(detached.as_slice(), b"aZ\0");
            assert_eq!(view.as_ref().size(), 0);
            assert!(view.as_ref().contents().is_none());
            detached
        };
        drop(detached);
        drop(buffer);

        // SAFETY: balances the successful initialization above after all
        // allocations using the configured allocator have been dropped.
        let remaining = unsafe { ffi::git_libgit2_shutdown() };
        assert!(remaining >= 0);
    }
}
