//! Safe wrappers for libgit2 oid APIs.

use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CSlice, CSliceMut};

use crate::ffi;

/// Wraps: git_oid_t
/// The hash algorithm identifying an object ID's byte width.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum OidType {
    /// A 20-byte SHA-1 object ID.
    Sha1 = ffi::git_oid_t_GIT_OID_SHA1,
    /// A 32-byte SHA-256 object ID.
    Sha256 = ffi::git_oid_t_GIT_OID_SHA256,
}

/// A raw value that is not a published [`OidType`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidOidType(ffi::git_oid_t);

impl InvalidOidType {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_oid_t {
        self.0
    }
}

impl From<OidType> for ffi::git_oid_t {
    fn from(oid_type: OidType) -> Self {
        oid_type as Self
    }
}

impl TryFrom<ffi::git_oid_t> for OidType {
    type Error = InvalidOidType;

    fn try_from(oid_type: ffi::git_oid_t) -> Result<Self, Self::Error> {
        match oid_type {
            ffi::git_oid_t_GIT_OID_SHA1 => Ok(Self::Sha1),
            ffi::git_oid_t_GIT_OID_SHA256 => Ok(Self::Sha256),
            value => Err(InvalidOidType(value)),
        }
    }
}

ffibox::define_ctype!(
    /// Wraps: git_oid
    /// A layout-compatible Git object identifier.
    Oid,
    OidRef,
    OidMut,
    ffi::git_oid
);

impl<'a> OidRef<'a> {
    /// Wraps: git_oid.type
    /// Returns the object ID's hash algorithm.
    ///
    /// An all-zero or otherwise incomplete C value returns an error rather
    /// than constructing an invalid Rust enum.
    pub fn oid_type(&self) -> Result<OidType, InvalidOidType> {
        // SAFETY: this live shared handle covers the complete C value, and
        // raw-place projection reads the initialized byte without forming a
        // reference to C-visible memory.
        let raw = unsafe { addr_of!((*self.as_ptr()).type_).read() };
        OidType::try_from(ffi::git_oid_t::from(raw))
    }

    /// Wraps: git_oid.id
    /// Returns all 32 bytes of the inline object-ID field.
    ///
    /// For SHA-1 values, only the first 20 bytes carry the digest; libgit2
    /// leaves the remaining field bytes unused.
    #[must_use]
    pub fn raw_bytes(&self) -> CSlice<'a, u8> {
        // SAFETY: this live shared handle covers the initialized C value, and
        // raw-place projection does not form a reference to its byte array.
        let bytes = unsafe { addr_of!((*self.as_ptr()).id) }
            .cast::<u8>()
            .cast_mut();
        // SAFETY: `bytes` points to the 32 initialized bytes of the inline
        // array, which remain live for the handle's `'a` borrow. `CSlice`
        // exposes copies rather than a reference over C-visible memory.
        unsafe { CSlice::from_raw_parts(NonNull::new_unchecked(bytes), 32) }
    }
}

impl OidMut<'_> {
    /// Sets the object ID's hash algorithm.
    pub fn set_oid_type(&mut self, oid_type: OidType) {
        let raw = oid_type as ffi::git_oid_t as u8;
        // SAFETY: this exclusive handle permits a raw-place write to the
        // scalar field, and every `OidType` discriminant fits in its C byte.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).type_).write(raw) }
    }

    /// Borrows all 32 inline object-ID bytes exclusively.
    #[must_use]
    pub fn raw_bytes_mut(&mut self) -> CSliceMut<'_, u8> {
        // SAFETY: this exclusive handle covers the initialized C value, and
        // raw-place projection does not form a reference to its byte array.
        let bytes = unsafe { addr_of_mut!((*self.as_mut_ptr()).id) }.cast::<u8>();
        // SAFETY: `bytes` points to the 32 initialized bytes of the inline
        // array, and the mutable view is bounded by this exclusive reborrow.
        unsafe { CSliceMut::from_raw_parts(NonNull::new_unchecked(bytes), 32) }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn oid_types_round_trip_and_reject_unknown_values() {
        for oid_type in [OidType::Sha1, OidType::Sha256] {
            let raw = ffi::git_oid_t::from(oid_type);
            assert_eq!(OidType::try_from(raw), Ok(oid_type));
        }

        let invalid = ffi::git_oid_t_GIT_OID_SHA256 + 1;
        assert_eq!(OidType::try_from(invalid), Err(InvalidOidType(invalid)));
        assert_eq!(InvalidOidType(invalid).value(), invalid);
    }

    #[test]
    fn wrappers_match_the_c_layout() {
        assert_eq!(size_of::<OidType>(), size_of::<ffi::git_oid_t>());
        assert_eq!(align_of::<OidType>(), align_of::<ffi::git_oid_t>());
        assert_eq!(size_of::<Oid>(), size_of::<ffi::git_oid>());
        assert_eq!(align_of::<Oid>(), align_of::<ffi::git_oid>());
        assert_eq!(size_of::<OidRef<'_>>(), size_of::<*const ffi::git_oid>());
        assert_eq!(size_of::<OidMut<'_>>(), size_of::<*mut ffi::git_oid>());
    }

    #[test]
    fn borrowed_handles_read_and_write_oid_fields() {
        let mut raw = ffi::git_oid {
            type_: ffi::git_oid_t_GIT_OID_SHA1 as u8,
            id: [0; 32],
        };

        // SAFETY: `raw` is initialized, non-null, exclusively borrowed for
        // the handle's lifetime, and remains live until its last use.
        let mut oid = unsafe { OidMut::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");
        assert_eq!(oid.as_ref().oid_type(), Ok(OidType::Sha1));

        oid.set_oid_type(OidType::Sha256);
        let expected = core::array::from_fn(|index| index as u8);
        assert!(oid.raw_bytes_mut().copy_from_slice(&expected));

        assert_eq!(oid.as_ref().oid_type(), Ok(OidType::Sha256));
        let mut actual = [0; 32];
        assert!(oid.as_ref().raw_bytes().copy_to_slice(&mut actual));
        assert_eq!(actual, expected);
    }

    #[test]
    fn zeroed_oid_reports_an_invalid_type_without_creating_an_enum() {
        let mut oid = Oid::zeroed();
        let raw = addr_of_mut!(oid).cast::<ffi::git_oid>();

        // SAFETY: `raw` points to the live, initialized, layout-compatible
        // stack value above and is only borrowed by this handle.
        let oid =
            unsafe { OidRef::from_ptr(raw) }.expect("the address of a stack value is non-null");
        assert_eq!(oid.oid_type(), Err(InvalidOidType(0)));
    }

    #[test]
    fn safe_oid_operations_parse_compare_and_format() {
        let raw = [0xabu8; 20];
        let mut first = git_oid_fromraw(&raw).expect("twenty bytes form a SHA-1 OID");
        let mut second = git_oid_fromstrn(b"abababababababababababababababababababab")
            .expect("forty hexadecimal bytes form a SHA-1 OID");

        // SAFETY: both pointers address live initialized wrapper storage and
        // are borrowed only within this scope.
        let first_ref = unsafe { OidRef::from_ptr(addr_of_mut!(first).cast()) }.unwrap();
        // SAFETY: as above, for the independent `second` value.
        let second_ref = unsafe { OidRef::from_ptr(addr_of_mut!(second).cast()) }.unwrap();
        assert!(git_oid_equal(first_ref, second_ref));
        assert_eq!(
            git_oid_cmp(first_ref, second_ref),
            core::cmp::Ordering::Equal
        );
        assert!(!git_oid_is_zero(first_ref));

        let mut text = [0u8; 41];
        assert_eq!(
            git_oid_tostr(&mut text, first_ref).unwrap().to_bytes(),
            b"abababababababababababababababababababab"
        );
        assert!(git_oid_tostr(&mut [], first_ref).is_none());
    }
}

/// Wraps: git_oid_cmp
/// Compares object IDs by algorithm and digest bytes.
pub fn git_oid_cmp(a: OidRef<'_>, b: OidRef<'_>) -> core::cmp::Ordering {
    // SAFETY: both object IDs are live for this read-only comparison.
    unsafe { ffi::git_oid_cmp(a.as_ptr(), b.as_ptr()) }.cmp(&0)
}

/// Wraps: git_oid_equal
/// Reports whether two object IDs are identical.
pub fn git_oid_equal(a: OidRef<'_>, b: OidRef<'_>) -> bool {
    // SAFETY: both object IDs are live for this read-only comparison.
    unsafe { ffi::git_oid_equal(a.as_ptr(), b.as_ptr()) != 0 }
}

/// Wraps: git_oid_fromraw
/// Constructs a SHA-1 object ID from its 20 raw digest bytes.
pub fn git_oid_fromraw(raw: &[u8; 20]) -> Result<Oid, i32> {
    let mut out = Oid::zeroed();
    // SAFETY: `out` is writable and `raw` provides the 20 bytes required by
    // this deprecated SHA-1-specific entry point.
    let status = unsafe { ffi::git_oid_fromraw(addr_of_mut!(out).cast(), raw.as_ptr()) };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_oid_fromstrn
/// Parses a counted SHA-1 hexadecimal prefix.
pub fn git_oid_fromstrn(hex: &[u8]) -> Result<Oid, i32> {
    let mut out = Oid::zeroed();
    // SAFETY: `out` is writable and `hex` provides exactly the readable byte
    // count passed to C; the input need not be NUL terminated.
    let status =
        unsafe { ffi::git_oid_fromstrn(addr_of_mut!(out).cast(), hex.as_ptr().cast(), hex.len()) };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_oid_is_zero
/// Reports whether all digest bytes are zero.
pub fn git_oid_is_zero(id: OidRef<'_>) -> bool {
    // SAFETY: `id` is live for this read-only query.
    unsafe { ffi::git_oid_is_zero(id.as_ptr()) != 0 }
}

/// Wraps: git_oid_tostr
/// Formats an object ID into a nonempty caller-owned buffer.
pub fn git_oid_tostr<'a>(out: &'a mut [u8], oid: OidRef<'_>) -> Option<&'a core::ffi::CStr> {
    if out.is_empty() {
        return None;
    }
    // SAFETY: `out` is a writable run of `len` bytes and `oid` is live. C
    // writes a trailing NUL and retains neither pointer.
    unsafe { ffi::git_oid_tostr(out.as_mut_ptr().cast(), out.len(), oid.as_ptr()) };
    core::ffi::CStr::from_bytes_until_nul(out).ok()
}
