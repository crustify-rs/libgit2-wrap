//! Safe wrappers for libgit2 net APIs.

use core::ffi::CStr;
use core::ptr::{addr_of, addr_of_mut};

use crate::ffi;
use crate::oid::{OidMut, OidRef};

/// Wraps: git_direction
/// The direction of a network operation.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Direction {
    /// Download objects and references from a remote.
    Fetch = ffi::git_direction_GIT_DIRECTION_FETCH,
    /// Upload objects and references to a remote.
    Push = ffi::git_direction_GIT_DIRECTION_PUSH,
}

/// A raw value that is not a valid [`Direction`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidDirection(ffi::git_direction);

impl InvalidDirection {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_direction {
        self.0
    }
}

impl From<Direction> for ffi::git_direction {
    fn from(direction: Direction) -> Self {
        direction as Self
    }
}

impl TryFrom<ffi::git_direction> for Direction {
    type Error = InvalidDirection;

    fn try_from(direction: ffi::git_direction) -> Result<Self, Self::Error> {
        match direction {
            ffi::git_direction_GIT_DIRECTION_FETCH => Ok(Self::Fetch),
            ffi::git_direction_GIT_DIRECTION_PUSH => Ok(Self::Push),
            value => Err(InvalidDirection(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn directions_round_trip_through_the_c_type() {
        for direction in [Direction::Fetch, Direction::Push] {
            let raw = ffi::git_direction::from(direction);
            assert_eq!(Direction::try_from(raw), Ok(direction));
        }
    }

    #[test]
    fn invalid_directions_are_rejected_without_constructing_an_enum() {
        let invalid = ffi::git_direction_GIT_DIRECTION_PUSH + 1;
        let error = Direction::try_from(invalid).unwrap_err();
        assert_eq!(error.value(), invalid);
    }

    #[test]
    fn direction_preserves_the_c_enum_layout() {
        assert_eq!(size_of::<Direction>(), size_of::<ffi::git_direction>());
        assert_eq!(align_of::<Direction>(), align_of::<ffi::git_direction>());
    }
}

ffibox::define_ctype!(
    /// Wraps: git_remote_head
    /// A layout-compatible advertised remote reference.
    ///
    /// Remote and transport owners retain these records and their strings;
    /// borrowed handles expose views without assuming an independent head
    /// destructor.
    RemoteHead,
    RemoteHeadRef,
    RemoteHeadMut,
    ffi::git_remote_head
);

impl<'a> RemoteHeadRef<'a> {
    /// Wraps: git_remote_head.oid
    /// Borrows the remote object ID embedded in this head.
    #[must_use]
    pub fn oid(&self) -> OidRef<'a> {
        // SAFETY: raw-place projection reaches the live inline `git_oid`
        // without forming a reference over C-visible storage.
        let oid = unsafe { addr_of!((*self.as_ptr()).oid) }.cast_mut();
        // SAFETY: an inline field is non-null and remains live for this
        // head handle's `'a` lifetime.
        unsafe { OidRef::from_ptr(oid) }.expect("an inline oid field is non-null")
    }

    /// Wraps: git_remote_head.name
    /// Borrows the advertised reference name, when present.
    #[must_use]
    pub fn name(&self) -> Option<&'a CStr> {
        // SAFETY: raw-place projection reads the initialized pointer without
        // forming a reference over the head. Its owner retains any non-null
        // NUL-terminated string for this handle's lifetime.
        let name = unsafe { addr_of!((*self.as_ptr()).name).read() };
        if name.is_null() {
            None
        } else {
            // SAFETY: the field contract above guarantees a live C string for
            // the enclosing handle's `'a` lifetime.
            Some(unsafe { CStr::from_ptr(name) })
        }
    }

    /// Wraps: git_remote_head.local
    /// Reports whether the advertised object is available locally.
    #[must_use]
    pub fn is_local(&self) -> bool {
        // SAFETY: raw-place projection reads the initialized scalar without
        // forming a reference over C-visible storage.
        unsafe { addr_of!((*self.as_ptr()).local).read() != 0 }
    }

    /// Wraps: git_remote_head.loid
    /// Borrows the local object ID embedded in this head.
    #[must_use]
    pub fn local_oid(&self) -> OidRef<'a> {
        // SAFETY: raw-place projection reaches the live inline `git_oid`
        // without forming a reference over C-visible storage.
        let oid = unsafe { addr_of!((*self.as_ptr()).loid) }.cast_mut();
        // SAFETY: an inline field is non-null and remains live for this
        // head handle's `'a` lifetime.
        unsafe { OidRef::from_ptr(oid) }.expect("an inline oid field is non-null")
    }

    /// Wraps: git_remote_head.symref_target
    /// Borrows the symbolic-reference target, when the server advertised one.
    #[must_use]
    pub fn symref_target(&self) -> Option<&'a CStr> {
        // SAFETY: raw-place projection reads the initialized pointer without
        // forming a reference over the head. Its owner retains any non-null
        // NUL-terminated string for this handle's lifetime.
        let target = unsafe { addr_of!((*self.as_ptr()).symref_target).read() };
        if target.is_null() {
            None
        } else {
            // SAFETY: the field contract above guarantees a live C string for
            // the enclosing handle's `'a` lifetime.
            Some(unsafe { CStr::from_ptr(target) })
        }
    }
}

impl RemoteHeadMut<'_> {
    /// Exclusively borrows the embedded remote object ID.
    #[must_use]
    pub fn oid_mut(&mut self) -> OidMut<'_> {
        // SAFETY: raw-place projection reaches the live inline field and the
        // resulting handle is bounded by this exclusive reborrow.
        let oid = unsafe { addr_of_mut!((*self.as_mut_ptr()).oid) };
        // SAFETY: an inline field is non-null and exclusively borrowed here.
        unsafe { OidMut::from_ptr(oid) }.expect("an inline oid field is non-null")
    }

    /// Sets whether the advertised object is available locally.
    pub fn set_local(&mut self, local: bool) {
        // SAFETY: this exclusive handle permits a raw-place write of the
        // scalar C flag.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).local).write(i32::from(local)) }
    }

    /// Exclusively borrows the embedded local object ID.
    #[must_use]
    pub fn local_oid_mut(&mut self) -> OidMut<'_> {
        // SAFETY: raw-place projection reaches the live inline field and the
        // resulting handle is bounded by this exclusive reborrow.
        let oid = unsafe { addr_of_mut!((*self.as_mut_ptr()).loid) };
        // SAFETY: an inline field is non-null and exclusively borrowed here.
        unsafe { OidMut::from_ptr(oid) }.expect("an inline oid field is non-null")
    }

    /// Stores a borrowed advertised reference name.
    ///
    /// # Safety
    ///
    /// The caller must first dispose any owned old value. A non-null `name`
    /// must remain alive until the concrete remote-head owner replaces the
    /// field or finishes using the head, and that owner must not free the
    /// borrowed string. Rust cannot express those external contracts.
    pub unsafe fn set_borrowed_name(&mut self, name: Option<&CStr>) {
        let name = name.map_or(core::ptr::null_mut(), |name| name.as_ptr().cast_mut());
        // SAFETY: this exclusive handle permits replacing the pointer field;
        // the caller supplies its unexpressible lifetime obligation.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).name).write(name) }
    }

    /// Stores a borrowed symbolic-reference target.
    ///
    /// # Safety
    ///
    /// The caller must first dispose any owned old value. A non-null `target`
    /// must remain alive until the concrete remote-head owner replaces the
    /// field or finishes using the head, and that owner must not free the
    /// borrowed string. Rust cannot express those external contracts.
    pub unsafe fn set_borrowed_symref_target(&mut self, target: Option<&CStr>) {
        let target = target.map_or(core::ptr::null_mut(), |target| target.as_ptr().cast_mut());
        // SAFETY: this exclusive handle permits replacing the pointer field;
        // the caller supplies its unexpressible lifetime obligation.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).symref_target).write(target) }
    }
}

#[cfg(test)]
mod remote_head_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn remote_head_and_handles_match_the_c_layout() {
        assert_eq!(size_of::<RemoteHead>(), size_of::<ffi::git_remote_head>());
        assert_eq!(align_of::<RemoteHead>(), align_of::<ffi::git_remote_head>());
        assert_eq!(
            size_of::<RemoteHeadRef<'_>>(),
            size_of::<*const ffi::git_remote_head>()
        );
        assert_eq!(
            size_of::<RemoteHeadMut<'_>>(),
            size_of::<*mut ffi::git_remote_head>()
        );
    }

    #[test]
    fn remote_head_handles_access_and_mutate_fields() {
        let mut raw = ffi::git_remote_head {
            local: 0,
            oid: ffi::git_oid {
                type_: ffi::git_oid_t_GIT_OID_SHA1 as u8,
                id: [1; 32],
            },
            loid: ffi::git_oid {
                type_: ffi::git_oid_t_GIT_OID_SHA256 as u8,
                id: [2; 32],
            },
            name: core::ptr::null_mut(),
            symref_target: core::ptr::null_mut(),
        };

        // SAFETY: `raw` remains live and is exclusively borrowed by the
        // handle for this scope.
        let mut head = unsafe { RemoteHeadMut::from_ptr(&raw mut raw) }.unwrap();
        assert!(!head.as_ref().is_local());
        assert!(head.as_ref().name().is_none());
        assert!(head.as_ref().symref_target().is_none());
        assert_eq!(head.as_ref().oid().raw_bytes().elem(0), Some(1));
        assert_eq!(head.as_ref().local_oid().raw_bytes().elem(0), Some(2));

        head.set_local(true);
        // SAFETY: these static C strings outlive the stack head and are never
        // freed through it in this test.
        unsafe {
            head.set_borrowed_name(Some(c"refs/heads/main"));
            head.set_borrowed_symref_target(Some(c"refs/heads/trunk"));
        }
        assert!(head.as_ref().is_local());
        assert_eq!(head.as_ref().name(), Some(c"refs/heads/main"));
        assert_eq!(head.as_ref().symref_target(), Some(c"refs/heads/trunk"));

        assert!(head.oid_mut().raw_bytes_mut().set_elem(0, 3));
        assert!(head.local_oid_mut().raw_bytes_mut().set_elem(0, 4));
        assert_eq!(head.as_ref().oid().raw_bytes().elem(0), Some(3));
        assert_eq!(head.as_ref().local_oid().raw_bytes().elem(0), Some(4));
    }
}
