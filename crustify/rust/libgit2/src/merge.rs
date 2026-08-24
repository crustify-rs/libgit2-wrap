//! Safe wrappers for libgit2 merge APIs.

use core::ffi::CStr;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CLenDropped, CSlice, CVal, CVec, CrustifyStr};

use crate::ffi;
use crate::oid::Oid;
use crate::repository::GitRepositoryRef;
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
    /// Wraps: git_merge_file_input.size
    /// Returns the number of bytes in the input contents.
    #[must_use]
    pub fn size(&self) -> usize {
        // SAFETY: this live shared handle covers the complete C value, and
        // raw-place projection reads the initialized scalar without forming a
        // reference to C-visible memory.
        unsafe { addr_of!((*self.as_ptr()).size).read() }
    }

    /// Wraps: git_merge_file_input.path
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

    /// Wraps: git_merge_file_input.mode
    /// Returns the file mode, or zero when no mode should be merged.
    #[must_use]
    pub fn mode(&self) -> u32 {
        // SAFETY: as `size`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).mode).read() }
    }

    /// Wraps: git_merge_file_input.version
    /// Returns the ABI version of this input descriptor.
    #[must_use]
    pub fn version(&self) -> u32 {
        // SAFETY: as `size`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Wraps: git_merge_file_input.ptr
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
    /// Wraps: git_merge_file_result.automergeable
    /// Reports whether the output was merged without conflict markers.
    #[must_use]
    pub fn is_automergeable(&self) -> bool {
        // SAFETY: this live shared handle permits a raw-place read of the
        // initialized scalar without forming a reference to C-owned memory.
        unsafe { addr_of!((*self.as_ptr()).automergeable).read() != 0 }
    }

    /// Wraps: git_merge_file_result.path
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

    /// Wraps: git_merge_file_result.mode
    /// Returns the file mode selected for the merged result.
    #[must_use]
    pub fn mode(&self) -> u32 {
        // SAFETY: as `is_automergeable`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).mode).read() }
    }

    /// Wraps: git_merge_file_result.ptr
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

    /// Wraps: git_merge_file_result.len
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
mod tests {
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
}

/// Known `git_merge_file_flag_t` bits accepted by file-level merges.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MergeFileFlags(ffi::git_merge_file_flag_t);

impl MergeFileFlags {
    /// Use libgit2's default file-merge behavior.
    pub const DEFAULT: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_DEFAULT);
    /// Produce standard two-sided conflict markers.
    pub const STYLE_MERGE: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_STYLE_MERGE);
    /// Include the common ancestor in conflict markers.
    pub const STYLE_DIFF3: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_STYLE_DIFF3);
    /// Condense non-alphanumeric regions while comparing.
    pub const SIMPLIFY_ALNUM: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_SIMPLIFY_ALNUM);
    /// Ignore all whitespace changes.
    pub const IGNORE_WHITESPACE: Self =
        Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_IGNORE_WHITESPACE);
    /// Ignore changes in the amount of whitespace.
    pub const IGNORE_WHITESPACE_CHANGE: Self =
        Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_IGNORE_WHITESPACE_CHANGE);
    /// Ignore whitespace changes at the ends of lines.
    pub const IGNORE_WHITESPACE_EOL: Self =
        Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_IGNORE_WHITESPACE_EOL);
    /// Use the patience-diff algorithm.
    pub const DIFF_PATIENCE: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_DIFF_PATIENCE);
    /// Spend extra time finding a minimal diff.
    pub const DIFF_MINIMAL: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_DIFF_MINIMAL);
    /// Produce zealous diff3 conflict markers.
    pub const STYLE_ZDIFF3: Self = Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_STYLE_ZDIFF3);
    /// Accept output containing conflict markers as a merge result.
    pub const ACCEPT_CONFLICTS: Self =
        Self(ffi::git_merge_file_flag_t_GIT_MERGE_FILE_ACCEPT_CONFLICTS);
    /// Every file-merge flag published by this libgit2 version.
    pub const ALL: Self = Self(
        Self::STYLE_MERGE.0
            | Self::STYLE_DIFF3.0
            | Self::SIMPLIFY_ALNUM.0
            | Self::IGNORE_WHITESPACE.0
            | Self::IGNORE_WHITESPACE_CHANGE.0
            | Self::IGNORE_WHITESPACE_EOL.0
            | Self::DIFF_PATIENCE.0
            | Self::DIFF_MINIMAL.0
            | Self::STYLE_ZDIFF3.0
            | Self::ACCEPT_CONFLICTS.0,
    );

    /// Converts raw bits when every bit is known to this libgit2 version.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_merge_file_flag_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 flag bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_merge_file_flag_t {
        self.0
    }

    /// Returns whether no behavior-changing flag is set.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every flag in `other` is present.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for MergeFileFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for MergeFileFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for MergeFileFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for MergeFileFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for MergeFileFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(self.0 ^ Self::ALL.0)
    }
}

ffibox::define_ctype!(
    /// Wraps: git_merge_file_options
    /// Borrowing options that control a file-level merge.
    MergeFileOptions,
    MergeFileOptionsRef,
    MergeFileOptionsMut,
    ffi::git_merge_file_options
);

impl<'a> MergeFileOptionsRef<'a> {
    /// Wraps: git_merge_file_options.flags
    /// Returns the validated file-merge behavior flags.
    pub fn flags(&self) -> Result<MergeFileFlags, ffi::git_merge_file_flag_t> {
        // SAFETY: this live shared handle permits a raw-place scalar read
        // without forming a reference to C-visible memory.
        let flags = unsafe { addr_of!((*self.as_ptr()).flags).read() };
        MergeFileFlags::from_bits(flags).ok_or(flags)
    }

    /// Wraps: git_merge_file_options.version
    /// Returns the ABI version of this options value.
    #[must_use]
    pub fn version(&self) -> u32 {
        // SAFETY: as `flags`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Wraps: git_merge_file_options.favor
    /// Returns the validated conflict-side preference.
    pub fn favor(&self) -> Result<MergeFileFavor, ffi::git_merge_file_favor_t> {
        // SAFETY: as `flags`, for this initialized scalar field.
        let favor = unsafe { addr_of!((*self.as_ptr()).favor).read() };
        MergeFileFavor::try_from(favor)
    }

    /// Wraps: git_merge_file_options.marker_size
    /// Returns the requested conflict-marker width, or zero for the default.
    #[must_use]
    pub fn marker_size(&self) -> u16 {
        // SAFETY: as `flags`, for this initialized scalar field.
        unsafe { addr_of!((*self.as_ptr()).marker_size).read() }
    }

    /// Wraps: git_merge_file_options.their_label
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

    /// Wraps: git_merge_file_options.our_label
    /// Returns the optional label borrowed for the "ours" side.
    #[must_use]
    pub fn our_label(&self) -> Option<&'a CStr> {
        // SAFETY: as `their_label`, for this initialized pointer field.
        let label = unsafe { addr_of!((*self.as_ptr()).our_label).read() };
        // SAFETY: as `their_label`, for this borrowed string field.
        unsafe { optional_borrowed_label(label) }
    }

    /// Wraps: git_merge_file_options.ancestor_label
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
