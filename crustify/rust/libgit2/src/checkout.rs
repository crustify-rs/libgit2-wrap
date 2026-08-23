//! Safe wrappers for libgit2 checkout APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::ffi;

/// Wraps: git_checkout_notify_t
///
/// A layout-compatible set of checkout notification flags. Unknown bits are
/// retained so values introduced by a newer libgit2 remain safe to carry.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct CheckoutNotify(ffi::git_checkout_notify_t);

impl CheckoutNotify {
    /// No checkout notifications.
    pub const NONE: Self = Self(ffi::git_checkout_notify_t_GIT_CHECKOUT_NOTIFY_NONE);
    /// A conflict was found.
    pub const CONFLICT: Self = Self(ffi::git_checkout_notify_t_GIT_CHECKOUT_NOTIFY_CONFLICT);
    /// A dirty file was found.
    pub const DIRTY: Self = Self(ffi::git_checkout_notify_t_GIT_CHECKOUT_NOTIFY_DIRTY);
    /// A file was updated.
    pub const UPDATED: Self = Self(ffi::git_checkout_notify_t_GIT_CHECKOUT_NOTIFY_UPDATED);
    /// An untracked file was found.
    pub const UNTRACKED: Self = Self(ffi::git_checkout_notify_t_GIT_CHECKOUT_NOTIFY_UNTRACKED);
    /// An ignored file was found.
    pub const IGNORED: Self = Self(ffi::git_checkout_notify_t_GIT_CHECKOUT_NOTIFY_IGNORED);
    /// All notification classes recognized by libgit2.
    pub const ALL: Self = Self(ffi::git_checkout_notify_t_GIT_CHECKOUT_NOTIFY_ALL);

    /// Retain all bits from a raw libgit2 value, including unknown bits.
    #[inline]
    #[must_use]
    pub const fn from_bits_retain(bits: ffi::git_checkout_notify_t) -> Self {
        Self(bits)
    }

    /// Return the raw libgit2 flag bits.
    #[inline]
    #[must_use]
    pub const fn bits(self) -> ffi::git_checkout_notify_t {
        self.0
    }

    /// Whether every bit in `other` is present.
    #[inline]
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether at least one bit in `other` is present.
    #[inline]
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Whether no notification bits are set.
    #[inline]
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

impl From<ffi::git_checkout_notify_t> for CheckoutNotify {
    #[inline]
    fn from(bits: ffi::git_checkout_notify_t) -> Self {
        Self::from_bits_retain(bits)
    }
}

impl From<CheckoutNotify> for ffi::git_checkout_notify_t {
    #[inline]
    fn from(flags: CheckoutNotify) -> Self {
        flags.bits()
    }
}

impl BitOr for CheckoutNotify {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for CheckoutNotify {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for CheckoutNotify {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for CheckoutNotify {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for CheckoutNotify {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self(self.0 ^ Self::ALL.0)
    }
}

ffibox::define_ctype!(
    /// Wraps: git_checkout_perfdata
    CheckoutPerfData,
    CheckoutPerfDataRef,
    CheckoutPerfDataMut,
    ffi::git_checkout_perfdata
);

impl CheckoutPerfDataRef<'_> {
    /// Wraps: git_checkout_perfdata.stat_calls
    #[inline]
    #[must_use]
    pub fn stat_calls(&self) -> usize {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).stat_calls).read() }
    }

    /// Wraps: git_checkout_perfdata.chmod_calls
    #[inline]
    #[must_use]
    pub fn chmod_calls(&self) -> usize {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).chmod_calls).read() }
    }

    /// Wraps: git_checkout_perfdata.mkdir_calls
    #[inline]
    #[must_use]
    pub fn mkdir_calls(&self) -> usize {
        let ptr = self.as_ptr();
        // SAFETY: `ptr` comes from this live shared handle; raw-place
        // projection reads the initialized scalar without forming a reference.
        unsafe { core::ptr::addr_of!((*ptr).mkdir_calls).read() }
    }
}

impl CheckoutPerfDataMut<'_> {
    /// Set the number of `stat` calls.
    #[inline]
    pub fn set_stat_calls(&mut self, value: usize) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).stat_calls).write(value) }
    }

    /// Set the number of `chmod` calls.
    #[inline]
    pub fn set_chmod_calls(&mut self, value: usize) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).chmod_calls).write(value) }
    }

    /// Set the number of `mkdir` calls.
    #[inline]
    pub fn set_mkdir_calls(&mut self, value: usize) {
        let ptr = self.as_mut_ptr();
        // SAFETY: `ptr` comes from this exclusive handle; raw-place projection
        // writes the scalar without forming a reference to C-visible memory.
        unsafe { core::ptr::addr_of_mut!((*ptr).mkdir_calls).write(value) }
    }
}

/// Wraps: git_checkout_progress_cb
/// Safe callable surface for checkout progress notifications.
pub trait GitCheckoutProgressCallback {
    /// Reports progress; `path == None` is the initial zero-step baseline.
    fn call(&mut self, path: Option<&core::ffi::CStr>, completed: usize, total: usize);
}

impl<F> GitCheckoutProgressCallback for F
where
    F: FnMut(Option<&core::ffi::CStr>, usize, usize),
{
    fn call(&mut self, path: Option<&core::ffi::CStr>, completed: usize, total: usize) {
        self(path, completed, total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_flags_combine_and_retain_unknown_bits() {
        let flags = CheckoutNotify::CONFLICT | CheckoutNotify::UPDATED;
        assert!(flags.contains(CheckoutNotify::CONFLICT));
        assert!(flags.intersects(CheckoutNotify::UPDATED));
        assert!(!flags.intersects(CheckoutNotify::DIRTY));

        let unknown = CheckoutNotify::from_bits_retain(1 << 31);
        assert_eq!(unknown.bits(), 1 << 31);
        assert_eq!(ffi::git_checkout_notify_t::from(unknown), 1 << 31);
    }

    #[test]
    fn perfdata_handles_read_and_write_all_fields() {
        let mut raw = ffi::git_checkout_perfdata {
            mkdir_calls: 1,
            stat_calls: 2,
            chmod_calls: 3,
        };

        // SAFETY: `raw` is initialized, non-null, exclusively borrowed for the
        // handle's lifetime, and remains live until the handle is last used.
        let mut data = unsafe { CheckoutPerfDataMut::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(data.as_ref().mkdir_calls(), 1);
        assert_eq!(data.as_ref().stat_calls(), 2);
        assert_eq!(data.as_ref().chmod_calls(), 3);

        data.set_mkdir_calls(4);
        data.set_stat_calls(5);
        data.set_chmod_calls(6);
        assert_eq!(data.as_ref().mkdir_calls(), 4);
        assert_eq!(data.as_ref().stat_calls(), 5);
        assert_eq!(data.as_ref().chmod_calls(), 6);
    }

    #[test]
    fn progress_callback_accepts_the_null_baseline_path() {
        let mut seen = None;
        let mut callback = |path: Option<&core::ffi::CStr>, completed, total| {
            seen = Some((path.is_none(), completed, total));
        };
        GitCheckoutProgressCallback::call(&mut callback, None, 0, 12);
        assert_eq!(seen, Some((true, 0, 12)));
    }

    #[test]
    fn perfdata_wrapper_preserves_c_layout() {
        assert_eq!(
            core::mem::size_of::<CheckoutPerfData>(),
            core::mem::size_of::<ffi::git_checkout_perfdata>()
        );
        assert_eq!(
            core::mem::align_of::<CheckoutPerfData>(),
            core::mem::align_of::<ffi::git_checkout_perfdata>()
        );
    }
}
