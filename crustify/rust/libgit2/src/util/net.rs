//! Safe wrappers for libgit2 net APIs.

use core::ffi::CStr;
use core::ptr::{addr_of, addr_of_mut};

use ffibox::CrustifyStr;

use crate::ffi;
use crate::oid::{OidMut, OidRef};
use crate::util::alloc::GitStrdupFree;

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
mod unit_tests {
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
    /// A head has no destructor of its own: its concrete owner - a transport
    /// packet, a local-transport head, or a remote's reference vector -
    /// releases the record together with its strings.
    ///
    /// `symref_target` is always an owned allocation from libgit2's configured
    /// allocator, so it has safe owning accessors. `name` is owned by every
    /// head the transports build but borrowed by the temporary search keys and
    /// object-ID heads that `remote.c` and `fetch.c` place on the stack, so its
    /// owning accessors carry that unexpressible distinction as a caller
    /// obligation.
    RemoteHead,
    RemoteHeadRef,
    RemoteHeadMut,
    ffi::git_remote_head
);

/// An owned advertised-reference string held by a [`RemoteHead`].
///
/// Every producer builds these with `git__strdup`, `git__malloc` or
/// `git_str_detach` and releases them with `git__free`, so they share the
/// configured-allocator strategy.
pub type RemoteHeadString = CrustifyStr<GitStrdupFree>;

impl<'a> RemoteHeadRef<'a> {
    /// Field: git_remote_head.oid
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

    /// Field: git_remote_head.name
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

    /// Field: git_remote_head.local
    /// Reports whether the advertised object is available locally.
    #[must_use]
    pub fn is_local(&self) -> bool {
        // SAFETY: raw-place projection reads the initialized scalar without
        // forming a reference over C-visible storage.
        unsafe { addr_of!((*self.as_ptr()).local).read() != 0 }
    }

    /// Field: git_remote_head.loid
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

    /// Field: git_remote_head.symref_target
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
    /// This is the contract used by the stack search keys in `remote.c` and by
    /// the object-ID heads in `fetch.c`, which point at storage another object
    /// owns.
    ///
    /// # Safety
    ///
    /// The caller must first move out any owned old value with
    /// [`take_name`](Self::take_name); this write leaks it otherwise. A
    /// non-null `name` must remain alive until the head's concrete owner
    /// replaces the field or finishes using the head, and that owner must not
    /// free the borrowed string. Rust cannot express those external contracts.
    pub unsafe fn set_borrowed_name(&mut self, name: Option<&CStr>) {
        let name = name.map_or(core::ptr::null_mut(), |name| name.as_ptr().cast_mut());
        // SAFETY: this exclusive handle permits replacing the pointer field;
        // the caller supplies its unexpressible lifetime obligation.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).name).write(name) }
    }

    /// Moves the owned advertised reference name out, leaving the head unnamed.
    ///
    /// # Safety
    ///
    /// The stored name must be an allocation owned by this head and made by
    /// libgit2's configured allocator, as every transport-built head holds.
    /// It must not be a name installed by
    /// [`set_borrowed_name`](Self::set_borrowed_name) or by one of the C
    /// search keys that borrow another object's storage: only the producer
    /// knows which contract a given head follows.
    #[must_use]
    pub unsafe fn take_name(&mut self) -> Option<RemoteHeadString> {
        let head = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits reading and clearing the
        // pointer field, which transfers whatever it addressed out of the head.
        let name = unsafe {
            let name = addr_of!((*head).name).read();
            addr_of_mut!((*head).name).write(core::ptr::null_mut());
            name
        };
        // SAFETY: the caller guarantees that a non-null name was a unique
        // allocation from libgit2's configured allocator, and the head no
        // longer refers to it.
        unsafe { RemoteHeadString::from_raw(name) }
    }

    /// Replaces the owned advertised reference name, releasing the old one.
    ///
    /// # Safety
    ///
    /// Carries [`take_name`](Self::take_name)'s obligation for the value being
    /// released.
    pub unsafe fn set_name(&mut self, name: Option<RemoteHeadString>) {
        // SAFETY: the caller guarantees that the replaced name is owned by the
        // head and releasable through this strategy.
        let old = unsafe { self.take_name() };
        let name = name.map_or(core::ptr::null_mut(), RemoteHeadString::into_raw);
        // SAFETY: this exclusive handle permits installing ownership of the
        // compatible allocation now that the old pointer has been cleared.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).name).write(name) }
        drop(old);
    }

    /// Moves the owned symbolic-reference target out, leaving the head without
    /// one.
    #[must_use]
    pub fn take_symref_target(&mut self) -> Option<RemoteHeadString> {
        let head = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits reading and clearing the owned
        // pointer field, which transfers its unique allocation out.
        let target = unsafe {
            let target = addr_of!((*head).symref_target).read();
            addr_of_mut!((*head).symref_target).write(core::ptr::null_mut());
            target
        };
        // SAFETY: a valid head's non-null symref_target is always a unique
        // NUL-terminated allocation from libgit2's configured allocator, and
        // the head no longer refers to it.
        unsafe { RemoteHeadString::from_raw(target) }
    }

    /// Replaces the owned symbolic-reference target, releasing the old one.
    ///
    /// This mirrors `smart.c`, which frees the advertised target before
    /// storing the one detached from its symref capability buffer.
    pub fn set_symref_target(&mut self, target: Option<RemoteHeadString>) {
        let old = self.take_symref_target();
        let target = target.map_or(core::ptr::null_mut(), RemoteHeadString::into_raw);
        // SAFETY: this exclusive handle permits installing ownership of the
        // compatible allocation now that the old pointer has been cleared.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).symref_target).write(target) }
        drop(old);
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
        // SAFETY: these static C strings outlive the stack head, and the test
        // moves them back out below rather than releasing them through it.
        unsafe {
            head.set_borrowed_name(Some(c"refs/heads/main"));
        }
        assert!(head.as_ref().is_local());
        assert_eq!(head.as_ref().name(), Some(c"refs/heads/main"));

        assert!(head.oid_mut().raw_bytes_mut().set_elem(0, 3));
        assert!(head.local_oid_mut().raw_bytes_mut().set_elem(0, 4));
        assert_eq!(head.as_ref().oid().raw_bytes().elem(0), Some(3));
        assert_eq!(head.as_ref().local_oid().raw_bytes().elem(0), Some(4));

        // SAFETY: clearing the field stores null, which owes no release; the
        // borrowed static string is simply forgotten by the head.
        unsafe { head.set_borrowed_name(None) };
        assert!(head.as_ref().name().is_none());
    }

    #[test]
    fn owned_head_strings_are_moved_in_and_out() {
        // SAFETY: libgit2 initialization is refcounted and this test balances
        // it below, after every configured allocation has been released.
        let init_count = unsafe { ffi::git_libgit2_init() };
        assert!(init_count > 0);

        let mut raw = ffi::git_remote_head {
            local: 0,
            // SAFETY: the bindgen OID record holds only integers, so an
            // all-zero value is a valid initialized one.
            oid: unsafe { core::mem::zeroed() },
            // SAFETY: as above, for the second inline OID.
            loid: unsafe { core::mem::zeroed() },
            name: core::ptr::null_mut(),
            symref_target: core::ptr::null_mut(),
        };

        // SAFETY: `raw` remains live for this scope, its two string fields are
        // null rather than borrowed, and the handle is the only access path.
        let mut head = unsafe { RemoteHeadMut::from_ptr(&raw mut raw) }.unwrap();

        assert!(head.take_symref_target().is_none());
        // SAFETY: the field is null, so nothing is released.
        assert!(unsafe { head.take_name() }.is_none());

        head.set_symref_target(Some(duplicate(c"refs/heads/trunk")));
        // SAFETY: the previous name is null and the new one is a libgit2
        // allocation owned by this head from now on.
        unsafe { head.set_name(Some(duplicate(c"HEAD"))) };
        assert_eq!(head.as_ref().name(), Some(c"HEAD"));
        assert_eq!(head.as_ref().symref_target(), Some(c"refs/heads/trunk"));

        // Replacing an owned string releases the previous allocation, as
        // `smart.c` does when a symref capability arrives.
        head.set_symref_target(Some(duplicate(c"refs/heads/main")));
        assert_eq!(head.as_ref().symref_target(), Some(c"refs/heads/main"));

        // SAFETY: the stored name is the libgit2 allocation installed above.
        let name = unsafe { head.take_name() }.expect("the name is owned");
        let target = head
            .take_symref_target()
            .expect("the symbolic target is owned");
        assert_eq!(name.as_c_str(), c"HEAD");
        assert_eq!(target.as_c_str(), c"refs/heads/main");
        assert!(head.as_ref().name().is_none());
        assert!(head.as_ref().symref_target().is_none());

        drop(name);
        drop(target);

        // SAFETY: balances this test's successful initialization call.
        let remaining = unsafe { ffi::git_libgit2_shutdown() };
        assert!(remaining >= 0);
    }

    fn duplicate(value: &CStr) -> RemoteHeadString {
        // SAFETY: `value` is a live NUL-terminated string for this
        // synchronous copy, and libgit2 is initialized by the caller.
        let raw = unsafe { ffi::crustify_git__strdup(value.as_ptr()) };
        // SAFETY: a non-null result is a fresh unique allocation from
        // libgit2's configured allocator, matched by `GitStrdupFree`.
        unsafe { RemoteHeadString::from_raw(raw) }.expect("libgit2 should duplicate the string")
    }
}
