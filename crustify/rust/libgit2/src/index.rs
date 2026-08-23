//! Safe wrappers for libgit2 index APIs.

use crate::ffi;

ffibox::define_ctype!(
    /// Wraps: git_index_time
    /// A seconds-and-nanoseconds timestamp stored in an index entry.
    IndexTime,
    IndexTimeRef,
    IndexTimeMut,
    ffi::git_index_time
);

impl IndexTimeRef<'_> {
    /// Wraps: git_index_time.seconds
    /// Returns the whole-second component of this timestamp.
    #[inline]
    #[must_use]
    pub fn seconds(&self) -> i32 {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).seconds).read() }
    }

    /// Wraps: git_index_time.nanoseconds
    /// Returns the subsecond nanosecond component of this timestamp.
    #[inline]
    #[must_use]
    pub fn nanoseconds(&self) -> u32 {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).nanoseconds).read() }
    }
}

impl IndexTimeMut<'_> {
    /// Sets the whole-second component of this timestamp.
    #[inline]
    pub fn set_seconds(&mut self, value: i32) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).seconds).write(value) }
    }

    /// Sets the subsecond nanosecond component of this timestamp.
    #[inline]
    pub fn set_nanoseconds(&mut self, value: u32) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).nanoseconds).write(value) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn index_time_wrapper_preserves_the_c_layout() {
        assert_eq!(size_of::<IndexTime>(), size_of::<ffi::git_index_time>());
        assert_eq!(align_of::<IndexTime>(), align_of::<ffi::git_index_time>());
        assert_eq!(
            size_of::<IndexTimeRef<'_>>(),
            size_of::<*const ffi::git_index_time>()
        );
        assert_eq!(
            size_of::<IndexTimeMut<'_>>(),
            size_of::<*mut ffi::git_index_time>()
        );
    }

    #[test]
    fn borrowed_handles_read_and_write_both_components() {
        let mut raw = ffi::git_index_time {
            seconds: -2,
            nanoseconds: 3,
        };

        // SAFETY: `raw` is initialized, non-null, exclusively borrowed for the
        // handle's lifetime, and remains live until the handle is last used.
        let mut time = unsafe { IndexTimeMut::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");
        assert_eq!(time.as_ref().seconds(), -2);
        assert_eq!(time.as_ref().nanoseconds(), 3);

        time.set_seconds(4);
        time.set_nanoseconds(5);
        assert_eq!(time.as_ref().seconds(), 4);
        assert_eq!(time.as_ref().nanoseconds(), 5);
    }
}
