//! Safe wrappers for libgit2 describe APIs.

use core::ffi::CStr;
use core::ptr::{addr_of, addr_of_mut};

use ffibox::define_ctype;

use crate::ffi;

define_ctype!(
    /// Formatting options consumed by libgit2 describe operations.
    ///
    /// Wraps: git_describe_format_options
    DescribeFormatOptions,
    DescribeFormatOptionsRef,
    DescribeFormatOptionsMut,
    ffi::git_describe_format_options
);

impl<'a> DescribeFormatOptionsRef<'a> {
    /// Wraps: git_describe_format_options.version
    /// Returns the ABI version of this options value.
    #[must_use]
    pub fn version(&self) -> u32 {
        // SAFETY: `self` carries a live shared borrow of the complete C
        // object, and `addr_of!` projects the scalar field without forming a
        // reference to C-owned memory.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Wraps: git_describe_format_options.dirty_suffix
    /// Returns the optional suffix borrowed by this options value.
    #[must_use]
    pub fn dirty_suffix(&self) -> Option<&'a CStr> {
        // SAFETY: `self` carries a live shared borrow of the complete C
        // object, and `addr_of!` projects the pointer field without forming a
        // reference to C-owned memory.
        let suffix = unsafe { addr_of!((*self.as_ptr()).dirty_suffix).read() };

        if suffix.is_null() {
            None
        } else {
            // SAFETY: a live `git_describe_format_options` requires its
            // non-null `dirty_suffix` to point to a NUL-terminated string that
            // remains valid while the options value is usable. The result is
            // bounded by the handle's borrow of that value.
            Some(unsafe { CStr::from_ptr(suffix) })
        }
    }

    /// Wraps: git_describe_format_options.always_use_long_format
    /// Returns whether the long form is requested for exact matches.
    #[must_use]
    pub fn always_use_long_format(&self) -> bool {
        // SAFETY: `self` carries a live shared borrow of the complete C
        // object, and `addr_of!` projects the scalar field without forming a
        // reference to C-owned memory.
        unsafe { addr_of!((*self.as_ptr()).always_use_long_format).read() != 0 }
    }

    /// Wraps: git_describe_format_options.abbreviated_size
    /// Returns the lower bound for abbreviated object identifiers.
    #[must_use]
    pub fn abbreviated_size(&self) -> u32 {
        // SAFETY: `self` carries a live shared borrow of the complete C
        // object, and `addr_of!` projects the scalar field without forming a
        // reference to C-owned memory.
        unsafe { addr_of!((*self.as_ptr()).abbreviated_size).read() }
    }
}

impl DescribeFormatOptionsMut<'_> {
    /// Sets the ABI version of this options value.
    pub fn set_version(&mut self, version: u32) {
        // SAFETY: `self` carries an exclusive borrow of the complete C object,
        // and `addr_of_mut!` projects the scalar field without forming a
        // reference to C-owned memory.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Stores an optional borrowed suffix.
    ///
    /// # Safety
    ///
    /// A non-null `suffix` must remain alive and NUL-terminated for every
    /// later use of the options value, including uses after this handle is
    /// released.
    pub unsafe fn set_dirty_suffix(&mut self, suffix: Option<&CStr>) {
        let suffix = suffix.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: `self` carries an exclusive borrow of the complete C object,
        // `addr_of_mut!` forms no reference to C-owned memory, and the caller
        // upholds the stored pointer's lifetime contract.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).dirty_suffix).write(suffix) }
    }

    /// Clears the optional borrowed suffix.
    pub fn clear_dirty_suffix(&mut self) {
        // SAFETY: storing null creates no borrowed-pointer lifetime
        // obligation.
        unsafe { self.set_dirty_suffix(None) }
    }

    /// Selects whether exact matches use the long format.
    pub fn set_always_use_long_format(&mut self, always: bool) {
        // SAFETY: `self` carries an exclusive borrow of the complete C object,
        // and `addr_of_mut!` projects the scalar field without forming a
        // reference to C-owned memory.
        unsafe {
            addr_of_mut!((*self.as_mut_ptr()).always_use_long_format).write(i32::from(always))
        }
    }

    /// Sets the lower bound for abbreviated object identifiers.
    pub fn set_abbreviated_size(&mut self, size: u32) {
        // SAFETY: `self` carries an exclusive borrow of the complete C object,
        // and `addr_of_mut!` projects the scalar field without forming a
        // reference to C-owned memory.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).abbreviated_size).write(size) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn describe_format_options_preserve_the_c_layout() {
        assert_eq!(
            size_of::<DescribeFormatOptions>(),
            size_of::<ffi::git_describe_format_options>()
        );
        assert_eq!(
            align_of::<DescribeFormatOptions>(),
            align_of::<ffi::git_describe_format_options>()
        );
    }

    #[test]
    fn borrowed_handles_read_and_write_every_field() {
        let mut options = DescribeFormatOptions::zeroed();
        let raw = addr_of_mut!(options).cast::<ffi::git_describe_format_options>();

        // SAFETY: `raw` points to the live, initialized, layout-compatible
        // stack value above, and this is its only active handle.
        let mut options = unsafe { DescribeFormatOptionsMut::from_ptr(raw) }
            .expect("the address of a stack value is non-null");

        options.set_version(1);
        options.set_abbreviated_size(12);
        options.set_always_use_long_format(true);
        // SAFETY: this string literal has static storage and remains valid for
        // every later use of the stack options value.
        unsafe { options.set_dirty_suffix(Some(c"-dirty")) };

        let shared = options.as_ref();
        assert_eq!(shared.version(), 1);
        assert_eq!(shared.abbreviated_size(), 12);
        assert!(shared.always_use_long_format());
        assert_eq!(shared.dirty_suffix(), Some(c"-dirty"));

        options.clear_dirty_suffix();
        assert_eq!(options.as_ref().dirty_suffix(), None);
    }
}
