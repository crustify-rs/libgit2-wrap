//! Safe wrappers for libgit2 config APIs.

use core::ffi::CStr;
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CVal, CValued};

use crate::ffi;

/// Wraps: git_configmap_t
/// The conversion applied when matching a textual configuration value.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GitConfigmapType {
    /// Parse and match a false Boolean value.
    False = ffi::git_configmap_t_GIT_CONFIGMAP_FALSE,
    /// Parse and match a true Boolean value.
    True = ffi::git_configmap_t_GIT_CONFIGMAP_TRUE,
    /// Parse the value as a signed 32-bit integer.
    Int32 = ffi::git_configmap_t_GIT_CONFIGMAP_INT32,
    /// Compare the value to the mapping's string exactly.
    String = ffi::git_configmap_t_GIT_CONFIGMAP_STRING,
}

/// A raw configuration-map type not published by this libgit2 API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitConfigmapType(ffi::git_configmap_t);

impl InvalidGitConfigmapType {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_configmap_t {
        self.0
    }
}

impl From<GitConfigmapType> for ffi::git_configmap_t {
    fn from(value: GitConfigmapType) -> Self {
        value as Self
    }
}

impl TryFrom<ffi::git_configmap_t> for GitConfigmapType {
    type Error = InvalidGitConfigmapType;

    fn try_from(value: ffi::git_configmap_t) -> Result<Self, Self::Error> {
        match value {
            ffi::git_configmap_t_GIT_CONFIGMAP_FALSE => Ok(Self::False),
            ffi::git_configmap_t_GIT_CONFIGMAP_TRUE => Ok(Self::True),
            ffi::git_configmap_t_GIT_CONFIGMAP_INT32 => Ok(Self::Int32),
            ffi::git_configmap_t_GIT_CONFIGMAP_STRING => Ok(Self::String),
            value => Err(InvalidGitConfigmapType(value)),
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn configmap_types_round_trip_through_the_c_type() {
        for kind in [
            GitConfigmapType::False,
            GitConfigmapType::True,
            GitConfigmapType::Int32,
            GitConfigmapType::String,
        ] {
            let raw = ffi::git_configmap_t::from(kind);
            assert_eq!(GitConfigmapType::try_from(raw), Ok(kind));
        }
    }

    #[test]
    fn configmap_type_rejects_unknown_values() {
        let raw = ffi::git_configmap_t_GIT_CONFIGMAP_STRING + 1;
        assert_eq!(GitConfigmapType::try_from(raw).unwrap_err().value(), raw);
    }

    #[test]
    fn configmap_type_matches_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitConfigmapType>(),
            size_of::<ffi::git_configmap_t>()
        );
        assert_eq!(
            align_of::<GitConfigmapType>(),
            align_of::<ffi::git_configmap_t>()
        );
    }
}

ffibox::define_ctype!(
    /// Wraps: git_configmap
    /// Layout-compatible mapping from textual configuration to an integer.
    GitConfigmap,
    GitConfigmapRef,
    GitConfigmapMut,
    ffi::git_configmap
);

// SAFETY: a config mapping only borrows its optional match string and owns no
// resource, so disposing its inline storage requires no action.
unsafe impl CValued for GitConfigmap {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl GitConfigmap {
    /// Constructs a mapping with no string match.
    #[must_use]
    pub fn new(kind: GitConfigmapType, map_value: core::ffi::c_int) -> CVal<Self> {
        let mut mapping = CVal::new(Self::zeroed());
        let mut view = mapping.as_mut();
        view.set_kind(kind);
        view.set_map_value(map_value);
        mapping
    }
}

impl<'a> GitConfigmapRef<'a> {
    /// Field: git_configmap.type
    /// Returns the checked mapping discriminator.
    pub fn kind(&self) -> Result<GitConfigmapType, InvalidGitConfigmapType> {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        let raw = unsafe { addr_of!((*self.as_ptr()).type_).read() };
        GitConfigmapType::try_from(raw)
    }

    /// Field: git_configmap.map_value
    /// Returns the integer produced when this entry matches.
    #[must_use]
    pub fn map_value(&self) -> core::ffi::c_int {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).map_value).read() }
    }

    /// Field: git_configmap.str_match
    /// Borrows the optional NUL-terminated string match.
    #[must_use]
    pub fn str_match(&self) -> Option<&'a CStr> {
        // SAFETY: this live shared handle permits the pointer-field read.
        let value = unsafe { addr_of!((*self.as_ptr()).str_match).read() };
        if value.is_null() {
            None
        } else {
            // SAFETY: a valid non-null mapping string is NUL-terminated and
            // remains live for the mapping handle's borrow.
            Some(unsafe { CStr::from_ptr(value) })
        }
    }
}

impl GitConfigmapMut<'_> {
    /// Replaces the mapping discriminator.
    pub fn set_kind(&mut self, kind: GitConfigmapType) {
        // SAFETY: this exclusive handle permits the scalar write, and `kind`
        // is one of the published C discriminants.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).type_).write(kind.into()) }
    }

    /// Replaces the integer produced when this entry matches.
    pub fn set_map_value(&mut self, map_value: core::ffi::c_int) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).map_value).write(map_value) }
    }

    /// Stores an optional borrowed NUL-terminated match string.
    ///
    /// # Safety
    ///
    /// A non-null `value` must remain live for every later use of the
    /// underlying mapping, including uses after this mutable reborrow ends.
    pub unsafe fn set_borrowed_str_match(&mut self, value: Option<&CStr>) {
        let value = value.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: this exclusive handle permits the pointer write; the caller
        // supplies the stored referent's unexpressible lifetime.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).str_match).write(value) }
    }
}

#[cfg(test)]
mod configmap_tests {
    use core::mem::{align_of, size_of};

    use ffibox::{CCell, CValued};

    use super::*;

    #[test]
    fn mapping_preserves_layout_and_inline_ownership() {
        fn assert_cell<T: CCell>() {}
        fn assert_valued<T: CValued>() {}

        assert_cell::<GitConfigmap>();
        assert_valued::<GitConfigmap>();
        assert_eq!(size_of::<GitConfigmap>(), size_of::<ffi::git_configmap>());
        assert_eq!(align_of::<GitConfigmap>(), align_of::<ffi::git_configmap>());
    }

    #[test]
    fn mapping_fields_round_trip() {
        let mut mapping = GitConfigmap::new(GitConfigmapType::String, 42);
        // SAFETY: this static string outlives every use of `mapping`.
        unsafe { mapping.as_mut().set_borrowed_str_match(Some(c"input")) };
        assert_eq!(mapping.as_ref().kind(), Ok(GitConfigmapType::String));
        assert_eq!(mapping.as_ref().map_value(), 42);
        assert_eq!(mapping.as_ref().str_match(), Some(c"input"));

        mapping.as_mut().set_kind(GitConfigmapType::False);
        // SAFETY: clearing the stored pointer creates no borrow.
        unsafe { mapping.as_mut().set_borrowed_str_match(None) };
        assert_eq!(mapping.as_ref().kind(), Ok(GitConfigmapType::False));
        assert!(mapping.as_ref().str_match().is_none());
    }
}

/// Wraps: git_config_foreach_cb
/// Safe callable surface for one transient configuration entry.
pub trait GitConfigForeachCallback {
    /// Visits an entry borrowed only for this invocation.
    fn call(&mut self, entry: crate::config::GitConfigEntryRef<'_>) -> i32;
}

impl<F> GitConfigForeachCallback for F
where
    F: FnMut(crate::config::GitConfigEntryRef<'_>) -> i32,
{
    fn call(&mut self, entry: crate::config::GitConfigEntryRef<'_>) -> i32 {
        self(entry)
    }
}
