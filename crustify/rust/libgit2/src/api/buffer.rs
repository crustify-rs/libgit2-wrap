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
    /// Field: git_buf.size
    /// Number of initialized content bytes, excluding the trailing NUL.
    #[inline]
    #[must_use]
    pub fn size(&self) -> usize {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).size).read() }
    }

    /// Field: git_buf.ptr
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

    /// Field: git_buf.reserved
    /// Allocation capacity recorded by libgit2.
    ///
    /// The public header describes this field as reserved and unused, but the
    /// implementation records the allocation size in it: `git_buf_fromstr`
    /// copies `git_str::asize` here and `git_buf_grow` stores the grown
    /// capacity. Observed behaviour governs, so the exclusive operations on
    /// [`GitBufMut`] use it as the owned-allocation witness.
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
    /// Libgit2's own owned-allocation witness for a buffer header.
    ///
    /// `git_str_is_allocated` requires a non-null pointer and a non-zero
    /// `asize`, which `git_buf_fromstr` copies into `reserved`; an allocated
    /// buffer always reserves room for its trailing NUL, so `reserved > size`.
    /// The two representations this rejects are not writable: a header
    /// attached to bytes libgit2 does not own records `reserved == 0` with a
    /// non-null pointer (`git_str_attach_notowned`, which `git_str_try_grow`
    /// then refuses as "a borrowed buffer"), and a disposed or sanitized
    /// buffer points at the shared `git_str__initstr` sentinel with
    /// `reserved == 0`. Every exclusive operation below is therefore gated on
    /// this witness rather than on a non-null pointer alone.
    ///
    /// Returns the content pointer and the content length.
    fn allocation(&self) -> Option<(NonNull<u8>, usize)> {
        let header = self.as_ref().as_ptr();
        // SAFETY: the three fields are initialized members of the live header
        // borrowed by this handle; raw-place projection reads them without
        // forming a reference to C-visible storage.
        let (ptr, reserved, size) = unsafe {
            (
                core::ptr::addr_of!((*header).ptr).read().cast::<u8>(),
                core::ptr::addr_of!((*header).reserved).read(),
                core::ptr::addr_of!((*header).size).read(),
            )
        };
        let ptr = NonNull::new(ptr)?;
        if reserved > size {
            Some((ptr, size))
        } else {
            None
        }
    }

    /// Borrow the initialized content bytes exclusively.
    ///
    /// `None` denotes a header that owns no writable allocation: the null and
    /// static-sentinel empty representations, and a header merely attached to
    /// bytes owned elsewhere.
    #[must_use]
    pub fn contents_mut(&mut self) -> Option<CSliceMut<'_, u8>> {
        let (ptr, size) = self.allocation()?;
        // SAFETY: the owned-allocation witness proves `size` initialized bytes
        // within a writable allocation at `ptr`, and the mutable handle
        // provides exclusive access for the returned view's lifetime.
        Some(unsafe { CSliceMut::from_raw_parts(ptr, size) })
    }

    /// Shrink the initialized contents and move the trailing NUL terminator.
    /// Returns `false` if `new_size` would grow the buffer, or if the header
    /// owns no writable allocation and the request is not already satisfied.
    pub fn truncate(&mut self, new_size: usize) -> bool {
        let Some((ptr, old_size)) = self.allocation() else {
            // Without a writable allocation only a no-op truncation succeeds:
            // there is no terminator to move and nothing to shrink.
            return new_size == self.as_ref().size();
        };

        if new_size > old_size {
            return false;
        }
        if new_size == old_size {
            return true;
        }

        // SAFETY: `new_size < old_size < reserved`, so the terminator write
        // stays inside the allocation proved by `allocation`, and this
        // exclusive handle permits both writes without forming a reference to
        // C-visible storage.
        unsafe {
            ptr.as_ptr().add(new_size).write(0);
            core::ptr::addr_of_mut!((*self.as_mut_ptr()).size).write(new_size);
        }
        true
    }

    /// Detach the allocated byte run, leaving this header empty.
    ///
    /// The returned allocation includes the trailing NUL. Static empty
    /// sentinels, null empty buffers, and headers attached to bytes owned
    /// elsewhere have no detachable allocation.
    #[must_use]
    pub fn take_allocation(&mut self) -> Option<GitBufAllocation> {
        let (ptr, size) = self.allocation()?;
        // `reserved > size` bounds the sum, so this cannot overflow.
        let count = size.checked_add(1)?;

        let header = self.as_mut_ptr();
        // SAFETY: the exclusive handle permits resetting all ownership fields;
        // null/zero is the public `GIT_BUF_INIT` representation.
        unsafe {
            core::ptr::addr_of_mut!((*header).ptr).write(core::ptr::null_mut());
            core::ptr::addr_of_mut!((*header).reserved).write(0);
            core::ptr::addr_of_mut!((*header).size).write(0);
        }

        // SAFETY: the owned-allocation witness proves `count` initialized
        // bytes (content plus NUL), unique ownership moved out of the reset
        // header, and compatibility with libgit2's configured allocator.
        unsafe { GitBufAllocation::from_raw_parts(ptr.as_ptr(), count) }
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
    fn a_borrowed_header_exposes_no_writable_allocation() {
        // The representation `git_str_attach_notowned` produces and
        // `git_buf_fromstr` forwards: non-null content bytes libgit2 does not
        // own, recorded with `reserved == 0`.
        let borrowed = *b"static";
        let mut raw = ffi::git_buf {
            ptr: borrowed.as_ptr().cast_mut().cast(),
            reserved: 0,
            size: borrowed.len(),
        };

        // SAFETY: `raw` is a live initialized header for this scope and no
        // other handle addresses it.
        let mut header = unsafe { GitBufMut::from_ptr(&raw mut raw) }.unwrap();

        // Reads are still available; writes are refused.
        assert_eq!(header.as_ref().size(), borrowed.len());
        assert_eq!(header.as_ref().contents().unwrap().elem(0), Some(b's'));
        assert!(header.contents_mut().is_none());
        assert!(header.take_allocation().is_none());
        assert!(!header.truncate(2));
        assert!(header.truncate(borrowed.len()));

        // The refused truncation left the borrowed bytes untouched.
        assert_eq!(&borrowed, b"static");
        assert_eq!(raw.size, 6);
    }

    #[test]
    fn the_empty_sentinel_header_truncates_only_to_zero() {
        let mut buffer = GitBuf::new();
        let mut view = buffer.as_mut();
        assert!(view.truncate(0));
        assert!(!view.truncate(1));
        assert!(view.contents_mut().is_none());
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
