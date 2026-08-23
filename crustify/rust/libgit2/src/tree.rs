//! Safe wrappers for libgit2 tree APIs.

use ffibox::{define_ctype, impl_dropped};

use crate::ffi;

/// Wraps: git_treewalk_mode
/// Selects whether a tree walk visits each entry before or after its children.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
#[repr(u32)]
pub enum TreeWalkMode {
    /// Visit an entry before visiting its children.
    Pre = ffi::git_treewalk_mode_GIT_TREEWALK_PRE,
    /// Visit an entry after visiting its children.
    Post = ffi::git_treewalk_mode_GIT_TREEWALK_POST,
}

/// A C value that is not a published [`TreeWalkMode`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidTreeWalkMode(ffi::git_treewalk_mode);

impl InvalidTreeWalkMode {
    /// Returns the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_treewalk_mode {
        self.0
    }
}

impl From<TreeWalkMode> for ffi::git_treewalk_mode {
    fn from(mode: TreeWalkMode) -> Self {
        mode as Self
    }
}

impl TryFrom<ffi::git_treewalk_mode> for TreeWalkMode {
    type Error = InvalidTreeWalkMode;

    fn try_from(mode: ffi::git_treewalk_mode) -> Result<Self, Self::Error> {
        match mode {
            ffi::git_treewalk_mode_GIT_TREEWALK_PRE => Ok(Self::Pre),
            ffi::git_treewalk_mode_GIT_TREEWALK_POST => Ok(Self::Post),
            value => Err(InvalidTreeWalkMode(value)),
        }
    }
}

/// Wraps: git_tree_update_t
/// Action applied to a path by a tree update.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TreeUpdateType {
    /// Insert a new entry or replace the entry at the path.
    Upsert = ffi::git_tree_update_t_GIT_TREE_UPDATE_UPSERT,
    /// Remove the entry at the path.
    Remove = ffi::git_tree_update_t_GIT_TREE_UPDATE_REMOVE,
}

/// A raw tree-update action not published by this libgit2 API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidTreeUpdateType(ffi::git_tree_update_t);

impl InvalidTreeUpdateType {
    /// Return the unrecognized C value.
    #[must_use]
    pub const fn value(self) -> ffi::git_tree_update_t {
        self.0
    }
}

impl From<TreeUpdateType> for ffi::git_tree_update_t {
    fn from(value: TreeUpdateType) -> Self {
        value as Self
    }
}

impl TryFrom<ffi::git_tree_update_t> for TreeUpdateType {
    type Error = InvalidTreeUpdateType;

    fn try_from(value: ffi::git_tree_update_t) -> Result<Self, Self::Error> {
        match value {
            ffi::git_tree_update_t_GIT_TREE_UPDATE_UPSERT => Ok(Self::Upsert),
            ffi::git_tree_update_t_GIT_TREE_UPDATE_REMOVE => Ok(Self::Remove),
            value => Err(InvalidTreeUpdateType(value)),
        }
    }
}

define_ctype!(
    /// Wraps: git_treebuilder
    /// An opaque in-memory tree builder owned by libgit2.
    ///
    /// The builder borrows the repository supplied during construction without
    /// retaining it. Any safe constructor must therefore keep that repository
    /// alive for the lifetime of the builder. Owned pointers are released with
    /// `git_treebuilder_free`.
    TreeBuilder,
    TreeBuilderRef,
    TreeBuilderMut,
    ffi::git_treebuilder
);

// SAFETY: `git_treebuilder_free` is the public destructor for a complete
// `git_treebuilder` allocation and accepts null, although `CDropped` supplies
// a live non-null allocation. `TreeBuilder` is transparent over the matching
// bindgen C type.
impl_dropped!(TreeBuilder, ffi::git_treebuilder, ffi::git_treebuilder_free);

#[cfg(test)]
mod tests {
    use core::mem::{MaybeUninit, align_of, size_of};

    use ffibox::CDropped;

    use super::*;

    #[test]
    fn tree_update_types_round_trip_through_the_c_type() {
        for update in [TreeUpdateType::Upsert, TreeUpdateType::Remove] {
            let raw = ffi::git_tree_update_t::from(update);
            assert_eq!(TreeUpdateType::try_from(raw), Ok(update));
        }
    }

    #[test]
    fn invalid_tree_update_types_are_rejected() {
        let invalid = ffi::git_tree_update_t_GIT_TREE_UPDATE_REMOVE + 1;
        let error = TreeUpdateType::try_from(invalid).unwrap_err();
        assert_eq!(error.value(), invalid);
    }

    #[test]
    fn tree_update_type_matches_the_c_abi_scalar() {
        assert_eq!(
            size_of::<TreeUpdateType>(),
            size_of::<ffi::git_tree_update_t>()
        );
        assert_eq!(
            align_of::<TreeUpdateType>(),
            align_of::<ffi::git_tree_update_t>()
        );
    }

    #[test]
    fn opaque_builder_representation_and_handles_match_the_c_seam() {
        assert_eq!(size_of::<TreeBuilder>(), size_of::<ffi::git_treebuilder>());
        assert_eq!(
            align_of::<TreeBuilder>(),
            align_of::<ffi::git_treebuilder>()
        );
        assert_eq!(
            size_of::<TreeBuilderRef<'static>>(),
            size_of::<*const ffi::git_treebuilder>()
        );
        assert_eq!(
            size_of::<TreeBuilderMut<'static>>(),
            size_of::<*mut ffi::git_treebuilder>()
        );

        let storage = Box::new(MaybeUninit::<ffi::git_treebuilder>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_treebuilder>();

        {
            // SAFETY: `raw` addresses live storage for the bindgen opaque type
            // and remains live without mutation for this scope.
            let shared = unsafe { TreeBuilderRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live, and
            // this scope has exclusive access to it.
            let mut exclusive = unsafe { TreeBuilderMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from `Box::into_raw`, no handle remains, and this
        // cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_treebuilder>>()) });
    }

    #[test]
    fn tree_builder_registers_its_c_destructor() {
        fn assert_dropped<T: CDropped>() {}
        assert_dropped::<TreeBuilder>();
    }

    #[test]
    fn tree_walk_modes_round_trip_through_the_c_type() {
        for mode in [TreeWalkMode::Pre, TreeWalkMode::Post] {
            let raw = ffi::git_treewalk_mode::from(mode);
            assert_eq!(TreeWalkMode::try_from(raw), Ok(mode));
        }
    }

    #[test]
    fn invalid_tree_walk_mode_is_rejected_without_constructing_an_enum() {
        let invalid = ffi::git_treewalk_mode_GIT_TREEWALK_POST + 1;
        let error = TreeWalkMode::try_from(invalid).unwrap_err();
        assert_eq!(error.value(), invalid);
    }

    #[test]
    fn tree_walk_mode_matches_the_c_abi_scalar() {
        assert_eq!(
            size_of::<TreeWalkMode>(),
            size_of::<ffi::git_treewalk_mode>()
        );
        assert_eq!(
            align_of::<TreeWalkMode>(),
            align_of::<ffi::git_treewalk_mode>()
        );
    }
}
