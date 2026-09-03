//! Safe wrappers for libgit2 refs APIs.

use core::ffi::CStr;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::ffi;
use crate::refs::GitReferenceTetheredOwned;

/// Wraps: git_reference_format_t
/// A checked set of options for validating and normalizing reference names.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitReferenceFormatFlags(ffi::git_reference_format_t);

impl GitReferenceFormatFlags {
    /// Apply the normal multi-level reference-name rules.
    pub const NORMAL: Self = Self(ffi::git_reference_format_t_GIT_REFERENCE_FORMAT_NORMAL);
    /// Permit one-level reference names such as `HEAD`.
    pub const ALLOW_ONELEVEL: Self =
        Self(ffi::git_reference_format_t_GIT_REFERENCE_FORMAT_ALLOW_ONELEVEL);
    /// Permit one full-component wildcard in a refspec pattern.
    pub const REFSPEC_PATTERN: Self =
        Self(ffi::git_reference_format_t_GIT_REFERENCE_FORMAT_REFSPEC_PATTERN);
    /// Interpret the name as a shorthand refspec component.
    pub const REFSPEC_SHORTHAND: Self =
        Self(ffi::git_reference_format_t_GIT_REFERENCE_FORMAT_REFSPEC_SHORTHAND);
    /// Every reference-format option published by this libgit2 version.
    pub const ALL: Self =
        Self(Self::ALLOW_ONELEVEL.0 | Self::REFSPEC_PATTERN.0 | Self::REFSPEC_SHORTHAND.0);

    /// Converts raw bits when every bit is a published format option.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_reference_format_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 option bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_reference_format_t {
        self.0
    }

    /// Returns whether no optional behavior is enabled.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every option in `other` is enabled.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any option in `other` is enabled.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitReferenceFormatFlags> for ffi::git_reference_format_t {
    fn from(flags: GitReferenceFormatFlags) -> Self {
        flags.bits()
    }
}

impl TryFrom<ffi::git_reference_format_t> for GitReferenceFormatFlags {
    type Error = ffi::git_reference_format_t;

    fn try_from(bits: ffi::git_reference_format_t) -> Result<Self, Self::Error> {
        Self::from_bits(bits).ok_or(bits)
    }
}

impl BitOr for GitReferenceFormatFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitReferenceFormatFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitReferenceFormatFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitReferenceFormatFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitReferenceFormatFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & Self::ALL.0)
    }
}

/// Wraps: git_reference_foreach_name_cb
/// Safe callable surface for one transient reference name.
pub trait GitReferenceForeachNameCallback {
    /// Receives a reference name borrowed for this invocation.
    ///
    /// Returning nonzero stops iteration and propagates that value to the
    /// caller.
    fn call(&mut self, name: &CStr) -> i32;
}

impl<F> GitReferenceForeachNameCallback for F
where
    F: FnMut(&CStr) -> i32,
{
    fn call(&mut self, name: &CStr) -> i32 {
        self(name)
    }
}

/// Wraps: git_reference_foreach_cb
/// Safe callable surface for one owned reference yielded by an iteration.
pub trait GitReferenceForeachCallback {
    /// Receives ownership of one reference tied to the repository borrow for
    /// this invocation. Returning nonzero stops iteration.
    fn call<'repo>(&mut self, reference: GitReferenceTetheredOwned<'repo>) -> i32;
}

impl<F> GitReferenceForeachCallback for F
where
    F: for<'repo> FnMut(GitReferenceTetheredOwned<'repo>) -> i32,
{
    fn call<'repo>(&mut self, reference: GitReferenceTetheredOwned<'repo>) -> i32 {
        self(reference)
    }
}

#[cfg(test)]
mod unit_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn reference_format_flags_combine_and_validate() {
        let flags =
            GitReferenceFormatFlags::ALLOW_ONELEVEL | GitReferenceFormatFlags::REFSPEC_PATTERN;
        assert!(flags.contains(GitReferenceFormatFlags::ALLOW_ONELEVEL));
        assert!(flags.intersects(GitReferenceFormatFlags::REFSPEC_PATTERN));
        assert!(!flags.intersects(GitReferenceFormatFlags::REFSPEC_SHORTHAND));
        assert_eq!(
            GitReferenceFormatFlags::from_bits(flags.bits()),
            Some(flags)
        );
        assert!(GitReferenceFormatFlags::NORMAL.is_empty());

        let unknown = GitReferenceFormatFlags::ALL.bits() << 1;
        assert_eq!(GitReferenceFormatFlags::from_bits(unknown), None);
        assert_eq!(GitReferenceFormatFlags::try_from(unknown), Err(unknown));
    }

    #[test]
    fn reference_format_flags_preserve_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitReferenceFormatFlags>(),
            size_of::<ffi::git_reference_format_t>()
        );
        assert_eq!(
            align_of::<GitReferenceFormatFlags>(),
            align_of::<ffi::git_reference_format_t>()
        );
    }

    #[test]
    fn reference_format_complement_stays_within_published_bits() {
        assert_eq!(
            !GitReferenceFormatFlags::ALLOW_ONELEVEL,
            GitReferenceFormatFlags::REFSPEC_PATTERN | GitReferenceFormatFlags::REFSPEC_SHORTHAND
        );
    }

    #[test]
    fn closure_implements_reference_name_callback() {
        let mut seen = |name: &CStr| i32::from(name == c"refs/heads/main");
        assert_eq!(
            GitReferenceForeachNameCallback::call(&mut seen, c"refs/heads/main"),
            1
        );
    }
}

#[cfg(test)]
mod reference_name_callback_traversal_tests {
    use super::*;

    use crate::refs::{git_reference_foreach_glob, git_reference_foreach_name};
    use crate::repository::git_repository_open_bare;

    /// A refcounted hold on the process-global libgit2 initialization.
    struct Libgit2Init;

    impl Libgit2Init {
        fn acquire() -> Self {
            // SAFETY: libgit2 initialization is process-global and refcounted;
            // `Drop` below balances this successful acquisition.
            assert!(unsafe { ffi::git_libgit2_init() } > 0);
            Self
        }
    }

    impl Drop for Libgit2Init {
        fn drop(&mut self) {
            // SAFETY: balances this guard's initialization; every repository
            // opened under it is released first.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    /// A hand-built bare repository holding two loose branch references.
    ///
    /// Name iteration walks `refs/` and never resolves the recorded IDs, so
    /// the branches need no objects behind them.
    struct BareRepo(std::path::PathBuf);

    impl BareRepo {
        fn create(tag: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("crustify-refs-cb-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(path.join("objects")).expect("a loose-object directory");
            std::fs::create_dir_all(path.join("refs/heads")).expect("a refs directory");
            std::fs::write(path.join("HEAD"), b"ref: refs/heads/main\n").expect("a HEAD file");
            std::fs::write(
                path.join("config"),
                b"[core]\n\trepositoryformatversion = 0\n\tbare = true\n",
            )
            .expect("a config file");
            for branch in ["main", "topic"] {
                std::fs::write(
                    path.join("refs/heads").join(branch),
                    b"1111111111111111111111111111111111111111\n",
                )
                .expect("a loose reference file");
            }
            Self(path)
        }

        fn c_path(&self) -> std::ffi::CString {
            std::ffi::CString::new(self.0.to_str().expect("a UTF-8 temporary path"))
                .expect("a path without interior NUL")
        }
    }

    impl Drop for BareRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn traversal_hands_the_callback_transient_reference_names() {
        let _init = Libgit2Init::acquire();
        let directory = BareRepo::create("visit");
        let mut owner = git_repository_open_bare(&directory.c_path())
            .expect("the hand-built directory is a bare repository");

        let mut seen = Vec::new();
        let mut collect = |name: &CStr| {
            // The borrow is call-scoped, so retaining it requires a copy.
            seen.push(name.to_owned());
            0
        };
        git_reference_foreach_name(&mut owner.as_mut(), &mut collect)
            .expect("iterating the hand-built references");

        seen.sort();
        assert_eq!(
            seen,
            [
                c"refs/heads/main".to_owned(),
                c"refs/heads/topic".to_owned()
            ]
        );
    }

    #[test]
    fn a_nonzero_callback_result_stops_the_traversal_and_propagates() {
        let _init = Libgit2Init::acquire();
        let directory = BareRepo::create("stop");
        let mut owner = git_repository_open_bare(&directory.c_path())
            .expect("the hand-built directory is a bare repository");

        let mut visits = 0;
        let mut stop = |_: &CStr| {
            visits += 1;
            -37
        };
        assert_eq!(
            git_reference_foreach_name(&mut owner.as_mut(), &mut stop),
            Err(-37)
        );
        assert_eq!(visits, 1);

        let mut globbed = Vec::new();
        let mut collect = |name: &CStr| {
            globbed.push(name.to_owned());
            0
        };
        git_reference_foreach_glob(&mut owner.as_mut(), c"refs/heads/t*", &mut collect)
            .expect("iterating the matching references");
        assert_eq!(globbed, [c"refs/heads/topic".to_owned()]);
    }
}
