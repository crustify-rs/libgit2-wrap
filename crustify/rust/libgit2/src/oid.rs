//! Safe wrappers for libgit2 oid APIs.

use core::mem::{offset_of, size_of};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CSlice, CSliceMut};

use crate::ffi;

/// The inline digest capacity of a `git_oid`, matching C's `GIT_OID_MAX_SIZE`.
///
/// Derived from the C layout rather than spelled as a literal so that widening
/// the inline array cannot silently desynchronize the wrapper. `git_oid` is
/// byte-aligned, so its size less the digest field's offset is exactly the
/// array length; `wrappers_match_the_c_layout` pins that down.
pub const RAW_DIGEST_LEN: usize = size_of::<ffi::git_oid>() - offset_of!(ffi::git_oid, id);

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

impl OidType {
    /// Returns the significant digest width in bytes.
    ///
    /// This mirrors C's `git_oid_size`: only these leading bytes of the inline
    /// `git_oid.id` array carry the digest, and every C reader bounds itself
    /// this way. The trailing bytes stay zero-filled by libgit2's parsers.
    #[must_use]
    pub const fn digest_len(self) -> usize {
        match self {
            Self::Sha1 => 20,
            Self::Sha256 => 32,
        }
    }

    /// Returns the hexadecimal width, mirroring C's `git_oid_hexsize`.
    ///
    /// A formatting buffer must hold one more byte for the trailing NUL.
    #[must_use]
    pub const fn hex_len(self) -> usize {
        self.digest_len() * 2
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
    /// Field: git_oid.type
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

    /// Field: git_oid.id
    /// Returns the whole inline object-ID field, including unused bytes.
    ///
    /// The array is always [`RAW_DIGEST_LEN`] bytes wide, but only
    /// [`OidType::digest_len`] of them carry the digest. Use [`Self::digest`]
    /// to borrow exactly the significant run the way C does.
    #[must_use]
    pub fn raw_bytes(&self) -> CSlice<'a, u8> {
        // SAFETY: this live shared handle covers the initialized C value, and
        // raw-place projection does not form a reference to its byte array.
        let bytes = unsafe { addr_of!((*self.as_ptr()).id) }
            .cast::<u8>()
            .cast_mut();
        // SAFETY: `bytes` points to the `RAW_DIGEST_LEN` initialized bytes of
        // the inline array, which remain live for the handle's `'a` borrow.
        // `CSlice` exposes copies rather than a reference over C-visible
        // memory.
        unsafe { CSlice::from_raw_parts(NonNull::new_unchecked(bytes), RAW_DIGEST_LEN) }
    }

    /// Borrows only the digest bytes significant for this object ID's hash
    /// algorithm.
    ///
    /// This is the run every C reader bounds itself to through `git_oid_size`;
    /// a value whose stored algorithm is not published has no such run and
    /// returns an error instead of a truncated view.
    pub fn digest(&self) -> Result<CSlice<'a, u8>, InvalidOidType> {
        let len = self.oid_type()?.digest_len();
        debug_assert!(len <= RAW_DIGEST_LEN);
        // SAFETY: as `raw_bytes`, for the same live inline array.
        let bytes = unsafe { addr_of!((*self.as_ptr()).id) }
            .cast::<u8>()
            .cast_mut();
        // SAFETY: `len` is at most the inline array's `RAW_DIGEST_LEN` bytes,
        // all initialized and live for the handle's `'a` borrow.
        Ok(unsafe { CSlice::from_raw_parts(NonNull::new_unchecked(bytes), len) })
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

    /// Borrows the whole inline object-ID field exclusively.
    #[must_use]
    pub fn raw_bytes_mut(&mut self) -> CSliceMut<'_, u8> {
        // SAFETY: this exclusive handle covers the initialized C value, and
        // raw-place projection does not form a reference to its byte array.
        let bytes = unsafe { addr_of_mut!((*self.as_mut_ptr()).id) }.cast::<u8>();
        // SAFETY: `bytes` points to the `RAW_DIGEST_LEN` initialized bytes of
        // the inline array, and the mutable view is bounded by this exclusive
        // reborrow.
        unsafe { CSliceMut::from_raw_parts(NonNull::new_unchecked(bytes), RAW_DIGEST_LEN) }
    }

    /// Borrows only the digest bytes significant for the stored hash
    /// algorithm, exclusively.
    ///
    /// Writing through this view cannot disturb the unused trailing bytes that
    /// libgit2's parsers keep zero-filled.
    pub fn digest_mut(&mut self) -> Result<CSliceMut<'_, u8>, InvalidOidType> {
        let len = self.as_ref().oid_type()?.digest_len();
        debug_assert!(len <= RAW_DIGEST_LEN);
        // SAFETY: as `raw_bytes_mut`, for the same live inline array.
        let bytes = unsafe { addr_of_mut!((*self.as_mut_ptr()).id) }.cast::<u8>();
        // SAFETY: `len` is at most the inline array's `RAW_DIGEST_LEN` bytes,
        // and the mutable view is bounded by this exclusive reborrow.
        Ok(unsafe { CSliceMut::from_raw_parts(NonNull::new_unchecked(bytes), len) })
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

        // `RAW_DIGEST_LEN` is derived from the header size and the digest
        // offset; this literal only type-checks when it is the inline array's
        // real length, which C spells `GIT_OID_MAX_SIZE`.
        let raw = ffi::git_oid {
            type_: 0,
            id: [0; RAW_DIGEST_LEN],
        };
        assert_eq!(raw.id.len(), RAW_DIGEST_LEN);
        assert_eq!(RAW_DIGEST_LEN, OidType::Sha256.digest_len());
    }

    #[test]
    fn digest_widths_mirror_the_c_size_helpers() {
        assert_eq!(OidType::Sha1.digest_len(), 20);
        assert_eq!(OidType::Sha1.hex_len(), 40);
        assert_eq!(OidType::Sha256.digest_len(), 32);
        assert_eq!(OidType::Sha256.hex_len(), 64);
    }

    #[test]
    fn digest_views_cover_only_the_significant_bytes() {
        let mut raw = ffi::git_oid {
            type_: ffi::git_oid_t_GIT_OID_SHA1 as u8,
            id: core::array::from_fn(|index| index as u8 + 1),
        };

        // SAFETY: `raw` is initialized, non-null, exclusively borrowed for the
        // handle's lifetime, and remains live until its last use.
        let mut oid = unsafe { OidMut::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");

        assert_eq!(oid.as_ref().raw_bytes().len(), RAW_DIGEST_LEN);
        let digest = oid.as_ref().digest().expect("SHA-1 is published");
        assert_eq!(digest.len(), 20);
        let mut copied = [0u8; 20];
        assert!(digest.copy_to_slice(&mut copied));
        assert_eq!(copied, core::array::from_fn(|index| index as u8 + 1));

        // Writing through the exclusive digest view cannot reach the unused
        // trailing bytes libgit2 leaves zero-filled for SHA-1.
        assert!(
            oid.digest_mut()
                .expect("SHA-1 is published")
                .copy_from_slice(&[0xff; 20])
        );
        let mut whole = [0u8; RAW_DIGEST_LEN];
        assert!(oid.as_ref().raw_bytes().copy_to_slice(&mut whole));
        assert_eq!(&whole[..20], &[0xff; 20]);
        assert_eq!(whole[20], 21);

        // Widening to SHA-256 makes the trailing bytes significant.
        oid.set_oid_type(OidType::Sha256);
        assert_eq!(
            oid.as_ref().digest().expect("SHA-256 is published").len(),
            RAW_DIGEST_LEN
        );
    }

    #[test]
    fn digest_views_reject_an_unpublished_algorithm() {
        let mut oid = Oid::zeroed();
        let raw = addr_of_mut!(oid).cast::<ffi::git_oid>();

        // SAFETY: `raw` points to the live, initialized, layout-compatible
        // stack value above and is only borrowed by this handle.
        let mut oid =
            unsafe { OidMut::from_ptr(raw) }.expect("the address of a stack value is non-null");
        assert_eq!(oid.as_ref().digest().err(), Some(InvalidOidType(0)));
        assert_eq!(oid.digest_mut().err(), Some(InvalidOidType(0)));
        // The whole inline field stays reachable for a value C has not typed.
        assert_eq!(oid.as_ref().raw_bytes().len(), RAW_DIGEST_LEN);
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
