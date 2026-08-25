//! Safe wrappers for libgit2 indexer APIs.

use crate::ffi;
use crate::indexer::{GitIndexerProgressCallback, IndexerProgressRef};
use crate::odb::{GitOdbMut, GitOdbRef};
use crate::oid::{InvalidOidType, OidType};
use core::ptr::{addr_of, addr_of_mut};

ffibox::define_ctype!(
    /// Wraps: git_indexer_options
    /// Layout-compatible options used to construct a packfile indexer.
    /// Borrowed databases and callback state must outlive constructed indexers.
    GitIndexerOptions, GitIndexerOptionsRef, GitIndexerOptionsMut,
    ffi::git_indexer_options
);

impl<'a> GitIndexerOptionsRef<'a> {
    /// Field: git_indexer_options.version
    #[must_use]
    pub fn version(&self) -> u32 {
        // SAFETY: the live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_indexer_options.mode
    #[must_use]
    pub fn mode(&self) -> u32 {
        // SAFETY: the live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).mode).read() }
    }

    /// Field: git_indexer_options.oid_type
    pub fn oid_type(&self) -> Result<Option<OidType>, InvalidOidType> {
        // SAFETY: the live shared handle permits a raw-place scalar read.
        let value = unsafe { addr_of!((*self.as_ptr()).oid_type).read() };
        if value == 0 {
            Ok(None)
        } else {
            OidType::try_from(value).map(Some)
        }
    }

    /// Field: git_indexer_options.odb
    /// Borrows the optional object database installed in these options.
    #[must_use]
    pub fn odb(&self) -> Option<GitOdbRef<'a>> {
        // SAFETY: raw-place projection copies the initialized pointer.
        let odb = unsafe { addr_of!((*self.as_ptr()).odb).read() };
        // SAFETY: the options contract keeps a non-null database live.
        unsafe { GitOdbRef::from_ptr(odb) }
    }

    /// Field: git_indexer_options.verify
    #[must_use]
    pub fn verify(&self) -> bool {
        // SAFETY: the live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).verify).read() != 0 }
    }

    /// Field: git_indexer_options.progress_cb
    #[must_use]
    pub fn has_progress_callback(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized callback slot.
        unsafe { addr_of!((*self.as_ptr()).progress_cb).read().is_some() }
    }

    /// Field: git_indexer_options.progress_cb_payload
    #[must_use]
    pub fn has_progress_callback_payload(&self) -> bool {
        // SAFETY: raw-place projection copies the initialized opaque pointer.
        !unsafe { addr_of!((*self.as_ptr()).progress_cb_payload).read() }.is_null()
    }
}

impl GitIndexerOptionsMut<'_> {
    /// Sets the options ABI version.
    pub fn set_version(&mut self, value: u32) {
        // SAFETY: the exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(value) }
    }

    /// Sets packfile permissions, or zero for defaults.
    pub fn set_mode(&mut self, value: u32) {
        // SAFETY: the exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).mode).write(value) }
    }

    /// Selects an object-ID format, or the default when `None`.
    pub fn set_oid_type(&mut self, value: Option<OidType>) {
        let value = value.map_or(0, ffi::git_oid_t::from);
        // SAFETY: the handle permits the write and the value is valid or zero.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).oid_type).write(value) }
    }

    /// Borrows the optional database exclusively through these options.
    #[must_use]
    pub fn odb_mut(&mut self) -> Option<GitOdbMut<'_>> {
        // SAFETY: this exclusive handle permits copying the pointer.
        let odb = unsafe { addr_of!((*self.as_mut_ptr()).odb).read() };
        // SAFETY: the returned handle is bounded by this exclusive reborrow.
        unsafe { GitOdbMut::from_ptr(odb) }
    }

    /// Stores an optional borrowed object database.
    ///
    /// # Safety
    /// A non-null database must remain live and unaliased for every later use
    /// of this options value — including [`odb`](GitIndexerOptionsRef::odb),
    /// which hands the stored pointer back as a borrowed handle — and until
    /// every indexer constructed from these options is destroyed. libgit2
    /// copies the pointer without up-referencing it.
    pub unsafe fn set_odb(&mut self, mut odb: Option<GitOdbMut<'_>>) {
        let odb = odb
            .as_mut()
            .map_or(core::ptr::null_mut(), GitOdbMut::as_mut_ptr);
        // SAFETY: the caller supplies the external lifetime and aliasing guarantee.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).odb).write(odb) }
    }

    /// Sets whether connectivity verification is requested.
    pub fn set_verify(&mut self, value: bool) {
        // SAFETY: the exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).verify).write(u8::from(value)) }
    }

    /// Installs a typed Rust progress callback and its erased payload.
    ///
    /// # Safety
    /// `callback` must remain live and exclusively reserved until every
    /// indexer constructed from these options is destroyed. Invocations must
    /// not overlap unless the callback synchronizes its own state.
    pub unsafe fn set_progress_callback<C: GitIndexerProgressCallback>(
        &mut self,
        callback: &mut C,
    ) {
        unsafe extern "C" fn trampoline<C: GitIndexerProgressCallback>(
            progress: *const ffi::git_indexer_progress,
            payload: *mut core::ffi::c_void,
        ) -> core::ffi::c_int {
            // SAFETY: libgit2 supplies a live transient progress record.
            let Some(progress) = (unsafe { IndexerProgressRef::from_ptr(progress.cast_mut()) })
            else {
                return -1;
            };
            // SAFETY: the installation contract keeps this callback live and exclusive.
            let Some(callback) = (unsafe { payload.cast::<C>().as_mut() }) else {
                return -1;
            };
            callback.call(progress)
        }
        let options = self.as_mut_ptr();
        // SAFETY: the caller guarantees payload validity and this handle permits both writes.
        unsafe {
            addr_of_mut!((*options).progress_cb).write(Some(trampoline::<C>));
            addr_of_mut!((*options).progress_cb_payload)
                .write(core::ptr::from_mut(callback).cast());
        }
    }

    /// Clears the progress callback and its payload together.
    pub fn clear_progress_callback(&mut self) {
        let options = self.as_mut_ptr();
        // SAFETY: the exclusive handle permits both field writes.
        unsafe {
            addr_of_mut!((*options).progress_cb).write(None);
            addr_of_mut!((*options).progress_cb_payload).write(core::ptr::null_mut());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};
    use ffibox::CCell;

    #[test]
    fn wrapper_and_handles_match_the_c_seam() {
        fn assert_cell<T: CCell>() {}
        assert_cell::<GitIndexerOptions>();
        assert_eq!(
            size_of::<GitIndexerOptions>(),
            size_of::<ffi::git_indexer_options>()
        );
        assert_eq!(
            align_of::<GitIndexerOptions>(),
            align_of::<ffi::git_indexer_options>()
        );
        assert_eq!(
            size_of::<GitIndexerOptionsRef<'_>>(),
            size_of::<*const ffi::git_indexer_options>()
        );
        assert_eq!(
            size_of::<GitIndexerOptionsMut<'_>>(),
            size_of::<*mut ffi::git_indexer_options>()
        );
    }

    #[test]
    fn handles_read_and_write_all_scalar_fields() {
        let mut raw = GitIndexerOptions::zeroed();
        // SAFETY: raw remains live and exclusively accessed by the handle.
        let mut options =
            unsafe { GitIndexerOptionsMut::from_ptr(addr_of_mut!(raw).cast()).unwrap() };
        options.set_version(1);
        options.set_mode(0o640);
        options.set_oid_type(Some(OidType::Sha256));
        options.set_verify(true);
        assert_eq!(options.as_ref().version(), 1);
        assert_eq!(options.as_ref().mode(), 0o640);
        assert_eq!(options.as_ref().oid_type(), Ok(Some(OidType::Sha256)));
        assert!(options.as_ref().verify());
        assert!(options.as_ref().odb().is_none());
        assert!(!options.as_ref().has_progress_callback());
        assert!(!options.as_ref().has_progress_callback_payload());
    }

    #[test]
    fn callback_pair_uses_the_typed_callable_surface() {
        let mut raw = GitIndexerOptions::zeroed();
        // SAFETY: raw remains live and exclusively accessed by the handle.
        let mut options =
            unsafe { GitIndexerOptionsMut::from_ptr(addr_of_mut!(raw).cast()).unwrap() };
        let mut count = 0u32;
        let mut callback = |progress: IndexerProgressRef<'_>| {
            count = progress.total_objects();
            7
        };
        // SAFETY: callback remains live and reserved until the pair is cleared.
        unsafe { options.set_progress_callback(&mut callback) };
        assert!(options.as_ref().has_progress_callback());
        assert!(options.as_ref().has_progress_callback_payload());
        let progress = ffi::git_indexer_progress {
            total_objects: 17,
            indexed_objects: 0,
            received_objects: 0,
            local_objects: 0,
            total_deltas: 0,
            indexed_deltas: 0,
            received_bytes: 0,
        };
        let options_ptr = options.as_ref().as_ptr();
        // SAFETY: raw-place reads copy the coherently installed callback pair.
        let function = unsafe { addr_of!((*options_ptr).progress_cb).read() }.unwrap();
        // SAFETY: as above; the opaque pointer is not dereferenced here.
        let payload = unsafe { addr_of!((*options_ptr).progress_cb_payload).read() };
        // SAFETY: `progress` and the installed callback pair remain live and
        // the callback is exclusively reserved for this invocation.
        assert_eq!(unsafe { function(addr_of!(progress), payload) }, 7);
        options.clear_progress_callback();
        assert!(!options.as_ref().has_progress_callback());
        assert_eq!(count, 17);
    }
}
