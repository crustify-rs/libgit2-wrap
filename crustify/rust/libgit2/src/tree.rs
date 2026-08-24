//! Safe wrappers for libgit2 tree APIs.

use core::ptr::NonNull;

use ffibox::{CBox, CCloned, define_ctype, impl_dropped};

use crate::ffi;

define_ctype!(
    /// Wraps: git_tree
    /// An opaque, reference-counted Git tree.
    ///
    /// Owned handles release one cache reference with `git_tree_free`, while
    /// cloning acquires another reference with `git_tree_dup`. Repository-backed
    /// trees borrow their repository, which must remain alive while they are used.
    GitTree,
    GitTreeRef,
    GitTreeMut,
    ffi::git_tree
);

/// An owned reference to a Git tree.
pub type GitTreeOwned = CBox<GitTree>;

// SAFETY: `git_tree_free` consumes one reference to a complete tree and
// releases the allocation only when its underlying object cache refcount
// reaches zero. `GitTree` is transparent over the corresponding bindgen type.
impl_dropped!(GitTree, ffi::git_tree, ffi::git_tree_free);

// SAFETY: `git_tree_dup` increments the live tree's underlying object refcount
// and writes the same pointer to its non-null output slot. The new reference is
// independently released by the `CDropped` implementation above.
unsafe impl CCloned for GitTree {
    unsafe fn c_clone(obj: NonNull<Self>) -> Option<NonNull<Self>> {
        let mut duplicate = core::ptr::null_mut();
        // SAFETY: the `CCloned` caller supplies a live tree; `duplicate` is a
        // valid output slot, and the wrapper is layout-compatible with
        // `ffi::git_tree`.
        let result = unsafe {
            ffi::git_tree_dup(
                core::ptr::addr_of_mut!(duplicate),
                obj.as_ptr().cast::<ffi::git_tree>(),
            )
        };
        debug_assert_eq!(result, 0);
        NonNull::new(duplicate.cast::<Self>())
    }
}

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
    /// Wraps: git_tree_entry
    /// An opaque entry in a Git tree.
    ///
    /// A borrowed handle may refer to storage owned by a tree. Owned entries,
    /// such as deep copies, release their self-contained allocation with
    /// `git_tree_entry_free`.
    GitTreeEntry,
    GitTreeEntryRef,
    GitTreeEntryMut,
    ffi::git_tree_entry
);

/// An owned, self-contained tree entry.
pub type GitTreeEntryOwned = CBox<GitTreeEntry>;

// SAFETY: `git_tree_entry_free` is the public destructor for a user-owned,
// self-contained tree entry and accepts null, although `CDropped` supplies a
// live non-null allocation.
impl_dropped!(GitTreeEntry, ffi::git_tree_entry, ffi::git_tree_entry_free);

// SAFETY: `git_tree_entry_dup` leaves its live source unchanged and, on
// success, writes a fresh self-contained allocation that is independently
// releasable by `git_tree_entry_free`.
unsafe impl CCloned for GitTreeEntry {
    unsafe fn c_clone(obj: NonNull<Self>) -> Option<NonNull<Self>> {
        let mut duplicate = core::ptr::null_mut();
        // SAFETY: the `CCloned` caller supplies a live source, `duplicate` is
        // a valid output slot, and the wrapper is layout-compatible with the
        // corresponding bindgen type.
        let result = unsafe {
            ffi::git_tree_entry_dup(
                core::ptr::addr_of_mut!(duplicate),
                obj.as_ptr().cast::<ffi::git_tree_entry>(),
            )
        };

        if result == 0 {
            NonNull::new(duplicate.cast::<Self>())
        } else {
            debug_assert!(duplicate.is_null());
            None
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
    use core::ptr;

    use ffibox::{CCell, CCloned, CDropped};

    use super::*;

    #[test]
    fn opaque_tree_preserves_layout_and_refcount_contracts() {
        fn assert_cell<T: CCell>() {}
        fn assert_refcounted<T: CDropped + CCloned>() {}

        assert_cell::<GitTree>();
        assert_refcounted::<GitTree>();
        assert_eq!(size_of::<GitTree>(), size_of::<ffi::git_tree>());
        assert_eq!(align_of::<GitTree>(), align_of::<ffi::git_tree>());
        assert_eq!(
            size_of::<GitTreeRef<'_>>(),
            size_of::<*const ffi::git_tree>()
        );
        assert_eq!(size_of::<GitTreeMut<'_>>(), size_of::<*mut ffi::git_tree>());
        assert_eq!(size_of::<GitTreeOwned>(), size_of::<*mut ffi::git_tree>());
    }

    #[test]
    fn null_tree_seams_create_no_handle() {
        // SAFETY: these conversions explicitly accept null and return `None`
        // without borrowing or adopting an object.
        unsafe {
            assert!(GitTreeRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitTreeMut::from_ptr(ptr::null_mut()).is_none());
            assert!(GitTreeOwned::from_raw(ptr::null_mut()).is_none());
        }
    }

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
    fn opaque_entry_representation_and_handles_match_the_c_seam() {
        assert_eq!(size_of::<GitTreeEntry>(), size_of::<ffi::git_tree_entry>());
        assert_eq!(
            align_of::<GitTreeEntry>(),
            align_of::<ffi::git_tree_entry>()
        );
        assert_eq!(
            size_of::<GitTreeEntryRef<'static>>(),
            size_of::<*const ffi::git_tree_entry>()
        );
        assert_eq!(
            size_of::<GitTreeEntryMut<'static>>(),
            size_of::<*mut ffi::git_tree_entry>()
        );
        assert_eq!(
            size_of::<Option<GitTreeEntryOwned>>(),
            size_of::<*mut ffi::git_tree_entry>()
        );
    }

    #[test]
    fn borrowed_handles_preserve_the_entry_pointer() {
        let storage = Box::new(MaybeUninit::<ffi::git_tree_entry>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_tree_entry>();

        {
            // SAFETY: `raw` addresses live, suitably aligned opaque storage
            // and remains live for the duration of the shared handle.
            let shared = unsafe { GitTreeEntryRef::from_ptr(raw) }.unwrap();
            assert_eq!(shared.as_ptr(), raw.cast_const());
        }

        {
            // SAFETY: the shared handle is gone, the storage remains live,
            // and this scope has exclusive access to it.
            let mut exclusive = unsafe { GitTreeEntryMut::from_ptr(raw) }.unwrap();
            assert_eq!(exclusive.as_ref().as_ptr(), raw.cast_const());
            assert_eq!(exclusive.as_mut_ptr(), raw);
        }

        // SAFETY: `raw` came from `Box::into_raw`, no handle remains, and the
        // cast recovers the allocation's original type.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_tree_entry>>()) });
    }

    #[test]
    fn tree_entry_registers_deep_copy_and_drop_lifecycle() {
        fn assert_lifecycle<T: CDropped + CCloned>() {}
        assert_lifecycle::<GitTreeEntry>();
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
