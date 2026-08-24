//! Safe wrappers for libgit2 revparse APIs.

use core::ops::{BitOr, BitOrAssign};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CVal, CValued};

use crate::ffi;
use crate::object::{GitObjectOwned, GitObjectRef};

ffibox::define_ctype!(
    /// Wraps: git_revspec
    /// A caller-allocated revision parse result that owns its object results.
    GitRevspec,
    GitRevspecRef,
    GitRevspecMut,
    ffi::git_revspec
);

/// A by-value revision parse result whose object fields are released on drop.
pub type GitRevspecOwned = CVal<GitRevspec>;

/// Wraps: git_revspec.flags
/// The intent and range semantics reported for a parsed revision expression.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GitRevspecFlags(u32);

impl GitRevspecFlags {
    /// No parse intent has been recorded yet.
    pub const EMPTY: Self = Self(0);
    /// The expression identifies one object.
    pub const SINGLE: Self = Self(ffi::git_revspec_t_GIT_REVSPEC_SINGLE);
    /// The expression identifies a range.
    pub const RANGE: Self = Self(ffi::git_revspec_t_GIT_REVSPEC_RANGE);
    /// The range uses symmetric-difference merge-base semantics.
    pub const MERGE_BASE: Self = Self(ffi::git_revspec_t_GIT_REVSPEC_MERGE_BASE);
    /// Every flag published by libgit2.
    pub const ALL: Self = Self(Self::SINGLE.0 | Self::RANGE.0 | Self::MERGE_BASE.0);

    /// Converts raw bits when every set bit is published by libgit2.
    #[must_use]
    pub const fn from_bits(bits: u32) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying C bit set.
    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Returns whether no flag is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every bit in `other` is set.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for GitRevspecFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitRevspecFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Raw `git_revspec.flags` bits not published by libgit2.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitRevspecFlags(u32);

impl InvalidGitRevspecFlags {
    /// Returns the unrecognized bit set.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }
}

impl GitRevspec {
    /// Constructs an empty result ready for `git_revparse` to fill.
    #[must_use]
    pub fn new() -> GitRevspecOwned {
        // The C empty representation consists of null pointers and zero flags.
        CVal::new(Self::zeroed())
    }
}

impl<'a> GitRevspecRef<'a> {
    /// Returns the validated parse-intent flags.
    pub fn flags(&self) -> Result<GitRevspecFlags, InvalidGitRevspecFlags> {
        // SAFETY: this live shared handle permits a raw-place read of the
        // initialized scalar without forming a reference to C-visible memory.
        let bits = unsafe { addr_of!((*self.as_ptr()).flags).read() };
        GitRevspecFlags::from_bits(bits).ok_or(InvalidGitRevspecFlags(bits))
    }

    /// Wraps: git_revspec.from
    /// Borrows the owned left-hand object, when one is present.
    #[must_use]
    pub fn from(&self) -> Option<GitObjectRef<'a>> {
        // SAFETY: raw-place projection reads the initialized pointer field
        // without forming a reference to the result header.
        let object = unsafe { addr_of!((*self.as_ptr()).from).read() };
        // SAFETY: a non-null object is live and owned by this result, so this
        // shared view remains valid for the handle's lifetime.
        unsafe { GitObjectRef::from_ptr(object) }
    }

    /// Wraps: git_revspec.to
    /// Borrows the owned right-hand object, when the expression is a range.
    #[must_use]
    pub fn to(&self) -> Option<GitObjectRef<'a>> {
        // SAFETY: raw-place projection reads the initialized pointer field
        // without forming a reference to the result header.
        let object = unsafe { addr_of!((*self.as_ptr()).to).read() };
        // SAFETY: as `from`, for the independently owned right-hand object.
        unsafe { GitObjectRef::from_ptr(object) }
    }
}

impl GitRevspecMut<'_> {
    /// Sets the parse-intent flags.
    pub fn set_flags(&mut self, flags: GitRevspecFlags) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags.bits()) }
    }

    /// Moves the left-hand object out, leaving that field empty.
    #[must_use]
    pub fn take_from(&mut self) -> Option<GitObjectOwned> {
        let result = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits reading and clearing the
        // owned field as one transfer, leaving a valid null representation.
        let object = unsafe {
            let object = addr_of!((*result).from).read();
            addr_of_mut!((*result).from).write(core::ptr::null_mut());
            object
        };
        // SAFETY: a non-null pointer moved out of a valid revspec carries one
        // independently owned libgit2 object reference.
        unsafe { GitObjectOwned::from_raw(object) }
    }

    /// Replaces the left-hand object and releases the previous one.
    pub fn set_from(&mut self, object: Option<GitObjectOwned>) {
        let object = object.map_or(core::ptr::null_mut(), GitObjectOwned::into_raw);
        let old = self.take_from();
        // SAFETY: the exclusive handle permits installing the transferred
        // compatible object reference after the old field was cleared.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).from).write(object) }
        drop(old);
    }

    /// Moves the right-hand object out, leaving that field empty.
    #[must_use]
    pub fn take_to(&mut self) -> Option<GitObjectOwned> {
        let result = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits reading and clearing the
        // independently owned field as one transfer.
        let object = unsafe {
            let object = addr_of!((*result).to).read();
            addr_of_mut!((*result).to).write(core::ptr::null_mut());
            object
        };
        // SAFETY: a non-null pointer moved out of a valid revspec carries one
        // independently owned libgit2 object reference.
        unsafe { GitObjectOwned::from_raw(object) }
    }

    /// Replaces the right-hand object and releases the previous one.
    pub fn set_to(&mut self, object: Option<GitObjectOwned>) {
        let object = object.map_or(core::ptr::null_mut(), GitObjectOwned::into_raw);
        let old = self.take_to();
        // SAFETY: the exclusive handle permits installing the transferred
        // compatible object reference after the old field was cleared.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).to).write(object) }
        drop(old);
    }
}

// SAFETY: a valid `GitRevspec` owns each non-null object field independently.
// Disposing the by-value header releases exactly those references and clears
// both fields while retaining the header storage.
unsafe impl CValued for GitRevspec {
    unsafe fn c_dispose(this: NonNull<Self>) {
        let result = this.as_ptr().cast::<ffi::git_revspec>();
        // SAFETY: the `CValued` contract grants exclusive teardown access to
        // the live header. Reading and clearing both fields transfers their
        // unique references into the temporary owners below.
        let (from, to) = unsafe {
            let from = addr_of!((*result).from).read();
            let to = addr_of!((*result).to).read();
            addr_of_mut!((*result).from).write(core::ptr::null_mut());
            addr_of_mut!((*result).to).write(core::ptr::null_mut());
            (from, to)
        };
        // SAFETY: every non-null pointer came from one owned field of this
        // valid revspec and is released exactly once after being cleared.
        let from = unsafe { GitObjectOwned::from_raw(from) };
        // SAFETY: as above, for the independent right-hand object field.
        let to = unsafe { GitObjectOwned::from_raw(to) };
        drop((from, to));
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use super::*;

    #[test]
    fn revspec_wrapper_preserves_layout_and_value_ownership() {
        fn assert_valued<T: CValued>() {}
        assert_valued::<GitRevspec>();
        assert_eq!(size_of::<GitRevspec>(), size_of::<ffi::git_revspec>());
        assert_eq!(align_of::<GitRevspec>(), align_of::<ffi::git_revspec>());
        assert_eq!(
            size_of::<GitRevspecRef<'_>>(),
            size_of::<*const ffi::git_revspec>()
        );
        assert_eq!(
            size_of::<GitRevspecMut<'_>>(),
            size_of::<*mut ffi::git_revspec>()
        );
        assert_eq!(size_of::<GitRevspecOwned>(), size_of::<ffi::git_revspec>());

        let empty = GitRevspec::new();
        assert!(empty.as_ref().from().is_none());
        assert!(empty.as_ref().to().is_none());
        assert_eq!(empty.as_ref().flags(), Ok(GitRevspecFlags::EMPTY));
    }

    #[test]
    fn borrowed_revspec_handles_project_objects_and_validate_flags() {
        let object = Box::new(MaybeUninit::<ffi::git_object>::zeroed());
        let object = Box::into_raw(object).cast::<ffi::git_object>();
        let mut raw = ffi::git_revspec {
            from: object,
            to: core::ptr::null_mut(),
            flags: GitRevspecFlags::SINGLE.bits(),
        };

        {
            // SAFETY: `raw` and the opaque object storage remain live, and
            // this scope has exclusive access to the revspec header.
            let mut revspec = unsafe { GitRevspecMut::from_ptr(&raw mut raw) }.unwrap();
            assert_eq!(revspec.as_ref().from().unwrap().as_ptr(), object);
            assert!(revspec.as_ref().to().is_none());
            assert_eq!(revspec.as_ref().flags(), Ok(GitRevspecFlags::SINGLE));

            revspec.set_flags(GitRevspecFlags::RANGE | GitRevspecFlags::MERGE_BASE);
            assert_eq!(
                revspec.as_ref().flags(),
                Ok(GitRevspecFlags::RANGE | GitRevspecFlags::MERGE_BASE)
            );
        }
        raw.flags = GitRevspecFlags::ALL.bits() << 1;
        {
            // SAFETY: the exclusive handle was released; `raw` remains live
            // and is now only shared for this handle's lifetime.
            let revspec = unsafe { GitRevspecRef::from_ptr(&raw mut raw) }.unwrap();
            assert_eq!(
                revspec.flags().unwrap_err().value(),
                GitRevspecFlags::ALL.bits() << 1
            );
        }
        // SAFETY: no handle retains the object pointer, and this recovers the
        // exact allocation originally produced by `Box::into_raw`.
        drop(unsafe { Box::from_raw(object.cast::<MaybeUninit<ffi::git_object>>()) });
    }
}
