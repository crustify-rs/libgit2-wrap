//! Safe wrappers for libgit2 merge APIs.

use core::ffi::CStr;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::CSlice;

use crate::ffi;

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
}
