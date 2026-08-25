//! Safe wrappers for libgit2 repository APIs.

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

use crate::ffi;

/// Wraps: git_repository_init_flag_t
/// A checked set of optional repository-initialization behaviors.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GitRepositoryInitFlags(ffi::git_repository_init_flag_t);

impl GitRepositoryInitFlags {
    /// Use libgit2's default initialization behavior.
    pub const EMPTY: Self = Self(0);
    /// Create a bare repository without a working directory.
    pub const BARE: Self = Self(ffi::git_repository_init_flag_t_GIT_REPOSITORY_INIT_BARE);
    /// Fail if the destination already appears to contain a repository.
    pub const NO_REINIT: Self = Self(ffi::git_repository_init_flag_t_GIT_REPOSITORY_INIT_NO_REINIT);
    /// Create the trailing repository and working-directory components.
    pub const MKDIR: Self = Self(ffi::git_repository_init_flag_t_GIT_REPOSITORY_INIT_MKDIR);
    /// Recursively create all missing path components.
    pub const MKPATH: Self = Self(ffi::git_repository_init_flag_t_GIT_REPOSITORY_INIT_MKPATH);
    /// Load templates from an external template directory when available.
    pub const EXTERNAL_TEMPLATE: Self =
        Self(ffi::git_repository_init_flag_t_GIT_REPOSITORY_INIT_EXTERNAL_TEMPLATE);
    /// Store relative paths in an alternate working directory's gitlink.
    pub const RELATIVE_GITLINK: Self =
        Self(ffi::git_repository_init_flag_t_GIT_REPOSITORY_INIT_RELATIVE_GITLINK);
    /// Every initialization flag published by this libgit2 version.
    pub const ALL: Self = Self(
        Self::BARE.0
            | Self::NO_REINIT.0
            | Self::MKDIR.0
            | Self::MKPATH.0
            | Self::EXTERNAL_TEMPLATE.0
            | Self::RELATIVE_GITLINK.0,
    );

    /// Converts raw bits when every bit is published by this libgit2 version.
    #[must_use]
    pub const fn from_bits(bits: ffi::git_repository_init_flag_t) -> Option<Self> {
        if bits & !Self::ALL.0 == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }

    /// Returns the underlying libgit2 flag bits.
    #[must_use]
    pub const fn bits(self) -> ffi::git_repository_init_flag_t {
        self.0
    }

    /// Returns whether no optional behavior is enabled.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether every flag in `other` is enabled.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns whether any flag in `other` is enabled.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl From<GitRepositoryInitFlags> for ffi::git_repository_init_flag_t {
    fn from(flags: GitRepositoryInitFlags) -> Self {
        flags.bits()
    }
}

impl BitOr for GitRepositoryInitFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for GitRepositoryInitFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for GitRepositoryInitFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for GitRepositoryInitFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for GitRepositoryInitFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(self.0 ^ Self::ALL.0)
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn repository_init_flags_compose_and_round_trip() {
        let flags = GitRepositoryInitFlags::BARE
            | GitRepositoryInitFlags::MKPATH
            | GitRepositoryInitFlags::EXTERNAL_TEMPLATE;
        assert!(flags.contains(GitRepositoryInitFlags::BARE));
        assert!(flags.intersects(GitRepositoryInitFlags::MKPATH));
        assert!(!flags.intersects(GitRepositoryInitFlags::NO_REINIT));
        assert_eq!(GitRepositoryInitFlags::from_bits(flags.bits()), Some(flags));
        assert_eq!(
            ffi::git_repository_init_flag_t::from(flags),
            (1 << 0) | (1 << 4) | (1 << 5)
        );
    }

    #[test]
    fn repository_init_flags_reject_unpublished_bits() {
        assert_eq!(
            GitRepositoryInitFlags::from_bits(GitRepositoryInitFlags::ALL.bits()),
            Some(GitRepositoryInitFlags::ALL)
        );
        assert_eq!(GitRepositoryInitFlags::from_bits(1 << 2), None);
    }

    #[test]
    fn repository_init_flags_match_the_c_enum_layout() {
        assert_eq!(
            size_of::<GitRepositoryInitFlags>(),
            size_of::<ffi::git_repository_init_flag_t>()
        );
        assert_eq!(
            align_of::<GitRepositoryInitFlags>(),
            align_of::<ffi::git_repository_init_flag_t>()
        );
    }
}

/// Wraps: git_repository_init_mode_t
/// A standard shared-repository permission mode.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GitRepositoryInitMode {
    /// Use permissions derived from the process umask.
    #[default]
    Umask = ffi::git_repository_init_mode_t_GIT_REPOSITORY_INIT_SHARED_UMASK,
    /// Make the repository group-writable and preserve group assignment.
    Group = ffi::git_repository_init_mode_t_GIT_REPOSITORY_INIT_SHARED_GROUP,
    /// Also make the repository world-readable.
    All = ffi::git_repository_init_mode_t_GIT_REPOSITORY_INIT_SHARED_ALL,
}

impl GitRepositoryInitMode {
    /// Returns the underlying C enum value.
    #[must_use]
    pub const fn as_raw(self) -> ffi::git_repository_init_mode_t {
        self as ffi::git_repository_init_mode_t
    }
}

/// A raw repository-initialization mode that is not one of libgit2's standard modes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitRepositoryInitMode(ffi::git_repository_init_mode_t);

impl InvalidGitRepositoryInitMode {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_repository_init_mode_t {
        self.0
    }
}

impl From<GitRepositoryInitMode> for ffi::git_repository_init_mode_t {
    fn from(mode: GitRepositoryInitMode) -> Self {
        mode.as_raw()
    }
}

impl TryFrom<ffi::git_repository_init_mode_t> for GitRepositoryInitMode {
    type Error = InvalidGitRepositoryInitMode;

    fn try_from(raw: ffi::git_repository_init_mode_t) -> Result<Self, Self::Error> {
        match raw {
            ffi::git_repository_init_mode_t_GIT_REPOSITORY_INIT_SHARED_UMASK => Ok(Self::Umask),
            ffi::git_repository_init_mode_t_GIT_REPOSITORY_INIT_SHARED_GROUP => Ok(Self::Group),
            ffi::git_repository_init_mode_t_GIT_REPOSITORY_INIT_SHARED_ALL => Ok(Self::All),
            value => Err(InvalidGitRepositoryInitMode(value)),
        }
    }
}

/// Wraps: git_repository_item_t
/// A checked item in libgit2's repository layout.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GitRepositoryItem {
    /// The repository's Git directory.
    GitDir = ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_GITDIR,
    /// The repository's working directory.
    WorkDir = ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_WORKDIR,
    /// The shared common directory.
    CommonDir = ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_COMMONDIR,
    /// The index file.
    Index = ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_INDEX,
    /// The object database directory.
    Objects = ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_OBJECTS,
    /// The references directory.
    Refs = ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_REFS,
    /// The packed-references file.
    PackedRefs = ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_PACKED_REFS,
    /// The legacy remotes directory.
    Remotes = ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_REMOTES,
    /// The repository configuration file.
    Config = ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_CONFIG,
    /// The repository information directory.
    Info = ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_INFO,
    /// The hooks directory.
    Hooks = ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_HOOKS,
    /// The reflog directory.
    Logs = ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_LOGS,
    /// The submodule metadata directory.
    Modules = ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_MODULES,
    /// The linked-worktree metadata directory.
    Worktrees = ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_WORKTREES,
    /// The worktree-specific configuration file.
    WorktreeConfig = ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_WORKTREE_CONFIG,
}

impl GitRepositoryItem {
    /// Returns the underlying C enum value.
    #[must_use]
    pub const fn as_raw(self) -> ffi::git_repository_item_t {
        self as ffi::git_repository_item_t
    }
}

/// A raw repository-layout item not published for use by callers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGitRepositoryItem(ffi::git_repository_item_t);

impl InvalidGitRepositoryItem {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_repository_item_t {
        self.0
    }
}

impl From<GitRepositoryItem> for ffi::git_repository_item_t {
    fn from(item: GitRepositoryItem) -> Self {
        item.as_raw()
    }
}

impl TryFrom<ffi::git_repository_item_t> for GitRepositoryItem {
    type Error = InvalidGitRepositoryItem;

    fn try_from(raw: ffi::git_repository_item_t) -> Result<Self, Self::Error> {
        match raw {
            ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_GITDIR => Ok(Self::GitDir),
            ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_WORKDIR => Ok(Self::WorkDir),
            ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_COMMONDIR => Ok(Self::CommonDir),
            ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_INDEX => Ok(Self::Index),
            ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_OBJECTS => Ok(Self::Objects),
            ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_REFS => Ok(Self::Refs),
            ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_PACKED_REFS => Ok(Self::PackedRefs),
            ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_REMOTES => Ok(Self::Remotes),
            ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_CONFIG => Ok(Self::Config),
            ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_INFO => Ok(Self::Info),
            ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_HOOKS => Ok(Self::Hooks),
            ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_LOGS => Ok(Self::Logs),
            ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_MODULES => Ok(Self::Modules),
            ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_WORKTREES => Ok(Self::Worktrees),
            ffi::git_repository_item_t_GIT_REPOSITORY_ITEM_WORKTREE_CONFIG => {
                Ok(Self::WorktreeConfig)
            }
            value => Err(InvalidGitRepositoryItem(value)),
        }
    }
}

#[cfg(test)]
mod repository_enum_tests {
    use core::mem::{align_of, size_of};

    use super::*;

    #[test]
    fn standard_repository_modes_round_trip() {
        for mode in [
            GitRepositoryInitMode::Umask,
            GitRepositoryInitMode::Group,
            GitRepositoryInitMode::All,
        ] {
            let raw = ffi::git_repository_init_mode_t::from(mode);
            assert_eq!(GitRepositoryInitMode::try_from(raw), Ok(mode));
        }
        let custom = 0o755;
        assert_eq!(
            GitRepositoryInitMode::try_from(custom).unwrap_err().value(),
            custom
        );
    }

    #[test]
    fn repository_items_round_trip_and_reject_the_private_sentinel() {
        let items = [
            GitRepositoryItem::GitDir,
            GitRepositoryItem::WorkDir,
            GitRepositoryItem::CommonDir,
            GitRepositoryItem::Index,
            GitRepositoryItem::Objects,
            GitRepositoryItem::Refs,
            GitRepositoryItem::PackedRefs,
            GitRepositoryItem::Remotes,
            GitRepositoryItem::Config,
            GitRepositoryItem::Info,
            GitRepositoryItem::Hooks,
            GitRepositoryItem::Logs,
            GitRepositoryItem::Modules,
            GitRepositoryItem::Worktrees,
            GitRepositoryItem::WorktreeConfig,
        ];
        for item in items {
            let raw = ffi::git_repository_item_t::from(item);
            assert_eq!(GitRepositoryItem::try_from(raw), Ok(item));
        }
        let sentinel = ffi::git_repository_item_t_GIT_REPOSITORY_ITEM__LAST;
        assert_eq!(
            GitRepositoryItem::try_from(sentinel).unwrap_err().value(),
            sentinel
        );
    }

    #[test]
    fn repository_enums_match_their_c_layouts() {
        assert_eq!(
            size_of::<GitRepositoryInitMode>(),
            size_of::<ffi::git_repository_init_mode_t>()
        );
        assert_eq!(
            align_of::<GitRepositoryInitMode>(),
            align_of::<ffi::git_repository_init_mode_t>()
        );
        assert_eq!(
            size_of::<GitRepositoryItem>(),
            size_of::<ffi::git_repository_item_t>()
        );
        assert_eq!(
            align_of::<GitRepositoryItem>(),
            align_of::<ffi::git_repository_item_t>()
        );
    }
}
