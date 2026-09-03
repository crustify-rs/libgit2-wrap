//! Safe wrappers for libgit2 merge APIs.

use core::ffi::CStr;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CLenDropped, CSlice, CVal, CVec, CrustifyStr};

use crate::annotated_commit::AnnotatedCommitRef;
use crate::ffi;
use crate::oid::{Oid, OidRef};
use crate::refs::GitReferenceRef;
use crate::repository::{GitRepositoryMut, GitRepositoryRef};
use crate::util::alloc::GitStrdupFree;

/// Wraps: git_merge_analysis_t
/// A layout-compatible set of merge opportunities reported by libgit2.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitMergeAnalysis(ffi::git_merge_analysis_t);

impl GitMergeAnalysis {
    /// No merge is possible.
    pub const NONE: Self = Self(ffi::git_merge_analysis_t_GIT_MERGE_ANALYSIS_NONE);
    /// The inputs and `HEAD` have diverged and require a normal merge.
    pub const NORMAL: Self = Self(ffi::git_merge_analysis_t_GIT_MERGE_ANALYSIS_NORMAL);
    /// Every input is already reachable from `HEAD`.
    pub const UP_TO_DATE: Self = Self(ffi::git_merge_analysis_t_GIT_MERGE_ANALYSIS_UP_TO_DATE);
    /// The input can be checked out as a fast-forward.
    pub const FASTFORWARD: Self = Self(ffi::git_merge_analysis_t_GIT_MERGE_ANALYSIS_FASTFORWARD);
    /// `HEAD` is unborn and can be set directly to the target.
    pub const UNBORN: Self = Self(ffi::git_merge_analysis_t_GIT_MERGE_ANALYSIS_UNBORN);
    /// Every merge-analysis bit published by this version of libgit2.
    pub const ALL: Self =
        Self(Self::NORMAL.0 | Self::UP_TO_DATE.0 | Self::FASTFORWARD.0 | Self::UNBORN.0);

    /// Converts raw bits when they contain only published opportunities.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_merge_analysis_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 flag bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_merge_analysis_t {
        self.0
    }

    /// Returns whether no merge opportunity is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every opportunity in `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any opportunity in `other` is present.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitMergeAnalysis> for ffi::git_merge_analysis_t {
    fn from(analysis: GitMergeAnalysis) -> Self {
        analysis.bits()
    }
}

impl TryFrom<ffi::git_merge_analysis_t> for GitMergeAnalysis {
    type Error = ffi::git_merge_analysis_t;

    fn try_from(bits: ffi::git_merge_analysis_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitMergeAnalysis {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitMergeAnalysis {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitMergeAnalysis {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitMergeAnalysis {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitMergeAnalysis {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(self.0 ^ Self::ALL.0)
    }
}

/// Wraps: git_merge_file_favor_t
/// Selects which side wins conflicting regions during a file merge.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
#[repr(u32)]
pub enum MergeFileFavor {
    /// Record conflicts normally.
    #[default]
    Normal = ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_NORMAL,
    /// Keep the "ours" side of conflicting regions.
    Ours = ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_OURS,
    /// Keep the "theirs" side of conflicting regions.
    Theirs = ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_THEIRS,
    /// Combine the unique lines from both sides.
    Union = ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_UNION,
}

impl From<MergeFileFavor> for ffi::git_merge_file_favor_t {
    fn from(value: MergeFileFavor) -> Self {
        value as Self
    }
}

impl TryFrom<ffi::git_merge_file_favor_t> for MergeFileFavor {
    type Error = ffi::git_merge_file_favor_t;

    fn try_from(value: ffi::git_merge_file_favor_t) -> Result<Self, Self::Error> {
        match value {
            ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_NORMAL => Ok(Self::Normal),
            ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_OURS => Ok(Self::Ours),
            ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_THEIRS => Ok(Self::Theirs),
            ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_UNION => Ok(Self::Union),
            other => Err(other),
        }
    }
}

ffibox::define_ctype!(
    /// Wraps: git_merge_file_input
    /// A non-owning descriptor for one side of an in-memory file merge.
    MergeFileInput,
    MergeFileInputRef,
    MergeFileInputMut,
    ffi::git_merge_file_input
);

impl<'a> MergeFileInputRef<'a> {
    /// Field: git_merge_file_input.size
    /// Returns the number of bytes in the input contents.
    #[must_use]
    pub fn size(&self) -> usize {
        // SAFETY: this live shared handle covers the complete C value, and
        // raw-place projection reads the initialized scalar without forming a
        // reference to C-visible memory.
        unsafe { addr_of!((*self.as_ptr()).size).read() }
    }

    /// Field: git_merge_file_input.path
    /// Returns the optional NUL-terminated file name borrowed by this input.
    #[must_use]
    pub fn path(&self) -> Option<&'a CStr> {
        // SAFETY: raw-place projection reads the initialized pointer field
        // without forming a reference to the C value.
        let path = unsafe { addr_of!((*self.as_ptr()).path).read() };
        if path.is_null() {
            None
        } else {
            // SAFETY: a valid input requires a non-null path to remain a
            // NUL-terminated string for the input's usable lifetime. The
            // returned reference is bounded by this handle's borrow.
            Some(unsafe { CStr::from_ptr(path) })
        }
    }

    /// Field: git_merge_file_input.mode
    /// Returns the file mode, or zero when no mode should be merged.
    #[must_use]
    pub fn mode(&self) -> u32 {
        // SAFETY: as `size`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).mode).read() }
    }

    /// Field: git_merge_file_input.version
    /// Returns the ABI version of this input descriptor.
    #[must_use]
    pub fn version(&self) -> u32 {
        // SAFETY: as `size`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_merge_file_input.ptr
    /// Returns the counted, non-NUL-terminated input contents.
    ///
    /// `None` denotes the initialized empty representation with a null
    /// pointer. A non-null zero-length input is represented by an empty view.
    #[must_use]
    pub fn contents(&self) -> Option<CSlice<'a, u8>> {
        // SAFETY: both fields are initialized members of the live input and
        // are read by raw-place projection without forming references.
        let (ptr, size) = unsafe {
            (
                addr_of!((*self.as_ptr()).ptr)
                    .read()
                    .cast_mut()
                    .cast::<u8>(),
                addr_of!((*self.as_ptr()).size).read(),
            )
        };
        let ptr = NonNull::new(ptr)?;
        // SAFETY: a valid input exposes `size` initialized bytes at its
        // non-null content pointer. The view is bounded by the input handle's
        // lifetime and permits no mutation.
        Some(unsafe { CSlice::from_raw_parts(ptr, size) })
    }
}

impl MergeFileInputMut<'_> {
    /// Stores a borrowed content span and its byte count.
    ///
    /// # Safety
    ///
    /// A non-null `contents` span must remain alive and unchanged for every
    /// later use of the input value, including uses after this handle is
    /// released.
    pub unsafe fn set_borrowed_contents(&mut self, contents: Option<&[u8]>) {
        let (ptr, size) = contents.map_or((core::ptr::null(), 0), |contents| {
            (
                contents.as_ptr().cast::<core::ffi::c_char>(),
                contents.len(),
            )
        });
        let input = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits raw-place field writes, and
        // the caller upholds the stored pointer's lifetime and immutability.
        unsafe {
            addr_of_mut!((*input).ptr).write(ptr);
            addr_of_mut!((*input).size).write(size);
        }
    }

    /// Clears the borrowed content span.
    pub fn clear_contents(&mut self) {
        // SAFETY: storing the null empty representation creates no borrowed
        // pointer lifetime obligation.
        unsafe { self.set_borrowed_contents(None) }
    }

    /// Stores an optional borrowed NUL-terminated file name.
    ///
    /// # Safety
    ///
    /// A non-null `path` must remain alive and NUL-terminated for every later
    /// use of the input value, including uses after this handle is released.
    pub unsafe fn set_borrowed_path(&mut self, path: Option<&CStr>) {
        let path = path.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: this exclusive handle permits a raw-place field write, and
        // the caller upholds the stored pointer's lifetime contract.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).path).write(path) }
    }

    /// Clears the optional borrowed file name.
    pub fn clear_path(&mut self) {
        // SAFETY: storing null creates no borrowed-pointer lifetime
        // obligation.
        unsafe { self.set_borrowed_path(None) }
    }

    /// Sets the file mode, using zero to omit mode merging.
    pub fn set_mode(&mut self, mode: u32) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).mode).write(mode) }
    }

    /// Sets the ABI version of this input descriptor.
    pub fn set_version(&mut self, version: u32) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }
}

ffibox::define_ctype!(
    /// Wraps: git_merge_file_result
    /// An inline result header that owns its optional path and merged bytes.
    MergeFileResult,
    MergeFileResultRef,
    MergeFileResultMut,
    ffi::git_merge_file_result
);

// `git_merge_file_result_free` releases the result's fields but retains its
// inline header.
ffibox::impl_cvalued!(
    MergeFileResult,
    ffi::git_merge_file_result,
    ffi::git_merge_file_result_free
);

/// Owned path allocation stored in a [`MergeFileResult`].
pub type MergeFilePath = CrustifyStr<GitStrdupFree>;

/// Deleter for the counted byte allocation stored in a
/// [`MergeFileResult`].
pub struct MergeFileContentsFree;

// SAFETY: valid merge results store a uniquely owned byte allocation made by
// libgit2's configured allocator. The installed allocator must remain
// compatible until the allocation is dropped, as for `GitStrdupFree`.
unsafe impl CLenDropped for MergeFileContentsFree {
    unsafe fn c_drop_len(ptr: *mut u8, _byte_len: usize) {
        // SAFETY: the `CLenDropped` contract proves that `ptr` is the unique
        // libgit2 allocation moved out of a valid merge result.
        unsafe { ffi::crustify_git__free(ptr.cast()) }
    }
}

/// Owned merged bytes detached from a [`MergeFileResult`].
pub type MergeFileContents = CVec<u8, MergeFileContentsFree>;

impl MergeFileResult {
    /// Constructs an empty owned result whose fields are disposed on drop.
    #[must_use]
    pub fn new() -> CVal<Self> {
        // Bindgen's C layout consists only of integers and pointers, and this
        // is the empty result representation used by libgit2.
        CVal::new(Self::zeroed())
    }
}

impl<'a> MergeFileResultRef<'a> {
    /// Field: git_merge_file_result.automergeable
    /// Reports whether the output was merged without conflict markers.
    #[must_use]
    pub fn is_automergeable(&self) -> bool {
        // SAFETY: this live shared handle permits a raw-place read of the
        // initialized scalar without forming a reference to C-owned memory.
        unsafe { addr_of!((*self.as_ptr()).automergeable).read() != 0 }
    }

    /// Field: git_merge_file_result.path
    /// Borrows the selected result path, or returns `None` for a path conflict.
    #[must_use]
    pub fn path(&self) -> Option<&'a CStr> {
        // SAFETY: raw-place projection reads the initialized pointer field
        // without forming a reference to the result header.
        let path = unsafe { addr_of!((*self.as_ptr()).path).read() };
        if path.is_null() {
            return None;
        }

        // SAFETY: a valid merge result's non-null path is NUL-terminated and
        // remains owned by the result for the handle's lifetime.
        Some(unsafe { CStr::from_ptr(path) })
    }

    /// Field: git_merge_file_result.mode
    /// Returns the file mode selected for the merged result.
    #[must_use]
    pub fn mode(&self) -> u32 {
        // SAFETY: as `is_automergeable`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).mode).read() }
    }

    /// Field: git_merge_file_result.ptr
    /// Borrows the counted merged bytes.
    ///
    /// A null pointer is represented by `None`, including the empty result.
    #[must_use]
    pub fn contents(&self) -> Option<CSlice<'a, u8>> {
        // SAFETY: both fields are initialized members of this live shared
        // result and are read by raw-place projection.
        let (ptr, len) = unsafe {
            (
                addr_of!((*self.as_ptr()).ptr)
                    .read()
                    .cast_mut()
                    .cast::<u8>(),
                addr_of!((*self.as_ptr()).len).read(),
            )
        };
        let ptr = NonNull::new(ptr)?;
        // SAFETY: a valid result owns at least `len` initialized bytes at its
        // non-null content pointer, and the view is tied to the result handle.
        Some(unsafe { CSlice::from_raw_parts(ptr, len) })
    }

    /// Field: git_merge_file_result.len
    /// Returns the number of initialized merged bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        // SAFETY: as `is_automergeable`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).len).read() }
    }

    /// Returns whether the merged byte span is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl MergeFileResultMut<'_> {
    /// Sets whether the result was merged without conflict markers.
    pub fn set_automergeable(&mut self, automergeable: bool) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).automergeable).write(u32::from(automergeable)) }
    }

    /// Sets the file mode selected for the merged result.
    pub fn set_mode(&mut self, mode: u32) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).mode).write(mode) }
    }

    /// Moves the owned path out, leaving the result's path empty.
    #[must_use]
    pub fn take_path(&mut self) -> Option<MergeFilePath> {
        let result = self.as_mut_ptr();
        // SAFETY: this exclusive handle permits reading and clearing the owned
        // pointer field. Clearing it transfers its unique ownership out.
        let path = unsafe {
            let path = addr_of!((*result).path).read().cast_mut();
            addr_of_mut!((*result).path).write(core::ptr::null());
            path
        };
        // SAFETY: a non-null path moved from a valid merge result is a unique,
        // NUL-terminated allocation from libgit2's configured allocator.
        unsafe { MergeFilePath::from_raw(path) }
    }

    /// Replaces the owned path and disposes the previous allocation.
    pub fn set_path(&mut self, path: Option<MergeFilePath>) {
        let path = path.map_or(core::ptr::null_mut(), MergeFilePath::into_raw);
        let old = self.take_path();
        // SAFETY: the exclusive handle permits installing ownership of the
        // compatible path allocation after the old pointer was cleared.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).path).write(path) }
        drop(old);
    }

    /// Moves the owned merged bytes out, leaving a null, empty span.
    #[must_use]
    pub fn take_contents(&mut self) -> Option<MergeFileContents> {
        let result = self.as_mut_ptr();
        // SAFETY: the exclusive handle permits reading and clearing both
        // ownership fields as one transfer, leaving a valid empty result.
        let (ptr, len) = unsafe {
            let ptr = addr_of!((*result).ptr).read().cast_mut().cast::<u8>();
            let len = addr_of!((*result).len).read();
            addr_of_mut!((*result).ptr).write(core::ptr::null());
            addr_of_mut!((*result).len).write(0);
            (ptr, len)
        };
        // SAFETY: a non-null pointer moved from a valid result uniquely owns
        // `len` initialized bytes from libgit2's configured allocator.
        unsafe { MergeFileContents::from_raw_parts(ptr, len) }
    }

    /// Replaces the owned merged bytes and disposes the previous allocation.
    pub fn set_contents(&mut self, contents: Option<MergeFileContents>) {
        let (ptr, len) = contents.map_or((core::ptr::null_mut(), 0), CVec::into_raw_parts);
        let old = self.take_contents();
        let result = self.as_mut_ptr();
        // SAFETY: the exclusive handle permits installing the compatible
        // allocation and its exact initialized length after the old span was
        // cleared.
        unsafe {
            addr_of_mut!((*result).ptr).write(ptr.cast());
            addr_of_mut!((*result).len).write(len);
        }
        drop(old);
    }
}

/// Wraps: git_merge_preference_t
/// The user's configured preference for fast-forward merges.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[repr(u32)]
pub enum MergePreference {
    /// No merge preference was configured.
    #[default]
    None = ffi::git_merge_preference_t_GIT_MERGE_PREFERENCE_NONE,
    /// Do not perform a fast-forward merge.
    NoFastForward = ffi::git_merge_preference_t_GIT_MERGE_PREFERENCE_NO_FASTFORWARD,
    /// Only perform a fast-forward merge.
    FastForwardOnly = ffi::git_merge_preference_t_GIT_MERGE_PREFERENCE_FASTFORWARD_ONLY,
}

impl From<MergePreference> for ffi::git_merge_preference_t {
    fn from(preference: MergePreference) -> Self {
        preference as Self
    }
}

impl TryFrom<ffi::git_merge_preference_t> for MergePreference {
    type Error = ffi::git_merge_preference_t;

    fn try_from(preference: ffi::git_merge_preference_t) -> Result<Self, Self::Error> {
        match preference {
            ffi::git_merge_preference_t_GIT_MERGE_PREFERENCE_NONE => Ok(Self::None),
            ffi::git_merge_preference_t_GIT_MERGE_PREFERENCE_NO_FASTFORWARD => {
                Ok(Self::NoFastForward)
            }
            ffi::git_merge_preference_t_GIT_MERGE_PREFERENCE_FASTFORWARD_ONLY => {
                Ok(Self::FastForwardOnly)
            }
            other => Err(other),
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn analysis_flags_form_valid_combinations() {
        let mut analysis = GitMergeAnalysis::NORMAL;
        analysis |= GitMergeAnalysis::FASTFORWARD;

        assert!(analysis.contains(GitMergeAnalysis::NORMAL));
        assert!(analysis.intersects(GitMergeAnalysis::FASTFORWARD));
        assert!(!analysis.intersects(GitMergeAnalysis::UNBORN));
        assert_eq!(GitMergeAnalysis::from_bits(analysis.bits()), Some(analysis));
        assert!(GitMergeAnalysis::NONE.is_empty());
    }

    #[test]
    fn unknown_analysis_bits_are_rejected() {
        let unknown = GitMergeAnalysis::ALL.bits() << 1;
        assert_eq!(GitMergeAnalysis::from_bits(unknown), None);
        assert_eq!(GitMergeAnalysis::try_from(unknown), Err(unknown));
    }

    #[test]
    fn analysis_flags_match_the_c_abi_scalar() {
        assert_eq!(
            size_of::<GitMergeAnalysis>(),
            size_of::<ffi::git_merge_analysis_t>()
        );
        assert_eq!(
            align_of::<GitMergeAnalysis>(),
            align_of::<ffi::git_merge_analysis_t>()
        );
    }

    #[test]
    fn merge_file_favor_preserves_the_c_layout_and_validates_values() {
        assert_eq!(
            size_of::<MergeFileFavor>(),
            size_of::<ffi::git_merge_file_favor_t>()
        );
        assert_eq!(
            align_of::<MergeFileFavor>(),
            align_of::<ffi::git_merge_file_favor_t>()
        );
        assert_eq!(ffi::git_merge_file_favor_t::from(MergeFileFavor::Union), 3);
        assert_eq!(MergeFileFavor::try_from(2), Ok(MergeFileFavor::Theirs));
        assert_eq!(MergeFileFavor::try_from(4), Err(4));
    }

    #[test]
    fn merge_file_input_preserves_the_c_layout() {
        assert_eq!(
            size_of::<MergeFileInput>(),
            size_of::<ffi::git_merge_file_input>()
        );
        assert_eq!(
            align_of::<MergeFileInput>(),
            align_of::<ffi::git_merge_file_input>()
        );
        assert_eq!(
            size_of::<MergeFileInputRef<'_>>(),
            size_of::<*const ffi::git_merge_file_input>()
        );
        assert_eq!(
            size_of::<MergeFileInputMut<'_>>(),
            size_of::<*mut ffi::git_merge_file_input>()
        );
    }

    #[test]
    fn merge_file_input_handles_read_and_write_fields() {
        let contents = b"left\0right";
        let path = c"conflict.txt";
        let mut input = MergeFileInput::zeroed();
        let raw = addr_of_mut!(input).cast::<ffi::git_merge_file_input>();

        // SAFETY: `raw` points to the live, initialized, layout-compatible
        // stack value above and this is its only active handle.
        let mut input = unsafe { MergeFileInputMut::from_ptr(raw) }
            .expect("the address of a stack value is non-null");
        input.set_version(1);
        input.set_mode(0o100644);
        // SAFETY: both test values outlive every later use of the input.
        unsafe {
            input.set_borrowed_contents(Some(contents));
            input.set_borrowed_path(Some(path));
        }

        let shared = input.as_ref();
        assert_eq!(shared.version(), 1);
        assert_eq!(shared.mode(), 0o100644);
        assert_eq!(shared.size(), contents.len());
        assert_eq!(shared.path(), Some(path));
        let bytes = shared.contents().expect("the stored span is non-null");
        let mut copied = [0; 10];
        assert!(bytes.copy_to_slice(&mut copied));
        assert_eq!(&copied, contents);

        input.clear_contents();
        input.clear_path();
        assert!(input.as_ref().contents().is_none());
        assert!(input.as_ref().path().is_none());
        assert_eq!(input.as_ref().size(), 0);
    }
    #[test]
    fn merge_file_result_preserves_the_c_layout() {
        assert_eq!(
            size_of::<MergeFileResult>(),
            size_of::<ffi::git_merge_file_result>()
        );
        assert_eq!(
            align_of::<MergeFileResult>(),
            align_of::<ffi::git_merge_file_result>()
        );
        assert_eq!(
            size_of::<MergeFileResultRef<'_>>(),
            size_of::<*const ffi::git_merge_file_result>()
        );
        assert_eq!(
            size_of::<MergeFileResultMut<'_>>(),
            size_of::<*mut ffi::git_merge_file_result>()
        );
    }

    #[test]
    fn empty_owned_result_supports_shared_and_exclusive_handles() {
        let mut result = MergeFileResult::new();
        assert!(result.as_ref().path().is_none());
        assert!(result.as_ref().contents().is_none());
        assert!(result.as_ref().is_empty());

        let mut result_mut = result.as_mut();
        result_mut.set_automergeable(true);
        result_mut.set_mode(0o100644);

        let result_ref = result_mut.as_ref();
        assert!(result_ref.is_automergeable());
        assert_eq!(result_ref.mode(), 0o100644);
    }

    #[test]
    fn owned_fields_move_through_the_result_without_double_free() {
        // SAFETY: libgit2 initialization is refcounted and this successful
        // call is balanced after all configured allocations are dropped.
        let initialized = unsafe { ffi::git_libgit2_init() };
        assert!(initialized > 0);

        // SAFETY: libgit2 is initialized, so successful calls return fresh
        // allocations from its configured allocator.
        let raw_path = unsafe { ffi::crustify_git__strdup(c"merged.txt".as_ptr()) };
        // SAFETY: `raw_path` is null or the unique NUL-terminated allocation
        // just returned by libgit2.
        let path = unsafe { MergeFilePath::from_raw(raw_path) }.unwrap();

        // SAFETY: libgit2 is initialized; a non-null result uniquely owns four
        // writable bytes from its configured allocator.
        let raw_contents = unsafe { ffi::crustify_git__malloc(4) }.cast::<u8>();
        assert!(!raw_contents.is_null());
        // SAFETY: the allocation above contains four writable bytes.
        unsafe { core::ptr::copy_nonoverlapping(b"text".as_ptr(), raw_contents, 4) };
        // SAFETY: all four uniquely owned bytes are initialized and use the
        // lifecycle strategy for libgit2's configured allocator.
        let contents = unsafe { MergeFileContents::from_raw_parts(raw_contents, 4) }.unwrap();

        let mut result = MergeFileResult::new();
        result.as_mut().set_path(Some(path));
        result.as_mut().set_contents(Some(contents));
        assert_eq!(result.as_ref().path(), Some(c"merged.txt"));
        assert_eq!(result.as_ref().len(), 4);
        let view = result.as_ref().contents().unwrap();
        let mut copied = [0; 4];
        assert!(view.copy_to_slice(&mut copied));
        assert_eq!(&copied, b"text");

        let path = result.as_mut().take_path().unwrap();
        let contents = result.as_mut().take_contents().unwrap();
        assert!(result.as_ref().path().is_none());
        assert!(result.as_ref().contents().is_none());
        drop(path);
        drop(contents);
        drop(result);

        // SAFETY: balances this test's successful initialization call.
        let remaining = unsafe { ffi::git_libgit2_shutdown() };
        assert!(remaining >= 0);
    }

    #[test]
    fn merge_preferences_validate_raw_values() {
        for preference in [
            MergePreference::None,
            MergePreference::NoFastForward,
            MergePreference::FastForwardOnly,
        ] {
            let raw = ffi::git_merge_preference_t::from(preference);
            assert_eq!(MergePreference::try_from(raw), Ok(preference));
        }
        assert_eq!(MergePreference::try_from(3), Err(3));
        assert_eq!(
            size_of::<MergePreference>(),
            size_of::<ffi::git_merge_preference_t>()
        );
        assert_eq!(
            align_of::<MergePreference>(),
            align_of::<ffi::git_merge_preference_t>()
        );
    }

    #[test]
    fn published_analysis_bits_match_the_c_constants() {
        for (analysis, raw) in [
            (
                GitMergeAnalysis::NONE,
                ffi::git_merge_analysis_t_GIT_MERGE_ANALYSIS_NONE,
            ),
            (
                GitMergeAnalysis::NORMAL,
                ffi::git_merge_analysis_t_GIT_MERGE_ANALYSIS_NORMAL,
            ),
            (
                GitMergeAnalysis::UP_TO_DATE,
                ffi::git_merge_analysis_t_GIT_MERGE_ANALYSIS_UP_TO_DATE,
            ),
            (
                GitMergeAnalysis::FASTFORWARD,
                ffi::git_merge_analysis_t_GIT_MERGE_ANALYSIS_FASTFORWARD,
            ),
            (
                GitMergeAnalysis::UNBORN,
                ffi::git_merge_analysis_t_GIT_MERGE_ANALYSIS_UNBORN,
            ),
        ] {
            assert_eq!(analysis.bits(), raw);
            assert_eq!(GitMergeAnalysis::from_bits(raw), Some(analysis));
        }
        assert_eq!(GitMergeAnalysis::default(), GitMergeAnalysis::NONE);
        assert_eq!(!GitMergeAnalysis::NONE, GitMergeAnalysis::ALL);
        assert!(GitMergeAnalysis::ALL.contains(GitMergeAnalysis::NONE));
    }

    #[test]
    fn every_combination_git_merge_analysis_reports_is_accepted() {
        // The four results `git_merge_analysis_for_ref` can OR into its
        // cleared out-slot on success.
        for reported in [
            GitMergeAnalysis::UP_TO_DATE,
            GitMergeAnalysis::NORMAL,
            GitMergeAnalysis::FASTFORWARD | GitMergeAnalysis::NORMAL,
            GitMergeAnalysis::FASTFORWARD | GitMergeAnalysis::UNBORN,
        ] {
            assert_eq!(GitMergeAnalysis::try_from(reported.bits()), Ok(reported));
            assert!(!reported.is_empty());
        }
    }

    #[test]
    fn every_favor_value_round_trips_through_the_c_scalar() {
        for (favor, raw) in [
            (
                MergeFileFavor::Normal,
                ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_NORMAL,
            ),
            (
                MergeFileFavor::Ours,
                ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_OURS,
            ),
            (
                MergeFileFavor::Theirs,
                ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_THEIRS,
            ),
            (
                MergeFileFavor::Union,
                ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_UNION,
            ),
        ] {
            assert_eq!(ffi::git_merge_file_favor_t::from(favor), raw);
            assert_eq!(MergeFileFavor::try_from(raw), Ok(favor));
        }
        // The favors are dense and mutually exclusive, so the first
        // unpublished value is one past the last.
        assert_eq!(MergeFileFavor::default(), MergeFileFavor::Normal);
        assert_eq!(
            MergeFileFavor::try_from(ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_UNION + 1),
            Err(ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_UNION + 1)
        );
    }

    #[test]
    fn a_non_null_empty_content_span_is_an_empty_view() {
        let mut raw = ffi::git_merge_file_input {
            version: 1,
            ptr: b"".as_ptr().cast(),
            size: 0,
            path: core::ptr::null(),
            mode: 0,
        };
        // SAFETY: `raw` is live initialized stack storage and this shared
        // handle is its only borrow for the rest of the test.
        let input = unsafe { MergeFileInputRef::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");
        let contents = input
            .contents()
            .expect("a non-null span is a view, not `None`");
        assert_eq!(contents.len(), 0);
        assert!(input.path().is_none());
    }
}

pub use crate::api::merge::MergeFileFlags;

ffibox::define_ctype!(
    /// Wraps: git_merge_file_options
    /// Borrowing options that control a file-level merge.
    ///
    /// The value owns nothing: `merge_file_normalize_opts` shallow-copies the
    /// whole struct, labels included, and no consumer releases them, so a
    /// non-null label must outlive every use of the options. `version` is
    /// declarative here — only `git_merge_file_options_init` reads it, to pick
    /// the defaults it writes back; the merge entry points never check it.
    MergeFileOptions,
    MergeFileOptionsRef,
    MergeFileOptionsMut,
    ffi::git_merge_file_options
);

impl<'a> MergeFileOptionsRef<'a> {
    /// Field: git_merge_file_options.flags
    /// Returns the validated file-merge behavior flags.
    pub fn flags(&self) -> Result<MergeFileFlags, ffi::git_merge_file_flag_t> {
        // SAFETY: this live shared handle permits a raw-place scalar read
        // without forming a reference to C-visible memory.
        let flags = unsafe { addr_of!((*self.as_ptr()).flags).read() };
        MergeFileFlags::from_bits(flags).ok_or(flags)
    }

    /// Field: git_merge_file_options.version
    /// Returns the ABI version of this options value.
    #[must_use]
    pub fn version(&self) -> u32 {
        // SAFETY: as `flags`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_merge_file_options.favor
    /// Returns the validated conflict-side preference.
    pub fn favor(&self) -> Result<MergeFileFavor, ffi::git_merge_file_favor_t> {
        // SAFETY: as `flags`, for this initialized scalar field.
        let favor = unsafe { addr_of!((*self.as_ptr()).favor).read() };
        MergeFileFavor::try_from(favor)
    }

    /// Field: git_merge_file_options.marker_size
    /// Returns the requested conflict-marker width, or zero for the default.
    #[must_use]
    pub fn marker_size(&self) -> u16 {
        // SAFETY: as `flags`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).marker_size).read() }
    }

    /// Field: git_merge_file_options.their_label
    /// Returns the optional label borrowed for the "theirs" side.
    #[must_use]
    pub fn their_label(&self) -> Option<&'a CStr> {
        // SAFETY: raw-place projection reads the initialized pointer field
        // without forming a reference to C-visible memory.
        let label = unsafe { addr_of!((*self.as_ptr()).their_label).read() };
        // SAFETY: a valid options value requires this non-null label to stay
        // NUL-terminated and live for the handle lifetime.
        unsafe { optional_borrowed_label(label) }
    }

    /// Field: git_merge_file_options.our_label
    /// Returns the optional label borrowed for the "ours" side.
    #[must_use]
    pub fn our_label(&self) -> Option<&'a CStr> {
        // SAFETY: as `their_label`, for this initialized pointer field.
        let label = unsafe { addr_of!((*self.as_ptr()).our_label).read() };
        // SAFETY: as `their_label`, for this borrowed string field.
        unsafe { optional_borrowed_label(label) }
    }

    /// Field: git_merge_file_options.ancestor_label
    /// Returns the optional label borrowed for the common ancestor.
    #[must_use]
    pub fn ancestor_label(&self) -> Option<&'a CStr> {
        // SAFETY: as `their_label`, for this initialized pointer field.
        let label = unsafe { addr_of!((*self.as_ptr()).ancestor_label).read() };
        // SAFETY: as `their_label`, for this borrowed string field.
        unsafe { optional_borrowed_label(label) }
    }
}

unsafe fn optional_borrowed_label<'a>(label: *const core::ffi::c_char) -> Option<&'a CStr> {
    if label.is_null() {
        None
    } else {
        // SAFETY: every non-null label in a valid options value is a
        // NUL-terminated string that remains live for the handle lifetime;
        // the caller obtains this helper only through such a handle getter.
        Some(unsafe { CStr::from_ptr(label) })
    }
}

impl MergeFileOptionsMut<'_> {
    /// Sets the file-merge behavior flags.
    pub fn set_flags(&mut self, flags: MergeFileFlags) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags.bits()) }
    }

    /// Sets the ABI version of this options value.
    pub fn set_version(&mut self, version: u32) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Sets which side wins conflicting regions.
    pub fn set_favor(&mut self, favor: MergeFileFavor) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).favor).write(favor.into()) }
    }

    /// Sets the conflict-marker width, using zero for libgit2's default.
    pub fn set_marker_size(&mut self, marker_size: u16) {
        // SAFETY: this exclusive handle permits a raw-place scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).marker_size).write(marker_size) }
    }

    /// Stores an optional caller-owned label for the "theirs" side.
    ///
    /// # Safety
    ///
    /// A non-null `label` must remain alive and NUL-terminated for every later
    /// use of the options value, including uses after this handle is released.
    pub unsafe fn set_borrowed_their_label(&mut self, label: Option<&CStr>) {
        let label = label.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: this exclusive handle permits the raw-place field write, and
        // the caller upholds the stored string's lifetime contract.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).their_label).write(label) }
    }

    /// Stores an optional caller-owned label for the "ours" side.
    ///
    /// # Safety
    ///
    /// A non-null `label` must remain alive and NUL-terminated for every later
    /// use of the options value, including uses after this handle is released.
    pub unsafe fn set_borrowed_our_label(&mut self, label: Option<&CStr>) {
        let label = label.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: as `set_borrowed_their_label`, for this pointer field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).our_label).write(label) }
    }

    /// Stores an optional caller-owned label for the common ancestor.
    ///
    /// # Safety
    ///
    /// A non-null `label` must remain alive and NUL-terminated for every later
    /// use of the options value, including uses after this handle is released.
    pub unsafe fn set_borrowed_ancestor_label(&mut self, label: Option<&CStr>) {
        let label = label.map_or(core::ptr::null(), CStr::as_ptr);
        // SAFETY: as `set_borrowed_their_label`, for this pointer field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).ancestor_label).write(label) }
    }

    /// Clears the optional "theirs" label.
    pub fn clear_their_label(&mut self) {
        // SAFETY: storing null creates no borrowed-pointer lifetime obligation.
        unsafe { self.set_borrowed_their_label(None) }
    }

    /// Clears the optional "ours" label.
    pub fn clear_our_label(&mut self) {
        // SAFETY: storing null creates no borrowed-pointer lifetime obligation.
        unsafe { self.set_borrowed_our_label(None) }
    }

    /// Clears the optional common-ancestor label.
    pub fn clear_ancestor_label(&mut self) {
        // SAFETY: storing null creates no borrowed-pointer lifetime obligation.
        unsafe { self.set_borrowed_ancestor_label(None) }
    }
}

#[cfg(test)]
mod file_options_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn file_options_wrapper_preserves_the_c_layout() {
        assert_eq!(
            size_of::<MergeFileOptions>(),
            size_of::<ffi::git_merge_file_options>()
        );
        assert_eq!(
            align_of::<MergeFileOptions>(),
            align_of::<ffi::git_merge_file_options>()
        );
        assert_eq!(
            size_of::<MergeFileOptionsRef<'_>>(),
            size_of::<*const ffi::git_merge_file_options>()
        );
        assert_eq!(
            size_of::<MergeFileOptionsMut<'_>>(),
            size_of::<*mut ffi::git_merge_file_options>()
        );
    }

    #[test]
    fn file_options_handles_validate_and_update_every_field() {
        let mut storage = MergeFileOptions::zeroed();
        let raw = addr_of_mut!(storage).cast::<ffi::git_merge_file_options>();

        // SAFETY: `raw` points to the live, initialized, layout-compatible
        // stack value above and this is its only active handle.
        let mut options = unsafe { MergeFileOptionsMut::from_ptr(raw) }
            .expect("the address of stack storage is non-null");
        let flags = MergeFileFlags::STYLE_DIFF3 | MergeFileFlags::DIFF_PATIENCE;
        options.set_version(1);
        options.set_flags(flags);
        options.set_favor(MergeFileFavor::Theirs);
        options.set_marker_size(11);
        // SAFETY: all three C string literals have static storage duration.
        unsafe {
            options.set_borrowed_ancestor_label(Some(c"base"));
            options.set_borrowed_our_label(Some(c"ours"));
            options.set_borrowed_their_label(Some(c"theirs"));
        }

        let shared = options.as_ref();
        assert_eq!(shared.version(), 1);
        assert_eq!(shared.flags(), Ok(flags));
        assert_eq!(shared.favor(), Ok(MergeFileFavor::Theirs));
        assert_eq!(shared.marker_size(), 11);
        assert_eq!(shared.ancestor_label(), Some(c"base"));
        assert_eq!(shared.our_label(), Some(c"ours"));
        assert_eq!(shared.their_label(), Some(c"theirs"));

        options.clear_ancestor_label();
        options.clear_our_label();
        options.clear_their_label();
        assert_eq!(options.as_ref().ancestor_label(), None);
        assert_eq!(options.as_ref().our_label(), None);
        assert_eq!(options.as_ref().their_label(), None);
    }

    #[test]
    fn unknown_file_option_values_are_rejected() {
        let unknown_flags = MergeFileFlags::ALL.bits() << 1;
        assert_eq!(MergeFileFlags::from_bits(unknown_flags), None);
        assert!(MergeFileFlags::DEFAULT.is_empty());

        let mut raw = ffi::git_merge_file_options {
            version: 1,
            ancestor_label: core::ptr::null(),
            our_label: core::ptr::null(),
            their_label: core::ptr::null(),
            favor: 99,
            flags: unknown_flags,
            marker_size: 0,
        };
        // SAFETY: `raw` is live initialized stack storage used only through
        // this shared handle for the rest of the test.
        let options = unsafe { MergeFileOptionsRef::from_ptr(&raw mut raw) }.unwrap();
        assert_eq!(options.flags(), Err(unknown_flags));
        assert_eq!(options.favor(), Err(99));
    }

    #[test]
    fn file_input_initializer_builds_the_public_default() {
        let mut input = git_merge_file_input_init(1).expect("version one is supported");
        // SAFETY: `input` is live initialized wrapper storage and this handle
        // is its only active borrow.
        let input = unsafe {
            MergeFileInputRef::from_ptr(addr_of_mut!(input).cast::<ffi::git_merge_file_input>())
        }
        .unwrap();
        assert_eq!(input.version(), 1);
        assert_eq!(input.size(), 0);
        assert!(input.contents().is_none());
        assert!(input.path().is_none());
    }

    #[test]
    fn file_input_initializer_rejects_an_unsupported_version() {
        // SAFETY: libgit2 initialization is refcounted and balanced below.
        // The version check records into thread-local error state, which only
        // initialization creates.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);

        // `GIT_MERGE_FILE_INPUT_VERSION` is 1, and the header's check rejects
        // both zero and anything above the current version.
        // `MergeFileInput` is a C layout newtype, so match the status rather
        // than comparing the whole `Result`.
        assert!(matches!(git_merge_file_input_init(0), Err(-1)));
        assert!(matches!(git_merge_file_input_init(2), Err(-1)));

        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}

/// Wraps: git_merge_base_octopus
/// Finds a merge base for at least two commit IDs.
pub fn git_merge_base_octopus(
    repo: GitRepositoryRef<'_>,
    inputs: CSlice<'_, Oid>,
) -> Result<Oid, i32> {
    if inputs.len() < 2 {
        return Err(-1);
    }
    let mut out = Oid::zeroed();
    // SAFETY: `out` is writable, `repo` is live for this call, and `inputs`
    // describes `len` initialized OIDs. Libgit2 retains none of the pointers.
    let status = unsafe {
        ffi::git_merge_base_octopus(
            addr_of_mut!(out).cast(),
            repo.as_ptr().cast_mut(),
            inputs.len(),
            inputs.as_ptr(),
        )
    };
    if status == 0 { Ok(out) } else { Err(status) }
}

/// Wraps: git_merge_file_input_init
/// Creates a merge-file input initialized for `version`.
pub fn git_merge_file_input_init(version: u32) -> Result<MergeFileInput, i32> {
    let mut input = MergeFileInput::zeroed();
    // SAFETY: `input` is writable layout-compatible storage and the C
    // initializer retains no pointer to it.
    let status = unsafe { ffi::git_merge_file_input_init(addr_of_mut!(input).cast(), version) };
    if status == 0 { Ok(input) } else { Err(status) }
}

/// Failure returned by the merge-analysis wrappers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MergeAnalysisError {
    /// Libgit2 returned an error status.
    Libgit2(i32),
    /// C returned analysis bits unknown to this build's headers.
    InvalidAnalysis(ffi::git_merge_analysis_t),
    /// C returned an unknown merge preference.
    InvalidPreference(ffi::git_merge_preference_t),
}

fn validate_analysis(
    status: i32,
    analysis: ffi::git_merge_analysis_t,
    preference: ffi::git_merge_preference_t,
) -> Result<(GitMergeAnalysis, MergePreference), MergeAnalysisError> {
    if status != 0 {
        return Err(MergeAnalysisError::Libgit2(status));
    }
    let analysis =
        GitMergeAnalysis::try_from(analysis).map_err(MergeAnalysisError::InvalidAnalysis)?;
    let preference =
        MergePreference::try_from(preference).map_err(MergeAnalysisError::InvalidPreference)?;
    Ok((analysis, preference))
}

/// Wraps: git_merge_analysis
/// Analyzes the single merge head supported by this libgit2 implementation.
pub fn git_merge_analysis(
    repository: GitRepositoryRef<'_>,
    their_head: AnnotatedCommitRef<'_>,
) -> Result<(GitMergeAnalysis, MergePreference), MergeAnalysisError> {
    let mut analysis = 0;
    let mut preference = 0;
    let mut head = their_head.as_ptr();
    // SAFETY: both output slots are writable, the repository and annotated
    // commit remain live, and the one-element pointer array is live for the
    // complete non-retaining call.
    let status = unsafe {
        ffi::git_merge_analysis(
            &mut analysis,
            &mut preference,
            repository.as_ptr().cast_mut(),
            core::ptr::addr_of_mut!(head),
            1,
        )
    };
    validate_analysis(status, analysis, preference)
}

/// Wraps: git_merge_analysis_for_ref
/// Analyzes one merge head relative to `our_reference`.
pub fn git_merge_analysis_for_ref(
    repository: GitRepositoryRef<'_>,
    our_reference: GitReferenceRef<'_>,
    their_head: AnnotatedCommitRef<'_>,
) -> Result<(GitMergeAnalysis, MergePreference), MergeAnalysisError> {
    let mut analysis = 0;
    let mut preference = 0;
    let mut head = their_head.as_ptr();
    // SAFETY: both outputs are writable, every handle is live, and the local
    // one-element pointer array remains valid for this non-retaining call.
    let status = unsafe {
        ffi::git_merge_analysis_for_ref(
            &mut analysis,
            &mut preference,
            repository.as_ptr().cast_mut(),
            our_reference.as_ptr().cast_mut(),
            core::ptr::addr_of_mut!(head),
            1,
        )
    };
    validate_analysis(status, analysis, preference)
}

fn oid_output(status: i32, output: Oid) -> Result<Oid, i32> {
    if status == 0 { Ok(output) } else { Err(status) }
}

/// Wraps: git_merge_base
/// Finds a best common ancestor of two commits.
pub fn git_merge_base(
    repository: GitRepositoryRef<'_>,
    one: OidRef<'_>,
    two: OidRef<'_>,
) -> Result<Oid, i32> {
    let mut output = Oid::zeroed();
    // SAFETY: output is writable layout-compatible storage and every input
    // handle remains live for this non-retaining graph walk.
    let status = unsafe {
        ffi::git_merge_base(
            core::ptr::addr_of_mut!(output).cast(),
            repository.as_ptr().cast_mut(),
            one.as_ptr(),
            two.as_ptr(),
        )
    };
    oid_output(status, output)
}

/// Wraps: git_merge_base_many
/// Finds a best common ancestor of at least two commits.
pub fn git_merge_base_many(
    repository: GitRepositoryRef<'_>,
    commits: &[OidRef<'_>],
) -> Result<Oid, i32> {
    if commits.len() < 2 {
        return Err(-1);
    }
    let raw: Vec<ffi::git_oid> = commits
        .iter()
        .map(|oid| {
            // SAFETY: `git_oid` is a plain bindgen C value with no destructor;
            // this copies its initialized bytes without forming a reference to
            // the C-visible object covered by the handle.
            unsafe { oid.as_ptr().read() }
        })
        .collect();
    let mut output = Oid::zeroed();
    // SAFETY: output is writable, `raw` is a contiguous array of complete
    // copied OIDs, and the repository remains live for the call.
    let status = unsafe {
        ffi::git_merge_base_many(
            core::ptr::addr_of_mut!(output).cast(),
            repository.as_ptr().cast_mut(),
            raw.len(),
            raw.as_ptr(),
        )
    };
    oid_output(status, output)
}

/// Wraps: git_merge_bases
/// Finds all best common ancestors of two commits.
pub fn git_merge_bases(
    repository: GitRepositoryRef<'_>,
    one: OidRef<'_>,
    two: OidRef<'_>,
) -> Result<CVal<crate::oidarray::OidArray>, i32> {
    let mut output = crate::oidarray::OidArray::new();
    let status = {
        let mut out = output.as_mut();
        // SAFETY: the output header is empty and exclusively borrowed; all
        // inputs remain live and C retains none of their pointers.
        unsafe {
            ffi::git_merge_bases(
                out.as_mut_ptr(),
                repository.as_ptr().cast_mut(),
                one.as_ptr(),
                two.as_ptr(),
            )
        }
    };
    if status == 0 { Ok(output) } else { Err(status) }
}

/// Wraps: git_merge_bases_many
/// Finds all best common ancestors for at least two commits.
pub fn git_merge_bases_many(
    repository: GitRepositoryRef<'_>,
    commits: &[OidRef<'_>],
) -> Result<CVal<crate::oidarray::OidArray>, i32> {
    if commits.len() < 2 {
        return Err(-1);
    }
    let raw: Vec<ffi::git_oid> = commits
        .iter()
        .map(|oid| {
            // SAFETY: a live OID handle addresses one initialized plain C
            // value; copying it forms no reference to C-visible memory.
            unsafe { oid.as_ptr().read() }
        })
        .collect();
    let mut output = crate::oidarray::OidArray::new();
    let status = {
        let mut out = output.as_mut();
        // SAFETY: the output is empty and writable; `raw` is a contiguous run
        // of complete OIDs live for this non-retaining graph walk.
        unsafe {
            ffi::git_merge_bases_many(
                out.as_mut_ptr(),
                repository.as_ptr().cast_mut(),
                raw.len(),
                raw.as_ptr(),
            )
        }
    };
    if status == 0 { Ok(output) } else { Err(status) }
}

/// Wraps: git_merge_file_options_init
/// Creates file-merge options initialized for `version`.
pub fn git_merge_file_options_init(version: u32) -> Result<MergeFileOptions, i32> {
    let mut options = MergeFileOptions::zeroed();
    // SAFETY: `options` is writable layout-compatible storage and the
    // initializer stores only scalars and static/null labels.
    let status = unsafe {
        ffi::git_merge_file_options_init(core::ptr::addr_of_mut!(options).cast(), version)
    };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}

#[cfg(test)]
mod scheduled_wrapper_tests {
    use super::*;

    #[test]
    fn file_options_initializer_returns_validated_defaults() {
        let options = git_merge_file_options_init(ffi::GIT_MERGE_FILE_OPTIONS_VERSION).unwrap();
        // SAFETY: `options` is initialized local layout-compatible storage and
        // this shared handle stays within its lifetime.
        let options = unsafe {
            MergeFileOptionsRef::from_ptr(core::ptr::addr_of!(options).cast_mut().cast())
        }
        .unwrap();
        assert_eq!(options.version(), ffi::GIT_MERGE_FILE_OPTIONS_VERSION);
        assert_eq!(options.flags(), Ok(MergeFileFlags::DEFAULT));
        assert_eq!(options.favor(), Ok(MergeFileFavor::Normal));
    }
}

/// Wraps: git_repository_mergehead_foreach
/// Visits each transient object ID parsed from `MERGE_HEAD`.
pub fn git_repository_mergehead_foreach<C>(
    repository: GitRepositoryRef<'_>,
    callback: &mut C,
) -> Result<(), i32>
where
    C: crate::repository::GitRepositoryMergeheadForeachCallback,
{
    unsafe extern "C" fn trampoline<C>(
        oid: *const ffi::git_oid,
        payload: *mut core::ffi::c_void,
    ) -> i32
    where
        C: crate::repository::GitRepositoryMergeheadForeachCallback,
    {
        if oid.is_null() || payload.is_null() {
            return -1;
        }
        // SAFETY: the wrapper supplies this exact live callback for the full
        // synchronous traversal.
        let callback = unsafe { &mut *payload.cast::<C>() };
        // SAFETY: libgit2 supplies a live transient non-null OID for this call.
        let oid = unsafe { OidRef::from_ptr(oid.cast_mut()) }.expect("checked non-null");
        callback.call(oid)
    }

    // SAFETY: the repository and exclusive callback remain live until the
    // traversal returns; neither pointer is retained afterwards.
    let status = unsafe {
        ffi::git_repository_mergehead_foreach(
            repository.as_ptr().cast_mut(),
            Some(trampoline::<C>),
            core::ptr::from_mut(callback).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_merge_trees
/// Merges three optional trees into a newly owned in-memory index.
pub fn git_merge_trees(
    repository: &mut GitRepositoryMut<'_>,
    ancestor: Option<crate::tree::GitTreeRef<'_>>,
    ours: Option<crate::tree::GitTreeRef<'_>>,
    theirs: Option<crate::tree::GitTreeRef<'_>>,
    options: Option<crate::api::merge::GitMergeOptionsRef<'_, '_>>,
) -> Result<crate::index::GitIndexOwned, i32> {
    let mut raw = core::ptr::null_mut();
    let tree_ptr = |tree: Option<crate::tree::GitTreeRef<'_>>| {
        tree.map_or(core::ptr::null(), |tree| tree.as_ptr())
    };
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: `raw` is a writable owner slot; the repository is exclusively
    // borrowed, and every optional tree and options pointer remains live for
    // the call. Libgit2 retains none of the inputs.
    let status = unsafe {
        ffi::git_merge_trees(
            core::ptr::addr_of_mut!(raw),
            repository.as_mut_ptr(),
            tree_ptr(ancestor),
            tree_ptr(ours),
            tree_ptr(theirs),
            options,
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success publishes one complete owned index.
    unsafe { crate::index::GitIndexOwned::from_raw(raw) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_merge
/// Merges one or more annotated heads into the repository and worktree.
pub fn git_merge(
    repo: &mut crate::repository::GitRepositoryMut<'_>,
    their_heads: &[AnnotatedCommitRef<'_>],
    merge_options: Option<crate::api::merge::GitMergeOptionsRef<'_, '_>>,
    checkout_options: Option<crate::api::checkout::GitCheckoutOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    if their_heads.is_empty() {
        return Err(ffi::git_error_code_GIT_EINVALID);
    }
    let mut heads: Vec<*const ffi::git_annotated_commit> =
        their_heads.iter().map(AnnotatedCommitRef::as_ptr).collect();
    let merge_options = merge_options.map_or(core::ptr::null(), |options| options.as_ptr());
    let checkout_options = checkout_options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: the repository is exclusive, `heads` is a contiguous nonempty
    // run of live borrowed pointers, and both options remain live for the call.
    let status = unsafe {
        ffi::git_merge(
            repo.as_mut_ptr(),
            heads.as_mut_ptr(),
            heads.len(),
            merge_options,
            checkout_options,
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_merge_commits
/// Computes the in-memory index produced by merging two commits.
///
/// The repository is borrowed exclusively for the same reason
/// [`git_merge_trees`] is: this call reaches `git_merge__iterators` through
/// `merge_annotated_commits`, whose `merge_normalize_opts` installs
/// `repo->_config` through `git_repository_config__weakptr`, and whose
/// merge-base walk opens the object database the same way. Both are writes to
/// repository storage that a shared handle may not publish.
pub fn git_merge_commits(
    repo: &mut GitRepositoryMut<'_>,
    our_commit: crate::commit::GitCommitRef<'_>,
    their_commit: crate::commit::GitCommitRef<'_>,
    options: Option<crate::api::merge::GitMergeOptionsRef<'_, '_>>,
) -> Result<crate::index::GitIndexOwned, i32> {
    let mut out = core::ptr::null_mut();
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: the output is writable, the repository is exclusively borrowed
    // for the lazy subsystem writes the merge performs, and every other
    // borrowed input remains live for the synchronous merge; the result is an
    // independent index allocation.
    let status = unsafe {
        ffi::git_merge_commits(
            &mut out,
            repo.as_mut_ptr(),
            our_commit.as_ptr(),
            their_commit.as_ptr(),
            options,
        )
    };
    // SAFETY: a non-null output transfers one complete owned index count.
    let out = unsafe { crate::index::GitIndexOwned::from_raw(out) };
    if status == 0 {
        out.ok_or(ffi::git_error_code_GIT_ERROR)
    } else {
        drop(out);
        Err(status)
    }
}

/// Wraps: git_merge_init_options
/// Initializes deprecated merge-options storage for `version`.
pub fn git_merge_init_options(
    options: &mut crate::api::merge::GitMergeOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies writable layout-compatible
    // storage and the initializer retains no pointer to the options header.
    let status = unsafe { ffi::git_merge_init_options(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod scheduled_merge_symbol_tests {
    use super::*;

    /// The shape of an in-memory merge over two commits.
    type CommitMerge = fn(
        &mut GitRepositoryMut<'_>,
        crate::commit::GitCommitRef<'_>,
        crate::commit::GitCommitRef<'_>,
        Option<crate::api::merge::GitMergeOptionsRef<'_, '_>>,
    ) -> Result<crate::index::GitIndexOwned, i32>;

    /// The shape of an in-memory merge over three trees.
    type TreeMerge = fn(
        &mut GitRepositoryMut<'_>,
        Option<crate::tree::GitTreeRef<'_>>,
        Option<crate::tree::GitTreeRef<'_>>,
        Option<crate::tree::GitTreeRef<'_>>,
        Option<crate::api::merge::GitMergeOptionsRef<'_, '_>>,
    ) -> Result<crate::index::GitIndexOwned, i32>;

    #[test]
    fn in_memory_merges_borrow_the_repository_exclusively() {
        // Both entry points reach `git_merge__iterators`, which installs the
        // repository config on first use, so neither accepts a shared handle.
        let _: CommitMerge = git_merge_commits;
        let _: TreeMerge = git_merge_trees;
    }

    #[test]
    fn deprecated_initializer_writes_current_merge_defaults() {
        let mut options = crate::api::merge::GitMergeOptions::new();
        git_merge_init_options(&mut options.as_mut(), ffi::GIT_MERGE_OPTIONS_VERSION).unwrap();
        assert_eq!(options.as_ref().version(), ffi::GIT_MERGE_OPTIONS_VERSION);
    }

    #[test]
    fn public_merge_initializer_returns_owned_options() {
        let options = git_merge_options_init(ffi::GIT_MERGE_OPTIONS_VERSION)
            .expect("the published merge-options version initializes");
        assert_eq!(options.as_ref().version(), ffi::GIT_MERGE_OPTIONS_VERSION);
    }

    #[test]
    fn an_unsupported_version_yields_no_merge_options_at_all() {
        // The rejection path reports through `git_error_set`, which needs the
        // thread state libgit2 installs at initialization.
        // SAFETY: libgit2 initialization is refcounted and balanced below.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        // `git_error__check_version` accepts `0 < version <= current` only, so
        // both ends are refused and the wrapper hands back no options rather
        // than a partially written record.
        assert_eq!(git_merge_options_init(0).err(), Some(-1));
        assert_eq!(
            git_merge_options_init(ffi::GIT_MERGE_OPTIONS_VERSION + 1).err(),
            Some(-1)
        );
        // SAFETY: balances the successful initialization above.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}

/// Wraps: git_merge_options_init
/// Creates merge options initialized for `version`.
pub fn git_merge_options_init<'data>(
    version: core::ffi::c_uint,
) -> Result<ffibox::CVal<crate::api::merge::GitMergeOptions<'data>>, i32> {
    let mut options = crate::api::merge::GitMergeOptions::<'data>::new();
    // SAFETY: the inline options storage is exclusively writable and the C
    // initializer retains no pointer to it or to any of its cleared fields.
    let status = unsafe { ffi::git_merge_options_init(options.as_mut().as_mut_ptr(), version) };
    if status == 0 {
        Ok(options)
    } else {
        Err(status)
    }
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    #[derive(Debug, Eq, PartialEq)]
    struct MergeObservation {
        paths: Vec<Vec<u8>>,
        conflicts: bool,
    }

    unsafe fn commits(
        repository: *mut ffi::git_repository,
    ) -> (*mut ffi::git_object, *mut ffi::git_object) {
        let mut ours = core::ptr::null_mut();
        let mut theirs = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_revparse_single(&mut ours, repository, c"HEAD~2".as_ptr()) },
            0
        );
        assert_eq!(
            unsafe { ffi::git_revparse_single(&mut theirs, repository, c"HEAD".as_ptr()) },
            0
        );
        (ours, theirs)
    }

    unsafe fn raw_observation(repository: *mut ffi::git_repository) -> MergeObservation {
        let (ours, theirs) = unsafe { commits(repository) };
        let mut index = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_merge_commits(
                    &mut index,
                    repository,
                    ours.cast(),
                    theirs.cast(),
                    core::ptr::null(),
                )
            },
            0
        );
        let mut paths = Vec::new();
        for position in 0..unsafe { ffi::git_index_entrycount(index) } {
            let entry = unsafe { ffi::git_index_get_byindex(index, position) };
            paths.push(unsafe { CStr::from_ptr((*entry).path) }.to_bytes().to_vec());
        }
        let conflicts = unsafe { ffi::git_index_has_conflicts(index) } != 0;
        unsafe {
            ffi::git_index_free(index);
            ffi::git_object_free(theirs);
            ffi::git_object_free(ours);
        }
        MergeObservation { paths, conflicts }
    }

    fn safe_observation(repository: *mut ffi::git_repository) -> MergeObservation {
        let (ours, theirs) = unsafe { commits(repository) };
        let ours = unsafe { crate::commit::GitCommitRef::from_ptr(ours.cast()) }.unwrap();
        let theirs = unsafe { crate::commit::GitCommitRef::from_ptr(theirs.cast()) }.unwrap();
        let mut view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut index = git_merge_commits(&mut view, ours, theirs, None).unwrap();
        let mut paths = Vec::new();
        for position in 0..crate::index::git_index_entrycount(index.as_ref()) {
            paths.push(
                crate::index::git_index_get_byindex(&mut index.as_mut(), position)
                    .unwrap()
                    .path()
                    .unwrap()
                    .to_bytes()
                    .to_vec(),
            );
        }
        let conflicts = crate::index::git_index_has_conflicts(index.as_ref());
        drop(index);
        unsafe {
            ffi::git_object_free(theirs.as_ptr().cast_mut().cast());
            ffi::git_object_free(ours.as_ptr().cast_mut().cast());
        }
        MergeObservation { paths, conflicts }
    }

    #[test]
    fn io_equiv_merge_commits_to_index() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("merge-raw");
        let safe = HistoryFixture::new("merge-safe");
        let raw_observation = unsafe { raw_observation(raw.repository.as_ptr()) };
        assert_eq!(raw_observation, safe_observation(safe.repository.as_ptr()));
        assert!(!raw_observation.conflicts);
        assert_eq!(
            raw_observation.paths,
            [
                b"README.md".to_vec(),
                b"src/alpha.c".to_vec(),
                b"src/beta.c".to_vec()
            ]
        );
    }

    fn prepare_divergent_branch(fixture: &HistoryFixture) {
        let path = fixture.directory.path();
        let run = |arguments: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(path)
                .args(arguments)
                .env("GIT_AUTHOR_DATE", "2023-11-14T22:30:00Z")
                .env("GIT_COMMITTER_DATE", "2023-11-14T22:30:00Z")
                .status()
                .unwrap();
            assert!(status.success(), "git command failed: {arguments:?}");
        };
        run(&["checkout", "-q", "-b", "divergent", "HEAD~1"]);
        std::fs::write(path.join("docs/topic.txt"), b"topic branch contribution\n").unwrap();
        run(&["add", "docs/topic.txt"]);
        run(&[
            "-c",
            "user.name=Crustify",
            "-c",
            "user.email=crustify@example.com",
            "commit",
            "-q",
            "-m",
            "divergent topic commit",
        ]);
        run(&["checkout", "-q", "master"]);
    }

    fn prepare_octopus_branches(fixture: &HistoryFixture) {
        let path = fixture.directory.path();
        let run = |arguments: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(path)
                .args(arguments)
                .env("GIT_AUTHOR_DATE", "2023-11-14T22:31:00Z")
                .env("GIT_COMMITTER_DATE", "2023-11-14T22:31:00Z")
                .status()
                .unwrap();
            assert!(status.success(), "git command failed: {arguments:?}");
        };
        for (branch, file, contents) in [
            (
                "octopus-one",
                "docs/one.txt",
                b"first octopus head\n".as_slice(),
            ),
            (
                "octopus-two",
                "docs/two.txt",
                b"second octopus head\n".as_slice(),
            ),
        ] {
            run(&["checkout", "-q", "-b", branch, "HEAD~1"]);
            std::fs::create_dir_all(path.join("docs")).unwrap();
            std::fs::write(path.join(file), contents).unwrap();
            run(&["add", file]);
            run(&[
                "-c",
                "user.name=Crustify",
                "-c",
                "user.email=crustify@example.com",
                "commit",
                "-q",
                "-m",
                branch,
            ]);
            run(&["checkout", "-q", "master"]);
        }
    }

    unsafe fn raw_octopus_merge(fixture: &HistoryFixture) -> (i32, Vec<Vec<u8>>, Vec<Vec<u8>>) {
        let repository = fixture.repository.as_ptr();
        let mut annotated = [core::ptr::null_mut(); 2];
        for (slot, name) in annotated.iter_mut().zip([c"octopus-one", c"octopus-two"]) {
            assert_eq!(
                unsafe { ffi::git_annotated_commit_from_revspec(slot, repository, name.as_ptr()) },
                0
            );
        }
        let mut heads = annotated.map(|head| head.cast_const());
        assert_eq!(
            unsafe {
                ffi::git_merge(
                    repository,
                    heads.as_mut_ptr(),
                    heads.len(),
                    core::ptr::null(),
                    core::ptr::null(),
                )
            },
            0
        );
        unsafe extern "C" fn collect(
            oid: *const ffi::git_oid,
            payload: *mut core::ffi::c_void,
        ) -> i32 {
            unsafe { &mut *payload.cast::<Vec<Vec<u8>>>() }.push(unsafe { (*oid).id }.to_vec());
            0
        }
        let mut mergeheads = Vec::new();
        assert_eq!(
            unsafe {
                ffi::git_repository_mergehead_foreach(
                    repository,
                    Some(collect),
                    core::ptr::from_mut(&mut mergeheads).cast(),
                )
            },
            0
        );
        mergeheads.sort();
        for head in annotated {
            unsafe { ffi::git_annotated_commit_free(head) };
        }
        (
            unsafe { ffi::git_repository_state(repository) },
            ["docs/one.txt", "docs/two.txt"]
                .map(|path| std::fs::read(fixture.directory.path().join(path)).unwrap())
                .to_vec(),
            mergeheads,
        )
    }

    fn safe_octopus_merge(fixture: &HistoryFixture) -> (i32, Vec<Vec<u8>>, Vec<Vec<u8>>) {
        let repository = fixture.repository.as_ptr();
        let mut annotated = [core::ptr::null_mut(); 2];
        for (slot, name) in annotated.iter_mut().zip([c"octopus-one", c"octopus-two"]) {
            assert_eq!(
                unsafe { ffi::git_annotated_commit_from_revspec(slot, repository, name.as_ptr()) },
                0
            );
        }
        let views = annotated.map(|head| unsafe { AnnotatedCommitRef::from_ptr(head) }.unwrap());
        let mut repository_view =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        git_merge(&mut repository_view, &views, None, None).unwrap();
        let mut mergeheads = Vec::new();
        git_repository_mergehead_foreach(repository_view.as_ref(), &mut |oid: OidRef<'_>| {
            mergeheads.push(oid.raw_bytes().elems().collect());
            0
        })
        .unwrap();
        mergeheads.sort();
        let state = crate::repository::git_repository_state(&mut repository_view).unwrap() as i32;
        for head in annotated {
            unsafe { ffi::git_annotated_commit_free(head) };
        }
        (
            state,
            ["docs/one.txt", "docs/two.txt"]
                .map(|path| std::fs::read(fixture.directory.path().join(path)).unwrap())
                .to_vec(),
            mergeheads,
        )
    }

    #[derive(Debug, Eq, PartialEq)]
    struct FullMergeObservation {
        state: i32,
        topic: Vec<u8>,
        paths: Vec<Vec<u8>>,
        conflicts: bool,
    }

    unsafe fn full_merge_observation(fixture: &HistoryFixture) -> FullMergeObservation {
        let repository = fixture.repository.as_ptr();
        let mut index = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_index(&mut index, repository) },
            0
        );
        let paths = (0..unsafe { ffi::git_index_entrycount(index) })
            .map(|position| {
                let entry = unsafe { ffi::git_index_get_byindex(index, position) };
                unsafe { CStr::from_ptr((*entry).path) }.to_bytes().to_vec()
            })
            .collect();
        let conflicts = unsafe { ffi::git_index_has_conflicts(index) } != 0;
        unsafe { ffi::git_index_free(index) };
        FullMergeObservation {
            state: unsafe { ffi::git_repository_state(repository) },
            topic: std::fs::read(fixture.directory.path().join("docs/topic.txt")).unwrap(),
            paths,
            conflicts,
        }
    }

    unsafe fn raw_full_merge(fixture: &HistoryFixture) -> FullMergeObservation {
        let repository = fixture.repository.as_ptr();
        let mut annotated = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_annotated_commit_from_revspec(
                    &mut annotated,
                    repository,
                    c"divergent".as_ptr(),
                )
            },
            0
        );
        let mut heads = [annotated.cast_const()];
        assert_eq!(
            unsafe {
                ffi::git_merge(
                    repository,
                    heads.as_mut_ptr(),
                    1,
                    core::ptr::null(),
                    core::ptr::null(),
                )
            },
            0
        );
        unsafe { ffi::git_annotated_commit_free(annotated) };
        unsafe { full_merge_observation(fixture) }
    }

    fn safe_full_merge(fixture: &HistoryFixture) -> FullMergeObservation {
        let repository = fixture.repository.as_ptr();
        let mut annotated = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_annotated_commit_from_revspec(
                    &mut annotated,
                    repository,
                    c"divergent".as_ptr(),
                )
            },
            0
        );
        {
            let mut repository_view =
                unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
            let annotated_view =
                unsafe { crate::annotated_commit::AnnotatedCommitRef::from_ptr(annotated) }
                    .unwrap();
            git_merge(&mut repository_view, &[annotated_view], None, None).unwrap();
        }
        unsafe { ffi::git_annotated_commit_free(annotated) };
        unsafe { full_merge_observation(fixture) }
    }

    #[test]
    fn io_equiv_full_non_fast_forward_merge_updates_workdir_and_index() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("full-merge-raw");
        let safe = HistoryFixture::new("full-merge-safe");
        prepare_divergent_branch(&raw);
        prepare_divergent_branch(&safe);
        let raw_observation = unsafe { raw_full_merge(&raw) };
        assert_eq!(raw_observation, safe_full_merge(&safe));
        assert_eq!(
            raw_observation.state,
            ffi::git_repository_state_t_GIT_REPOSITORY_STATE_MERGE as i32
        );
        assert!(!raw_observation.conflicts);
    }

    #[test]
    fn io_equiv_octopus_merge_two_heads_and_mergehead_iteration() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("octopus-merge-raw");
        let safe = HistoryFixture::new("octopus-merge-safe");
        prepare_octopus_branches(&raw);
        prepare_octopus_branches(&safe);
        let raw_observation = unsafe { raw_octopus_merge(&raw) };
        let safe_observation = safe_octopus_merge(&safe);
        assert_eq!(raw_observation, safe_observation);
        assert_eq!(raw_observation.2.len(), 2);
        assert_eq!(
            raw_observation.0,
            ffi::git_repository_state_t_GIT_REPOSITORY_STATE_MERGE as i32
        );
    }

    #[derive(Debug, Eq, PartialEq)]
    struct AnalysisObservation {
        analysis: ffi::git_merge_analysis_t,
        preference: ffi::git_merge_preference_t,
        for_ref_analysis: ffi::git_merge_analysis_t,
        base: Vec<u8>,
        base_many: Vec<u8>,
        base_octopus: Vec<u8>,
        bases: usize,
        bases_many: usize,
    }

    unsafe fn raw_analysis(repository: *mut ffi::git_repository) -> AnalysisObservation {
        let mut theirs = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_annotated_commit_from_revspec(
                    &mut theirs,
                    repository,
                    c"divergent".as_ptr(),
                )
            },
            0
        );
        let mut head = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_head(&mut head, repository) },
            0
        );
        let mut heads = [theirs.cast_const()];
        let mut analysis = 0;
        let mut preference = 0;
        assert_eq!(
            unsafe {
                ffi::git_merge_analysis(
                    &mut analysis,
                    &mut preference,
                    repository,
                    heads.as_mut_ptr(),
                    heads.len(),
                )
            },
            0
        );
        let mut for_ref_analysis = 0;
        let mut for_ref_preference = 0;
        assert_eq!(
            unsafe {
                ffi::git_merge_analysis_for_ref(
                    &mut for_ref_analysis,
                    &mut for_ref_preference,
                    repository,
                    head,
                    heads.as_mut_ptr(),
                    heads.len(),
                )
            },
            0
        );
        assert_eq!(preference, for_ref_preference);
        let mut ours = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        let mut theirs_id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut ours, repository, c"HEAD".as_ptr()) },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_reference_name_to_id(
                    &mut theirs_id,
                    repository,
                    c"refs/heads/divergent".as_ptr(),
                )
            },
            0
        );
        let inputs = [ours, theirs_id];
        let mut base = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        let mut base_many = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_merge_base(&mut base, repository, &inputs[0], &inputs[1]) },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_merge_base_many(&mut base_many, repository, inputs.len(), inputs.as_ptr())
            },
            0
        );
        let octopus_inputs = [inputs[0], inputs[1], base];
        let mut base_octopus = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_merge_base_octopus(
                    &mut base_octopus,
                    repository,
                    octopus_inputs.len(),
                    octopus_inputs.as_ptr(),
                )
            },
            0
        );
        let mut bases = unsafe { core::mem::zeroed::<ffi::git_oidarray>() };
        let mut bases_many = unsafe { core::mem::zeroed::<ffi::git_oidarray>() };
        assert_eq!(
            unsafe { ffi::git_merge_bases(&mut bases, repository, &inputs[0], &inputs[1]) },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_merge_bases_many(
                    &mut bases_many,
                    repository,
                    inputs.len(),
                    inputs.as_ptr(),
                )
            },
            0
        );
        let observation = AnalysisObservation {
            analysis,
            preference,
            for_ref_analysis,
            base: base.id.to_vec(),
            base_many: base_many.id.to_vec(),
            base_octopus: base_octopus.id.to_vec(),
            bases: bases.count,
            bases_many: bases_many.count,
        };
        unsafe {
            ffi::git_oidarray_dispose(&mut bases_many);
            ffi::git_oidarray_dispose(&mut bases);
            ffi::git_reference_free(head);
            ffi::git_annotated_commit_free(theirs);
        }
        observation
    }

    fn safe_analysis(repository: *mut ffi::git_repository) -> AnalysisObservation {
        let mut theirs = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_annotated_commit_from_revspec(
                    &mut theirs,
                    repository,
                    c"divergent".as_ptr(),
                )
            },
            0
        );
        let mut head = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_head(&mut head, repository) },
            0
        );
        let repository_ref =
            unsafe { crate::repository::GitRepositoryRef::from_ptr(repository) }.unwrap();
        let theirs_ref =
            unsafe { crate::annotated_commit::AnnotatedCommitRef::from_ptr(theirs) }.unwrap();
        let head_ref = unsafe { crate::refs::GitReferenceRef::from_ptr(head) }.unwrap();
        let (analysis, preference) = git_merge_analysis(repository_ref, theirs_ref).unwrap();
        let (for_ref_analysis, for_ref_preference) =
            git_merge_analysis_for_ref(repository_ref, head_ref, theirs_ref).unwrap();
        assert_eq!(preference, for_ref_preference);
        let mut ours = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        let mut theirs_id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut ours, repository, c"HEAD".as_ptr()) },
            0
        );
        assert_eq!(
            unsafe {
                ffi::git_reference_name_to_id(
                    &mut theirs_id,
                    repository,
                    c"refs/heads/divergent".as_ptr(),
                )
            },
            0
        );
        let ours_ref = unsafe { OidRef::from_ptr(core::ptr::from_mut(&mut ours)) }.unwrap();
        let theirs_id_ref =
            unsafe { OidRef::from_ptr(core::ptr::from_mut(&mut theirs_id)) }.unwrap();
        let mut base = git_merge_base(repository_ref, ours_ref, theirs_id_ref).unwrap();
        let mut base_many =
            git_merge_base_many(repository_ref, &[ours_ref, theirs_id_ref]).unwrap();
        let mut raw_inputs = [ours, theirs_id, unsafe {
            core::ptr::from_ref(&base).cast::<ffi::git_oid>().read()
        }];
        let inputs = unsafe {
            ffibox::CSlice::from_raw_parts(
                core::ptr::NonNull::new(raw_inputs.as_mut_ptr().cast::<Oid>()).unwrap(),
                raw_inputs.len(),
            )
        };
        let mut base_octopus = git_merge_base_octopus(repository_ref, inputs).unwrap();
        let bases = git_merge_bases(repository_ref, ours_ref, theirs_id_ref)
            .unwrap()
            .as_ref()
            .count();
        let bases_many = git_merge_bases_many(repository_ref, &[ours_ref, theirs_id_ref])
            .unwrap()
            .as_ref()
            .count();
        let oid_bytes = |value: &mut Oid| {
            unsafe { OidRef::from_ptr(core::ptr::from_mut(value).cast::<ffi::git_oid>()) }
                .unwrap()
                .raw_bytes()
                .elems()
                .collect()
        };
        let observation = AnalysisObservation {
            analysis: analysis.bits(),
            preference: preference.into(),
            for_ref_analysis: for_ref_analysis.bits(),
            base: oid_bytes(&mut base),
            base_many: oid_bytes(&mut base_many),
            base_octopus: oid_bytes(&mut base_octopus),
            bases,
            bases_many,
        };
        unsafe {
            ffi::git_reference_free(head);
            ffi::git_annotated_commit_free(theirs);
        }
        observation
    }

    #[test]
    fn io_equiv_merge_analysis_and_common_ancestor_queries() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("merge-analysis-raw");
        let safe = HistoryFixture::new("merge-analysis-safe");
        prepare_divergent_branch(&raw);
        prepare_divergent_branch(&safe);
        let raw_observation = unsafe { raw_analysis(raw.repository.as_ptr()) };
        assert_eq!(raw_observation, safe_analysis(safe.repository.as_ptr()));
        assert_ne!(raw_observation.analysis, 0);
        assert_eq!(raw_observation.bases, 1);
    }

    fn prepare_content_conflict(fixture: &HistoryFixture) {
        let path = fixture.directory.path();
        let run = |arguments: &[&str]| {
            let status = std::process::Command::new("git")
                .current_dir(path)
                .args(arguments)
                .env("GIT_AUTHOR_DATE", "1700110000 +0000")
                .env("GIT_COMMITTER_DATE", "1700110000 +0000")
                .stdout(std::process::Stdio::null())
                .status()
                .unwrap();
            assert!(status.success(), "git command failed: {arguments:?}");
        };
        run(&["checkout", "-q", "-b", "content-conflict"]);
        std::fs::write(path.join("README.md"), b"the side branch version\n").unwrap();
        run(&["add", "README.md"]);
        run(&[
            "-c",
            "user.name=Crustify",
            "-c",
            "user.email=crustify@example.com",
            "commit",
            "-q",
            "-m",
            "side content",
        ]);
        run(&["checkout", "-q", "master"]);
        std::fs::write(path.join("README.md"), b"the master branch version\n").unwrap();
        run(&["add", "README.md"]);
        run(&[
            "-c",
            "user.name=Crustify",
            "-c",
            "user.email=crustify@example.com",
            "commit",
            "-q",
            "-m",
            "master content",
        ]);
    }

    unsafe fn conflict_trees(repository: *mut ffi::git_repository) -> [*mut ffi::git_tree; 3] {
        let specs = [
            c"master~1^{tree}",
            c"master^{tree}",
            c"content-conflict^{tree}",
        ];
        let mut trees = [core::ptr::null_mut(); 3];
        for (slot, spec) in trees.iter_mut().zip(specs) {
            let mut object = core::ptr::null_mut();
            assert_eq!(
                unsafe { ffi::git_revparse_single(&mut object, repository, spec.as_ptr()) },
                0
            );
            *slot = object.cast();
        }
        trees
    }

    #[derive(Debug, Eq, PartialEq)]
    struct TreeMergeObservation(Vec<(bool, Vec<(u16, Vec<u8>, Vec<u8>)>)>);

    unsafe fn raw_tree_merges(repository: *mut ffi::git_repository) -> TreeMergeObservation {
        let trees = unsafe { conflict_trees(repository) };
        let mut variants = Vec::new();
        for favor in [
            ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_NORMAL,
            ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_OURS,
            ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_THEIRS,
            ffi::git_merge_file_favor_t_GIT_MERGE_FILE_FAVOR_UNION,
        ] {
            let mut options = unsafe { core::mem::zeroed::<ffi::git_merge_options>() };
            assert_eq!(
                unsafe {
                    ffi::git_merge_options_init(&mut options, ffi::GIT_MERGE_OPTIONS_VERSION)
                },
                0
            );
            options.file_favor = favor;
            options.file_flags = ffi::git_merge_file_flag_t_GIT_MERGE_FILE_STYLE_DIFF3
                | ffi::git_merge_file_flag_t_GIT_MERGE_FILE_DIFF_PATIENCE;
            let mut index = core::ptr::null_mut();
            assert_eq!(
                unsafe {
                    ffi::git_merge_trees(
                        &mut index, repository, trees[0], trees[1], trees[2], &options,
                    )
                },
                0
            );
            let conflicts = unsafe { ffi::git_index_has_conflicts(index) != 0 };
            let mut entries = (0..unsafe { ffi::git_index_entrycount(index) })
                .map(|position| {
                    let entry = unsafe { ffi::git_index_get_byindex(index, position) };
                    (
                        unsafe { (*entry).flags },
                        unsafe { CStr::from_ptr((*entry).path) }.to_bytes().to_vec(),
                        unsafe { (*entry).id.id }.to_vec(),
                    )
                })
                .collect::<Vec<_>>();
            entries.sort();
            variants.push((conflicts, entries));
            unsafe { ffi::git_index_free(index) };
        }
        for tree in trees {
            unsafe { ffi::git_object_free(tree.cast()) };
        }
        TreeMergeObservation(variants)
    }

    fn safe_tree_merges(repository: *mut ffi::git_repository) -> TreeMergeObservation {
        let trees = unsafe { conflict_trees(repository) };
        let tree_refs =
            trees.map(|tree| unsafe { crate::tree::GitTreeRef::from_ptr(tree) }.unwrap());
        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut variants = Vec::new();
        for favor in [
            MergeFileFavor::Normal,
            MergeFileFavor::Ours,
            MergeFileFavor::Theirs,
            MergeFileFavor::Union,
        ] {
            let mut options = crate::api::merge::GitMergeOptions::new();
            options.as_mut().set_file_favor(favor);
            options
                .as_mut()
                .set_file_flags(MergeFileFlags::STYLE_DIFF3 | MergeFileFlags::DIFF_PATIENCE);
            let mut index = git_merge_trees(
                &mut repository,
                Some(tree_refs[0]),
                Some(tree_refs[1]),
                Some(tree_refs[2]),
                Some(options.as_ref()),
            )
            .unwrap();
            let conflicts = crate::index::git_index_has_conflicts(index.as_ref());
            let mut entries = Vec::new();
            for position in 0..crate::index::git_index_entrycount(index.as_ref()) {
                let mut index_view = index.as_mut();
                let entry = crate::index::git_index_get_byindex(&mut index_view, position).unwrap();
                entries.push((
                    entry.flags(),
                    entry.path().unwrap().to_bytes().to_vec(),
                    entry.id().raw_bytes().elems().collect(),
                ));
            }
            entries.sort();
            variants.push((conflicts, entries));
        }
        drop(repository);
        for tree in trees {
            unsafe { ffi::git_object_free(tree.cast()) };
        }
        TreeMergeObservation(variants)
    }

    #[test]
    fn io_equiv_merge_trees_conflicts_and_content_favor_modes() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("merge-trees-raw");
        let safe = HistoryFixture::new("merge-trees-safe");
        prepare_content_conflict(&raw);
        prepare_content_conflict(&safe);
        let raw_observation = unsafe { raw_tree_merges(raw.repository.as_ptr()) };
        assert_eq!(raw_observation, safe_tree_merges(safe.repository.as_ptr()));
        assert!(raw_observation.0[0].0);
        assert!(raw_observation.0[1..].iter().all(|variant| !variant.0));
    }

    fn prepare_rename_conflict(fixture: &HistoryFixture) {
        let path = fixture.directory.path();
        let run = |arguments: &[&str], timestamp: &str| {
            let status = std::process::Command::new("git")
                .current_dir(path)
                .args(arguments)
                .env("GIT_AUTHOR_DATE", timestamp)
                .env("GIT_COMMITTER_DATE", timestamp)
                .stdout(std::process::Stdio::null())
                .status()
                .unwrap();
            assert!(status.success(), "git command failed: {arguments:?}");
        };
        run(&["checkout", "-q", "-b", "rename-side"], "1700120000 +0000");
        run(
            &["mv", "src/alpha.c", "src/side-alpha.c"],
            "1700120000 +0000",
        );
        run(&["add", "-A"], "1700120000 +0000");
        run(
            &[
                "-c",
                "user.name=Crustify",
                "-c",
                "user.email=crustify@example.com",
                "commit",
                "-q",
                "-m",
                "side rename",
            ],
            "1700120000 +0000",
        );
        run(&["checkout", "-q", "master"], "1700120100 +0000");
        run(
            &["mv", "src/alpha.c", "src/master-alpha.c"],
            "1700120100 +0000",
        );
        run(&["add", "-A"], "1700120100 +0000");
        run(
            &[
                "-c",
                "user.name=Crustify",
                "-c",
                "user.email=crustify@example.com",
                "commit",
                "-q",
                "-m",
                "master rename",
            ],
            "1700120100 +0000",
        );
    }

    unsafe fn rename_trees(repository: *mut ffi::git_repository) -> [*mut ffi::git_tree; 3] {
        let specs = [c"master~1^{tree}", c"master^{tree}", c"rename-side^{tree}"];
        let mut trees = [core::ptr::null_mut(); 3];
        for (tree, spec) in trees.iter_mut().zip(specs) {
            let mut object = core::ptr::null_mut();
            assert_eq!(
                unsafe { ffi::git_revparse_single(&mut object, repository, spec.as_ptr()) },
                0
            );
            *tree = object.cast();
        }
        trees
    }

    unsafe fn raw_rename_merge(repository: *mut ffi::git_repository) -> Vec<(u16, Vec<u8>)> {
        let trees = unsafe { rename_trees(repository) };
        let mut options = unsafe { core::mem::zeroed::<ffi::git_merge_options>() };
        assert_eq!(
            unsafe { ffi::git_merge_options_init(&mut options, ffi::GIT_MERGE_OPTIONS_VERSION) },
            0
        );
        options.flags = ffi::git_merge_flag_t_GIT_MERGE_FIND_RENAMES;
        options.rename_threshold = 40;
        let mut index = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_merge_trees(
                    &mut index, repository, trees[0], trees[1], trees[2], &options,
                )
            },
            0
        );
        assert_ne!(unsafe { ffi::git_index_has_conflicts(index) }, 0);
        let entries = (0..unsafe { ffi::git_index_entrycount(index) })
            .map(|position| {
                let entry = unsafe { ffi::git_index_get_byindex(index, position) };
                (
                    unsafe { (*entry).flags },
                    unsafe { CStr::from_ptr((*entry).path) }.to_bytes().to_vec(),
                )
            })
            .collect();
        unsafe { ffi::git_index_free(index) };
        for tree in trees {
            unsafe { ffi::git_object_free(tree.cast()) };
        }
        entries
    }

    fn safe_rename_merge(repository: *mut ffi::git_repository) -> Vec<(u16, Vec<u8>)> {
        let trees = unsafe { rename_trees(repository) };
        let trees = trees.map(|tree| unsafe { crate::tree::GitTreeRef::from_ptr(tree) }.unwrap());
        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let mut options = crate::api::merge::GitMergeOptions::new();
        options
            .as_mut()
            .set_flags(crate::api::merge::GitMergeFlags::FIND_RENAMES);
        options.as_mut().set_rename_threshold(40);
        let mut index = git_merge_trees(
            &mut repository,
            Some(trees[0]),
            Some(trees[1]),
            Some(trees[2]),
            Some(options.as_ref()),
        )
        .unwrap();
        assert!(crate::index::git_index_has_conflicts(index.as_ref()));
        let entries = (0..crate::index::git_index_entrycount(index.as_ref()))
            .map(|position| {
                let mut view = index.as_mut();
                let entry = crate::index::git_index_get_byindex(&mut view, position).unwrap();
                (entry.flags(), entry.path().unwrap().to_bytes().to_vec())
            })
            .collect();
        drop(index);
        drop(repository);
        for tree in trees {
            unsafe { ffi::git_object_free(tree.as_ptr().cast_mut().cast()) };
        }
        entries
    }

    #[test]
    fn io_equiv_rename_rename_conflict_detection() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("merge-rename-raw");
        let safe = HistoryFixture::new("merge-rename-safe");
        prepare_rename_conflict(&raw);
        prepare_rename_conflict(&safe);
        let raw = unsafe { raw_rename_merge(raw.repository.as_ptr()) };
        assert_eq!(raw, safe_rename_merge(safe.repository.as_ptr()));
        assert!(raw.iter().any(|(_, path)| path == b"src/master-alpha.c"));
        assert!(raw.iter().any(|(_, path)| path == b"src/side-alpha.c"));
    }

    fn prepare_rename_delete_conflict(fixture: &HistoryFixture) {
        let path = fixture.directory.path();
        let run = |arguments: &[&str], timestamp: &str| {
            let status = std::process::Command::new("git")
                .current_dir(path)
                .args(arguments)
                .env("GIT_AUTHOR_DATE", timestamp)
                .env("GIT_COMMITTER_DATE", timestamp)
                .stdout(std::process::Stdio::null())
                .status()
                .unwrap();
            assert!(status.success(), "git command failed: {arguments:?}");
        };
        run(&["checkout", "-q", "-b", "rename-side"], "1700130000 +0000");
        run(&["rm", "-q", "src/alpha.c"], "1700130000 +0000");
        run(
            &[
                "-c",
                "user.name=Crustify",
                "-c",
                "user.email=crustify@example.com",
                "commit",
                "-q",
                "-m",
                "side delete",
            ],
            "1700130000 +0000",
        );
        run(&["checkout", "-q", "master"], "1700130100 +0000");
        run(
            &["mv", "src/alpha.c", "src/master-alpha.c"],
            "1700130100 +0000",
        );
        run(
            &[
                "-c",
                "user.name=Crustify",
                "-c",
                "user.email=crustify@example.com",
                "commit",
                "-q",
                "-m",
                "master rename",
            ],
            "1700130100 +0000",
        );
    }

    #[test]
    fn io_equiv_rename_delete_conflict_detection() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("merge-rename-delete-raw");
        let safe = HistoryFixture::new("merge-rename-delete-safe");
        prepare_rename_delete_conflict(&raw);
        prepare_rename_delete_conflict(&safe);
        let raw = unsafe { raw_rename_merge(raw.repository.as_ptr()) };
        assert_eq!(raw, safe_rename_merge(safe.repository.as_ptr()));
        assert!(raw.iter().any(|(_, path)| path == b"src/master-alpha.c"));
    }

    fn prepare_directory_file_conflict(fixture: &HistoryFixture) {
        let path = fixture.directory.path();
        let run = |arguments: &[&str], timestamp: &str| {
            let status = std::process::Command::new("git")
                .current_dir(path)
                .args(arguments)
                .env("GIT_AUTHOR_DATE", timestamp)
                .env("GIT_COMMITTER_DATE", timestamp)
                .stdout(std::process::Stdio::null())
                .status()
                .unwrap();
            assert!(status.success(), "git command failed: {arguments:?}");
        };
        run(&["checkout", "-q", "-b", "rename-side"], "1700140000 +0000");
        run(&["rm", "-q", "-r", "src"], "1700140000 +0000");
        std::fs::write(path.join("src"), b"side replaced the directory\n").unwrap();
        run(&["add", "src"], "1700140000 +0000");
        run(
            &[
                "-c",
                "user.name=Crustify",
                "-c",
                "user.email=crustify@example.com",
                "commit",
                "-q",
                "-m",
                "side directory to file",
            ],
            "1700140000 +0000",
        );
        run(&["checkout", "-q", "master"], "1700140100 +0000");
        std::fs::write(
            path.join("src/beta.c"),
            b"int beta(int x) { return x * x * x; }\n",
        )
        .unwrap();
        run(&["add", "src/beta.c"], "1700140100 +0000");
        run(
            &[
                "-c",
                "user.name=Crustify",
                "-c",
                "user.email=crustify@example.com",
                "commit",
                "-q",
                "-m",
                "master directory edit",
            ],
            "1700140100 +0000",
        );
    }

    #[test]
    fn io_equiv_directory_file_conflict_detection() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("merge-directory-file-raw");
        let safe = HistoryFixture::new("merge-directory-file-safe");
        prepare_directory_file_conflict(&raw);
        prepare_directory_file_conflict(&safe);
        let raw = unsafe { raw_rename_merge(raw.repository.as_ptr()) };
        assert_eq!(raw, safe_rename_merge(safe.repository.as_ptr()));
        assert!(raw.iter().any(|(_, path)| path.starts_with(b"src")));
    }

    unsafe fn raw_conflicted_full_merge(fixture: &HistoryFixture) -> (i32, bool, Vec<Vec<u8>>) {
        let repository = fixture.repository.as_ptr();
        let mut annotated = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_annotated_commit_from_revspec(
                    &mut annotated,
                    repository,
                    c"rename-side".as_ptr(),
                )
            },
            0
        );
        let mut heads = [annotated.cast_const()];
        assert_eq!(
            unsafe {
                ffi::git_merge(
                    repository,
                    heads.as_mut_ptr(),
                    heads.len(),
                    core::ptr::null(),
                    core::ptr::null(),
                )
            },
            0
        );
        unsafe { ffi::git_annotated_commit_free(annotated) };
        let mut index = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_repository_index(&mut index, repository) },
            0
        );
        let conflicts = unsafe { ffi::git_index_has_conflicts(index) } != 0;
        let paths = (0..unsafe { ffi::git_index_entrycount(index) })
            .map(|position| {
                let entry = unsafe { ffi::git_index_get_byindex(index, position) };
                unsafe { CStr::from_ptr((*entry).path) }.to_bytes().to_vec()
            })
            .collect();
        unsafe { ffi::git_index_free(index) };
        (
            unsafe { ffi::git_repository_state(repository) },
            conflicts,
            paths,
        )
    }

    fn safe_conflicted_full_merge(fixture: &HistoryFixture) -> (i32, bool, Vec<Vec<u8>>) {
        let repository = fixture.repository.as_ptr();
        let mut annotated = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_annotated_commit_from_revspec(
                    &mut annotated,
                    repository,
                    c"rename-side".as_ptr(),
                )
            },
            0
        );
        let mut repository =
            unsafe { crate::repository::GitRepositoryMut::from_ptr(repository) }.unwrap();
        let annotated = unsafe { AnnotatedCommitRef::from_ptr(annotated) }.unwrap();
        git_merge(&mut repository, &[annotated], None, None).unwrap();
        unsafe { ffi::git_annotated_commit_free(annotated.as_ptr().cast_mut()) };
        let mut index = crate::repository::git_repository_index(&mut repository).unwrap();
        let conflicts = crate::index::git_index_has_conflicts(index.as_ref());
        let paths = (0..crate::index::git_index_entrycount(index.as_ref()))
            .map(|position| {
                let mut view = index.as_mut();
                crate::index::git_index_get_byindex(&mut view, position)
                    .unwrap()
                    .path()
                    .unwrap()
                    .to_bytes()
                    .to_vec()
            })
            .collect();
        let state = crate::repository::git_repository_state(&mut repository).unwrap() as i32;
        (state, conflicts, paths)
    }

    #[test]
    fn io_equiv_full_rename_rename_merge_updates_checkout_and_index() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("full-directory-file-raw");
        let safe = HistoryFixture::new("full-directory-file-safe");
        prepare_rename_conflict(&raw);
        prepare_rename_conflict(&safe);
        let raw_observation = unsafe { raw_conflicted_full_merge(&raw) };
        assert_eq!(raw_observation, safe_conflicted_full_merge(&safe));
        assert!(raw_observation.1);
        assert_eq!(
            raw_observation.0,
            ffi::git_repository_state_t_GIT_REPOSITORY_STATE_MERGE as i32
        );
    }

    #[test]
    fn io_equiv_full_rename_delete_merge_updates_checkout_and_index() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("full-rename-delete-raw");
        let safe = HistoryFixture::new("full-rename-delete-safe");
        prepare_rename_delete_conflict(&raw);
        prepare_rename_delete_conflict(&safe);
        let raw_observation = unsafe { raw_conflicted_full_merge(&raw) };
        assert_eq!(raw_observation, safe_conflicted_full_merge(&safe));
        assert!(raw_observation.1);
        assert_eq!(
            raw_observation.0,
            ffi::git_repository_state_t_GIT_REPOSITORY_STATE_MERGE as i32
        );
    }
}
