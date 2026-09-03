//! Safe wrappers for libgit2 tree APIs.

use core::ffi::{CStr, c_char, c_void};
use core::marker::PhantomData;
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CBox, CCloned, CSlice, define_ctype, impl_dropped};

use crate::api::tree::{GitTreebuilderFilterCallback, GitTreewalkCallback};
use crate::api::types::{GitFileMode, GitObjectType};
use crate::ffi;
use crate::object::{GitObjectOwned, RepositoryObject};
use crate::oid::{Oid, OidMut, OidRef};
use crate::repository::GitRepositoryRef;

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

/// An owned tree tied to the repository pointer retained by its base object.
pub struct RepositoryTree<'repo> {
    inner: GitTreeOwned,
    _repository: PhantomData<GitRepositoryRef<'repo>>,
}

#[cfg(test)]
mod io_equiv {
    #![allow(clippy::undocumented_unsafe_blocks)]

    use super::*;
    use crate::io_equiv_support::{HistoryFixture, Libgit2Init};

    #[derive(Debug, Eq, PartialEq)]
    struct TreeObservation {
        names: Vec<Vec<u8>>,
        walked: Vec<(Vec<u8>, Vec<u8>)>,
        nested_name: Vec<u8>,
        nested_mode: ffi::git_filemode_t,
        builder_entries: usize,
        rewritten_id: Vec<u8>,
    }

    unsafe fn raw_tree(repository: *mut ffi::git_repository) -> TreeObservation {
        let mut id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut id, repository, c"HEAD".as_ptr()) },
            0
        );
        let mut commit = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_commit_lookup(&mut commit, repository, &id) },
            0
        );
        let mut tree = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_commit_tree(&mut tree, commit) }, 0);
        let mut names = Vec::new();
        for index in 0..unsafe { ffi::git_tree_entrycount(tree) } {
            let entry = unsafe { ffi::git_tree_entry_byindex(tree, index) };
            names.push(
                unsafe { CStr::from_ptr(ffi::git_tree_entry_name(entry)) }
                    .to_bytes()
                    .to_vec(),
            );
            assert!(
                !unsafe { ffi::git_tree_entry_byid(tree, ffi::git_tree_entry_id(entry)) }.is_null()
            );
        }
        let mut nested = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_tree_entry_bypath(&mut nested, tree, c"src/alpha.c".as_ptr()) },
            0
        );
        let nested_name = unsafe { CStr::from_ptr(ffi::git_tree_entry_name(nested)) }
            .to_bytes()
            .to_vec();
        let nested_mode = unsafe { ffi::git_tree_entry_filemode(nested) };
        unsafe extern "C" fn walk(
            root: *const core::ffi::c_char,
            entry: *const ffi::git_tree_entry,
            payload: *mut core::ffi::c_void,
        ) -> i32 {
            let output = unsafe { &mut *payload.cast::<Vec<(Vec<u8>, Vec<u8>)>>() };
            output.push((
                unsafe { CStr::from_ptr(root) }.to_bytes().to_vec(),
                unsafe { CStr::from_ptr(ffi::git_tree_entry_name(entry)) }
                    .to_bytes()
                    .to_vec(),
            ));
            0
        }
        let mut walked = Vec::new();
        assert_eq!(
            unsafe {
                ffi::git_tree_walk(
                    tree,
                    ffi::git_treewalk_mode_GIT_TREEWALK_PRE,
                    Some(walk),
                    core::ptr::from_mut(&mut walked).cast(),
                )
            },
            0
        );
        let mut builder = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_treebuilder_new(&mut builder, repository, tree) },
            0
        );
        let readme = unsafe { ffi::git_tree_entry_byname(tree, c"README.md".as_ptr()) };
        let readme_id = unsafe { *ffi::git_tree_entry_id(readme) };
        let readme_mode = unsafe { ffi::git_tree_entry_filemode(readme) };
        assert_eq!(
            unsafe { ffi::git_treebuilder_remove(builder, c"README.md".as_ptr()) },
            0
        );
        let mut inserted = core::ptr::null();
        assert_eq!(
            unsafe {
                ffi::git_treebuilder_insert(
                    &mut inserted,
                    builder,
                    c"README.md".as_ptr(),
                    &readme_id,
                    readme_mode,
                )
            },
            0
        );
        assert_eq!(unsafe { ffi::git_tree_entry_cmp(readme, inserted) }, 0);
        let builder_entries = unsafe { ffi::git_treebuilder_entrycount(builder) };
        let mut rewritten = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_treebuilder_write(&mut rewritten, builder) },
            0
        );
        unsafe {
            ffi::git_treebuilder_free(builder);
            ffi::git_tree_entry_free(nested);
            ffi::git_tree_free(tree);
            ffi::git_commit_free(commit);
        }
        TreeObservation {
            names,
            walked,
            nested_name,
            nested_mode,
            builder_entries,
            rewritten_id: rewritten.id.to_vec(),
        }
    }

    fn safe_tree(repository: *mut ffi::git_repository) -> TreeObservation {
        let mut id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut id, repository, c"HEAD".as_ptr()) },
            0
        );
        let id_ref = unsafe { OidRef::from_ptr(core::ptr::from_mut(&mut id)) }.unwrap();
        let repository_ref =
            unsafe { crate::repository::GitRepositoryRef::from_ptr(repository) }.unwrap();
        let commit = crate::object_api::git_commit_lookup(repository_ref, id_ref).unwrap();
        let tree = crate::commit::git_commit_tree(commit.as_ref()).unwrap();
        let mut names = Vec::new();
        for index in 0..git_tree_entrycount(tree.as_ref()) {
            let entry = git_tree_entry_byindex(tree.as_ref(), index).unwrap();
            names.push(git_tree_entry_name(entry).to_bytes().to_vec());
            assert!(git_tree_entry_byid(tree.as_ref(), git_tree_entry_id(entry)).is_some());
        }
        let nested = git_tree_entry_bypath(tree.as_ref(), c"src/alpha.c").unwrap();
        let nested_name = git_tree_entry_name(nested.as_ref()).to_bytes().to_vec();
        let nested_mode = git_tree_entry_filemode(nested.as_ref()).as_raw();
        let mut walked = Vec::new();
        let mut callback = |root: &CStr, entry: GitTreeEntryRef<'_>| {
            walked.push((
                root.to_bytes().to_vec(),
                git_tree_entry_name(entry).to_bytes().to_vec(),
            ));
            0
        };
        git_tree_walk(tree.as_ref(), TreeWalkMode::Pre, &mut callback).unwrap();
        let readme = git_tree_entry_byname(tree.as_ref(), c"README.md").unwrap();
        let mut builder = git_treebuilder_new(repository_ref, Some(tree.as_ref())).unwrap();
        let mut builder_handle = builder.as_mut();
        git_treebuilder_remove(&mut builder_handle, c"README.md").unwrap();
        let inserted = git_treebuilder_insert(
            &mut builder_handle,
            c"README.md",
            git_tree_entry_id(readme),
            git_tree_entry_filemode(readme),
        )
        .unwrap();
        assert_eq!(
            git_tree_entry_cmp(readme, inserted),
            core::cmp::Ordering::Equal
        );
        let builder_entries = git_treebuilder_entrycount(builder_handle.as_ref());
        let mut rewritten = git_treebuilder_write(&mut builder_handle).unwrap();
        let rewritten_id =
            unsafe { OidRef::from_ptr(core::ptr::from_mut(&mut rewritten).cast::<ffi::git_oid>()) }
                .unwrap()
                .raw_bytes()
                .elems()
                .collect();
        TreeObservation {
            names,
            walked,
            nested_name,
            nested_mode,
            builder_entries,
            rewritten_id,
        }
    }

    #[test]
    fn io_equiv_tree_lookup_walk_path_and_builder_round_trip() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("tree-surface-raw");
        let safe = HistoryFixture::new("tree-surface-safe");
        let raw_observation = unsafe { raw_tree(raw.repository.as_ptr()) };
        assert_eq!(raw_observation, safe_tree(safe.repository.as_ptr()));
        assert_eq!(raw_observation.walked.len(), 4);
    }

    unsafe fn raw_filter_and_update(
        repository: *mut ffi::git_repository,
    ) -> (usize, usize, Vec<u8>) {
        unsafe extern "C" fn remove_readme(
            entry: *const ffi::git_tree_entry,
            _: *mut core::ffi::c_void,
        ) -> i32 {
            i32::from(unsafe { CStr::from_ptr(ffi::git_tree_entry_name(entry)) } == c"README.md")
        }
        let mut head = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut head, repository, c"HEAD".as_ptr()) },
            0
        );
        let mut commit = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_commit_lookup(&mut commit, repository, &head) },
            0
        );
        let mut tree = core::ptr::null_mut();
        assert_eq!(unsafe { ffi::git_commit_tree(&mut tree, commit) }, 0);
        let readme = unsafe { ffi::git_tree_entry_byname(tree, c"README.md".as_ptr()) };
        let mut builder = core::ptr::null_mut();
        assert_eq!(
            unsafe { ffi::git_treebuilder_new(&mut builder, repository, tree) },
            0
        );
        assert!(!unsafe { ffi::git_treebuilder_get(builder, c"README.md".as_ptr()) }.is_null());
        assert_eq!(
            unsafe {
                ffi::git_treebuilder_filter(builder, Some(remove_readme), core::ptr::null_mut())
            },
            0
        );
        let filtered = unsafe { ffi::git_treebuilder_entrycount(builder) };
        assert_eq!(unsafe { ffi::git_treebuilder_clear(builder) }, 0);
        let cleared = unsafe { ffi::git_treebuilder_entrycount(builder) };
        unsafe { ffi::git_treebuilder_free(builder) };

        let updates = [
            ffi::git_tree_update {
                action: ffi::git_tree_update_t_GIT_TREE_UPDATE_REMOVE,
                id: unsafe { core::mem::zeroed() },
                filemode: ffi::git_filemode_t_GIT_FILEMODE_UNREADABLE,
                path: c"README.md".as_ptr(),
            },
            ffi::git_tree_update {
                action: ffi::git_tree_update_t_GIT_TREE_UPDATE_UPSERT,
                id: unsafe { *ffi::git_tree_entry_id(readme) },
                filemode: unsafe { ffi::git_tree_entry_filemode(readme) },
                path: c"COPY.md".as_ptr(),
            },
        ];
        let mut updated = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe {
                ffi::git_tree_create_updated(
                    &mut updated,
                    repository,
                    tree,
                    updates.len(),
                    updates.as_ptr(),
                )
            },
            0
        );
        unsafe {
            ffi::git_tree_free(tree);
            ffi::git_commit_free(commit);
        }
        (filtered, cleared, updated.id.to_vec())
    }

    fn safe_filter_and_update(repository: *mut ffi::git_repository) -> (usize, usize, Vec<u8>) {
        let mut head = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        assert_eq!(
            unsafe { ffi::git_reference_name_to_id(&mut head, repository, c"HEAD".as_ptr()) },
            0
        );
        let head = unsafe { OidRef::from_ptr(&raw mut head) }.unwrap();
        let repository_ref = unsafe { GitRepositoryRef::from_ptr(repository) }.unwrap();
        let commit = crate::object_api::git_commit_lookup(repository_ref, head).unwrap();
        let tree = crate::commit::git_commit_tree(commit.as_ref()).unwrap();
        let readme = git_tree_entry_byname(tree.as_ref(), c"README.md").unwrap();
        let mut builder = git_treebuilder_new(repository_ref, Some(tree.as_ref())).unwrap();
        assert!(git_treebuilder_get(builder.as_ref(), c"README.md").is_some());
        git_treebuilder_filter(&mut builder.as_mut(), &mut |entry: GitTreeEntryRef<'_>| {
            git_tree_entry_name(entry) == c"README.md"
        })
        .unwrap();
        let filtered = git_treebuilder_entrycount(builder.as_ref());
        git_treebuilder_clear(&mut builder.as_mut()).unwrap();
        let cleared = git_treebuilder_entrycount(builder.as_ref());

        let mut updates = [
            ffi::git_tree_update {
                action: ffi::git_tree_update_t_GIT_TREE_UPDATE_REMOVE,
                id: unsafe { core::mem::zeroed() },
                filemode: ffi::git_filemode_t_GIT_FILEMODE_UNREADABLE,
                path: c"README.md".as_ptr(),
            },
            ffi::git_tree_update {
                action: ffi::git_tree_update_t_GIT_TREE_UPDATE_UPSERT,
                id: unsafe { *git_tree_entry_id(readme).as_ptr() },
                filemode: git_tree_entry_filemode(readme).as_raw(),
                path: c"COPY.md".as_ptr(),
            },
        ];
        let ptr = NonNull::new(updates.as_mut_ptr().cast::<TreeUpdate>()).unwrap();
        let updates = unsafe { CSlice::from_raw_parts(ptr, updates.len()) };
        let mut updated =
            git_tree_create_updated(repository_ref, Some(tree.as_ref()), updates).unwrap();
        let updated = unsafe { OidRef::from_ptr(core::ptr::addr_of_mut!(updated).cast()) }.unwrap();
        (filtered, cleared, updated.raw_bytes().elems().collect())
    }

    #[test]
    fn io_equiv_treebuilder_get_filter_clear_and_create_updated() {
        let _libgit2 = Libgit2Init::acquire();
        let raw = HistoryFixture::new("tree-update-raw");
        let safe = HistoryFixture::new("tree-update-safe");
        let raw = unsafe { raw_filter_and_update(raw.repository.as_ptr()) };
        assert_eq!(raw, safe_filter_and_update(safe.repository.as_ptr()));
        assert_eq!(raw.1, 0);
    }
}

impl RepositoryTree<'_> {
    /// Borrows the tree.
    #[must_use]
    pub fn as_ref(&self) -> GitTreeRef<'_> {
        self.inner.as_ref()
    }
}

impl<'repo> RepositoryTree<'repo> {
    pub(crate) fn from_owned(inner: GitTreeOwned, repository: GitRepositoryRef<'repo>) -> Self {
        let _ = repository;
        Self {
            inner,
            _repository: PhantomData,
        }
    }
}

impl Clone for RepositoryTree<'_> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _repository: PhantomData,
        }
    }
}

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
    /// `git_treebuilder_new` stores the repository it is given in the builder
    /// and every later builder operation dereferences it, but the builder does
    /// not own it and `git_treebuilder_free` never releases it. Any safe
    /// constructor must therefore keep that repository alive for the whole
    /// lifetime of the builder. Owned pointers are released with
    /// `git_treebuilder_free`, which also frees the entries the builder holds.
    TreeBuilder,
    TreeBuilderRef,
    TreeBuilderMut,
    ffi::git_treebuilder
);

/// An owned in-memory tree builder.
pub type TreeBuilderOwned = CBox<TreeBuilder>;

/// An owned tree builder tied to the repository pointer it retains.
pub struct RepositoryTreeBuilder<'repo> {
    inner: TreeBuilderOwned,
    _repository: PhantomData<GitRepositoryRef<'repo>>,
}

impl RepositoryTreeBuilder<'_> {
    /// Borrows the builder.
    #[must_use]
    pub fn as_ref(&self) -> TreeBuilderRef<'_> {
        self.inner.as_ref()
    }

    /// Borrows the builder exclusively.
    #[must_use]
    pub fn as_mut(&mut self) -> TreeBuilderMut<'_> {
        self.inner.as_mut()
    }
}

// SAFETY: `git_treebuilder_free` is the public destructor for a complete
// `git_treebuilder` allocation and accepts null, although `CDropped` supplies
// a live non-null allocation. `TreeBuilder` is transparent over the matching
// bindgen C type.
impl_dropped!(TreeBuilder, ffi::git_treebuilder, ffi::git_treebuilder_free);

define_ctype!(
    /// Wraps: git_tree_update
    /// A layout-compatible descriptor for one tree update.
    ///
    /// The descriptor borrows its path and owns no external resources. Code
    /// that stores a path in it must keep that path alive until the descriptor
    /// is no longer observed by C.
    TreeUpdate,
    TreeUpdateRef,
    TreeUpdateMut,
    ffi::git_tree_update
);

impl<'a> TreeUpdateRef<'a> {
    /// Field: git_tree_update.action
    /// Returns the requested update action.
    ///
    /// A malformed C value is reported instead of being converted into an
    /// invalid Rust enum.
    pub fn action(&self) -> Result<TreeUpdateType, InvalidTreeUpdateType> {
        // SAFETY: the shared handle covers a live initialized descriptor, and
        // raw-place projection reads its scalar without forming a reference to
        // C-visible memory.
        let action = unsafe { addr_of!((*self.as_ptr()).action).read() };
        TreeUpdateType::try_from(action)
    }

    /// Field: git_tree_update.path
    /// Borrows the non-null, NUL-terminated path from the descriptor.
    #[must_use]
    pub fn path(&self) -> &'a CStr {
        // SAFETY: raw-place projection reads the pointer field without forming
        // a reference to C-visible descriptor storage.
        let path = unsafe { addr_of!((*self.as_ptr()).path).read() };
        assert!(!path.is_null(), "a valid tree update has a path");
        // SAFETY: a valid `git_tree_update` path is NUL-terminated and remains
        // live while the descriptor is observed. The check above establishes
        // a valid start and the handle bounds the returned borrow's lifetime.
        unsafe { CStr::from_ptr(path) }
    }

    /// Field: git_tree_update.id
    /// Borrows the inline object identifier.
    #[must_use]
    pub fn id(&self) -> OidRef<'a> {
        // SAFETY: the projected inline field is non-null, initialized, and
        // remains live for the enclosing shared handle's lifetime.
        unsafe { OidRef::from_ptr(addr_of!((*self.as_ptr()).id).cast_mut()) }
            .expect("an inline field is non-null")
    }

    /// Field: git_tree_update.filemode
    /// Returns the entry's file mode when C supplied a published value.
    #[must_use]
    pub fn filemode(&self) -> Option<GitFileMode> {
        // SAFETY: the shared handle covers a live initialized descriptor, and
        // raw-place projection reads its scalar without forming a reference to
        // C-visible memory.
        let filemode = unsafe { addr_of!((*self.as_ptr()).filemode).read() };
        GitFileMode::from_raw(filemode)
    }
}

impl TreeUpdateMut<'_> {
    /// Sets the requested update action.
    pub fn set_action(&mut self, action: TreeUpdateType) {
        // SAFETY: this exclusive handle permits a raw-place write, and the
        // checked Rust enum has the same representation as the C field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).action).write(action.into()) }
    }

    /// Stores a borrowed path in the descriptor.
    ///
    /// # Safety
    ///
    /// `path` must remain alive and unchanged until C can no longer observe
    /// this descriptor or the field is replaced. Rust cannot attach that
    /// lifetime to a pointer stored in the C layout.
    pub unsafe fn set_borrowed_path(&mut self, path: &CStr) {
        // SAFETY: this exclusive handle permits a raw-place write. `CStr`
        // supplies a non-null NUL-terminated pointer, and the caller upholds
        // the stored borrow's lifetime.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).path).write(path.as_ptr()) }
    }

    /// Borrows the inline object identifier exclusively.
    #[must_use]
    pub fn id_mut(&mut self) -> OidMut<'_> {
        // SAFETY: the projected inline field is non-null and initialized, and
        // this handle's exclusive reborrow prevents competing Rust handles.
        unsafe { OidMut::from_ptr(addr_of_mut!((*self.as_mut_ptr()).id)) }
            .expect("an inline field is non-null")
    }

    /// Sets the entry's file mode.
    pub fn set_filemode(&mut self, filemode: GitFileMode) {
        // SAFETY: this exclusive handle permits a raw-place write, and
        // `GitFileMode` only contains values published for the C field.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).filemode).write(filemode.into()) }
    }
}

/// Wraps: git_tree_entry_cmp
/// Compares two entries using Git's tree ordering.
#[must_use]
pub fn git_tree_entry_cmp(
    first: GitTreeEntryRef<'_>,
    second: GitTreeEntryRef<'_>,
) -> core::cmp::Ordering {
    // SAFETY: both entries are live shared borrows for this read-only call.
    let result = unsafe { ffi::git_tree_entry_cmp(first.as_ptr(), second.as_ptr()) };
    result.cmp(&0)
}

/// Wraps: git_tree_entry_filemode
/// Returns the normalized file mode of an entry.
#[must_use]
pub fn git_tree_entry_filemode(entry: GitTreeEntryRef<'_>) -> GitFileMode {
    // SAFETY: `entry` is a live shared borrow and the getter only reads it.
    let mode = unsafe { ffi::git_tree_entry_filemode(entry.as_ptr()) };
    GitFileMode::from_raw(mode).expect("a normalized tree entry has a published file mode")
}

/// Wraps: git_tree_entry_filemode_raw
/// Returns the unnormalized mode when it is a published file-mode value.
pub fn git_tree_entry_filemode_raw(
    entry: GitTreeEntryRef<'_>,
) -> Result<GitFileMode, ffi::git_filemode_t> {
    // SAFETY: `entry` is a live shared borrow and the getter only reads it.
    let mode = unsafe { ffi::git_tree_entry_filemode_raw(entry.as_ptr()) };
    GitFileMode::from_raw(mode).ok_or(mode)
}

/// Wraps: git_tree_entry_id
/// Borrows the entry's inline object ID.
#[must_use]
pub fn git_tree_entry_id<'a>(entry: GitTreeEntryRef<'a>) -> OidRef<'a> {
    // SAFETY: the live entry owns the non-null inline OID returned here.
    let oid = unsafe { ffi::git_tree_entry_id(entry.as_ptr()) }.cast_mut();
    // SAFETY: an inline field remains live for the entry handle's lifetime.
    unsafe { OidRef::from_ptr(oid) }.expect("a tree entry has an inline OID")
}

/// Wraps: git_tree_entry_name
/// Borrows the entry's filename.
#[must_use]
pub fn git_tree_entry_name<'a>(entry: GitTreeEntryRef<'a>) -> &'a CStr {
    // SAFETY: the live entry owns a non-null NUL-terminated filename.
    let name = unsafe { ffi::git_tree_entry_name(entry.as_ptr()) };
    // SAFETY: the filename remains live for the entry handle's lifetime.
    unsafe { CStr::from_ptr(name) }
}

/// Wraps: git_tree_entry_to_object
/// Looks up the object named by an entry and ties it to `repository`.
pub fn git_tree_entry_to_object<'repo>(
    repository: GitRepositoryRef<'repo>,
    entry: GitTreeEntryRef<'_>,
) -> Result<RepositoryObject<'repo>, i32> {
    let mut raw = core::ptr::null_mut();
    // SAFETY: the output slot is writable and both inputs are live. Success
    // produces one owned object reference backed by `repository`.
    let status = unsafe {
        ffi::git_tree_entry_to_object(
            core::ptr::addr_of_mut!(raw),
            repository.as_ptr().cast_mut(),
            entry.as_ptr(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success writes one non-null fully initialized owned object.
    let object = unsafe { GitObjectOwned::from_raw(raw) }.ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(RepositoryObject::from_owned(object, repository))
}

/// Wraps: git_tree_entry_type
/// Returns the kind of object named by an entry.
#[must_use]
pub fn git_tree_entry_type(entry: GitTreeEntryRef<'_>) -> GitObjectType {
    // SAFETY: the live shared entry is read only by this getter.
    let kind = unsafe { ffi::git_tree_entry_type(entry.as_ptr()) };
    GitObjectType::from_raw(kind).expect("a tree entry names a published object kind")
}

/// Wraps: git_treebuilder_clear
/// Removes and frees every entry in a builder.
pub fn git_treebuilder_clear(builder: &mut TreeBuilderMut<'_>) -> Result<(), i32> {
    // SAFETY: the builder is live and exclusively borrowed while its owned
    // entry map is cleared.
    let status = unsafe { ffi::git_treebuilder_clear(builder.as_mut_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_treebuilder_entrycount
/// Returns the number of entries in a builder.
#[must_use]
pub fn git_treebuilder_entrycount(builder: TreeBuilderRef<'_>) -> usize {
    // SAFETY: the C body only reads the live builder's map; restoring
    // mutability satisfies its historical declaration without writing.
    unsafe { ffi::git_treebuilder_entrycount(builder.as_ptr().cast_mut()) }
}

/// Wraps: git_treebuilder_get
/// Borrows the builder-owned entry named by `filename`.
#[must_use]
pub fn git_treebuilder_get<'a>(
    builder: TreeBuilderRef<'a>,
    filename: &CStr,
) -> Option<GitTreeEntryRef<'a>> {
    // SAFETY: both inputs are live for the call; a non-null result is owned by
    // the builder and remains live for its shared handle lifetime.
    let entry = unsafe { ffi::git_treebuilder_get(builder.as_ptr().cast_mut(), filename.as_ptr()) }
        .cast_mut();
    // SAFETY: the non-null result follows the builder-tied contract above.
    unsafe { GitTreeEntryRef::from_ptr(entry) }
}

/// Wraps: git_treebuilder_insert
/// Inserts or updates an entry and borrows the builder-owned result.
pub fn git_treebuilder_insert<'a>(
    builder: &'a mut TreeBuilderMut<'_>,
    filename: &CStr,
    id: OidRef<'_>,
    filemode: GitFileMode,
) -> Result<GitTreeEntryRef<'a>, i32> {
    let mut entry = core::ptr::null();
    // SAFETY: the output slot is writable; the builder is live and exclusive;
    // the filename and OID are live for the call and the mode is validated.
    let status = unsafe {
        ffi::git_treebuilder_insert(
            core::ptr::addr_of_mut!(entry),
            builder.as_mut_ptr(),
            filename.as_ptr(),
            id.as_ptr(),
            filemode.into(),
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success writes the non-null builder-owned entry, and the return
    // lifetime holds the exclusive builder reborrow against invalidation.
    unsafe { GitTreeEntryRef::from_ptr(entry.cast_mut()) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_treebuilder_remove
/// Removes and frees the builder entry named by `filename`.
pub fn git_treebuilder_remove(
    builder: &mut TreeBuilderMut<'_>,
    filename: &CStr,
) -> Result<(), i32> {
    // SAFETY: the builder is live and exclusive and the filename is a live C
    // string retained only for the lookup.
    let status = unsafe { ffi::git_treebuilder_remove(builder.as_mut_ptr(), filename.as_ptr()) };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_treebuilder_write
/// Writes the builder as a tree object and returns its object ID.
pub fn git_treebuilder_write(builder: &mut TreeBuilderMut<'_>) -> Result<Oid, i32> {
    let mut oid = Oid::zeroed();
    // SAFETY: `oid` is writable layout-compatible storage and the builder is
    // live and exclusively borrowed for its write cache and repository use.
    let status = unsafe {
        ffi::git_treebuilder_write(core::ptr::addr_of_mut!(oid).cast(), builder.as_mut_ptr())
    };
    if status == 0 { Ok(oid) } else { Err(status) }
}

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
        assert_eq!(
            size_of::<Option<TreeBuilderOwned>>(),
            size_of::<*mut ffi::git_treebuilder>()
        );
    }

    #[test]
    fn null_owning_seams_adopt_no_entry_or_builder() {
        // SAFETY: these conversions explicitly accept null and return `None`
        // without adopting an object or scheduling a destructor call.
        unsafe {
            assert!(GitTreeEntryOwned::from_raw(ptr::null_mut()).is_none());
            assert!(TreeBuilderOwned::from_raw(ptr::null_mut()).is_none());
            assert!(GitTreeEntryRef::from_ptr(ptr::null_mut()).is_none());
            assert!(GitTreeEntryMut::from_ptr(ptr::null_mut()).is_none());
            assert!(TreeBuilderRef::from_ptr(ptr::null_mut()).is_none());
            assert!(TreeBuilderMut::from_ptr(ptr::null_mut()).is_none());
        }
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

    #[test]
    fn tree_update_representation_and_handles_match_the_c_seam() {
        assert_eq!(size_of::<TreeUpdate>(), size_of::<ffi::git_tree_update>());
        assert_eq!(align_of::<TreeUpdate>(), align_of::<ffi::git_tree_update>());
        assert_eq!(
            size_of::<TreeUpdateRef<'static>>(),
            size_of::<*const ffi::git_tree_update>()
        );
        assert_eq!(
            size_of::<TreeUpdateMut<'static>>(),
            size_of::<*mut ffi::git_tree_update>()
        );
    }

    #[test]
    fn tree_update_handles_access_and_replace_every_field() {
        let mut raw = ffi::git_tree_update {
            action: ffi::git_tree_update_t_GIT_TREE_UPDATE_UPSERT,
            id: ffi::git_oid {
                type_: ffi::git_oid_t_GIT_OID_SHA1 as u8,
                id: [7; 32],
            },
            filemode: ffi::git_filemode_t_GIT_FILEMODE_BLOB,
            path: c"src/old.rs".as_ptr(),
        };

        {
            // SAFETY: `raw` is a live initialized descriptor, this scope has
            // exclusive access, and both path pointers have static storage.
            let mut update = unsafe { TreeUpdateMut::from_ptr(&raw mut raw) }
                .expect("the address of a stack value is non-null");

            assert_eq!(update.as_ref().action(), Ok(TreeUpdateType::Upsert));
            assert_eq!(update.as_ref().path(), c"src/old.rs");
            assert_eq!(update.as_ref().filemode(), Some(GitFileMode::BLOB));
            assert_eq!(
                update.as_ref().id().oid_type(),
                Ok(crate::oid::OidType::Sha1)
            );
            let mut bytes = [0; 32];
            assert!(update.as_ref().id().raw_bytes().copy_to_slice(&mut bytes));
            assert_eq!(bytes, [7; 32]);

            update.set_action(TreeUpdateType::Remove);
            // SAFETY: this C string literal has static storage and therefore
            // outlives every future observation of `raw`.
            unsafe { update.set_borrowed_path(c"src/new.rs") };
            update.set_filemode(GitFileMode::LINK);
            update.id_mut().set_oid_type(crate::oid::OidType::Sha256);
        }

        // SAFETY: `raw` remains live and initialized, no exclusive handle
        // exists, and its path points at static storage.
        let update = unsafe { TreeUpdateRef::from_ptr(&raw mut raw) }
            .expect("the address of a stack value is non-null");
        assert_eq!(update.action(), Ok(TreeUpdateType::Remove));
        assert_eq!(update.path(), c"src/new.rs");
        assert_eq!(update.filemode(), Some(GitFileMode::LINK));
        assert_eq!(update.id().oid_type(), Ok(crate::oid::OidType::Sha256));
    }
}

/// Wraps: git_tree_create_updated
/// Writes a tree formed by applying a contiguous run of updates to an optional
/// baseline tree.
pub fn git_tree_create_updated(
    repository: GitRepositoryRef<'_>,
    baseline: Option<GitTreeRef<'_>>,
    updates: CSlice<'_, TreeUpdate>,
) -> Result<Oid, i32> {
    let mut oid = Oid::zeroed();
    let baseline = baseline.map_or(core::ptr::null_mut(), |tree| tree.as_ptr().cast_mut());
    // SAFETY: output storage is writable, the repository and optional tree
    // are live, and `updates` describes its full initialized contiguous run.
    // Libgit2 retains none of these pointers after returning.
    let status = unsafe {
        ffi::git_tree_create_updated(
            addr_of_mut!(oid).cast(),
            repository.as_ptr().cast_mut(),
            baseline,
            updates.len(),
            updates.as_ptr().cast_const(),
        )
    };
    if status == 0 { Ok(oid) } else { Err(status) }
}

/// Wraps: git_tree_entry_byid
/// Borrows the tree-owned entry whose object ID matches `id`.
#[must_use]
pub fn git_tree_entry_byid<'tree>(
    tree: GitTreeRef<'tree>,
    id: OidRef<'_>,
) -> Option<GitTreeEntryRef<'tree>> {
    // SAFETY: both inputs are live shared handles; a non-null result points
    // into storage owned by `tree` and remains valid for its borrow.
    let entry = unsafe { ffi::git_tree_entry_byid(tree.as_ptr(), id.as_ptr()) }.cast_mut();
    // SAFETY: the result follows the tree-tied contract above.
    unsafe { GitTreeEntryRef::from_ptr(entry) }
}

/// Wraps: git_tree_entry_byindex
/// Borrows the tree-owned entry at `index`, or returns `None` when out of
/// range.
#[must_use]
pub fn git_tree_entry_byindex(tree: GitTreeRef<'_>, index: usize) -> Option<GitTreeEntryRef<'_>> {
    // SAFETY: `tree` is live and the C getter bounds-checks `index`; a
    // non-null result is owned by the tree.
    let entry = unsafe { ffi::git_tree_entry_byindex(tree.as_ptr(), index) }.cast_mut();
    // SAFETY: the result remains live for the tree handle's lifetime.
    unsafe { GitTreeEntryRef::from_ptr(entry) }
}

/// Wraps: git_tree_entry_byname
/// Borrows the tree-owned entry named by `filename`.
#[must_use]
pub fn git_tree_entry_byname<'tree>(
    tree: GitTreeRef<'tree>,
    filename: &CStr,
) -> Option<GitTreeEntryRef<'tree>> {
    // SAFETY: both inputs are live and libgit2 retains neither filename nor a
    // new owner; a non-null result points into `tree`.
    let entry = unsafe { ffi::git_tree_entry_byname(tree.as_ptr(), filename.as_ptr()) }.cast_mut();
    // SAFETY: the result remains live for the tree handle's lifetime.
    unsafe { GitTreeEntryRef::from_ptr(entry) }
}

/// Wraps: git_tree_entry_bypath
/// Looks up a relative path and returns a caller-owned deep copy of its entry.
pub fn git_tree_entry_bypath(root: GitTreeRef<'_>, path: &CStr) -> Result<GitTreeEntryOwned, i32> {
    let mut entry = core::ptr::null_mut();
    // SAFETY: the output slot is writable and both inputs are live. Success
    // writes one independently owned entry allocation.
    let status =
        unsafe { ffi::git_tree_entry_bypath(addr_of_mut!(entry), root.as_ptr(), path.as_ptr()) };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success writes a complete non-null entry owned by the caller.
    unsafe { GitTreeEntryOwned::from_raw(entry) }.ok_or(ffi::git_error_code_GIT_ERROR)
}

/// Wraps: git_tree_entrycount
/// Returns the number of entries directly contained in a tree.
#[must_use]
pub fn git_tree_entrycount(tree: GitTreeRef<'_>) -> usize {
    // SAFETY: the live shared tree is only read by this scalar getter.
    unsafe { ffi::git_tree_entrycount(tree.as_ptr()) }
}

/// Wraps: git_tree_walk
/// Traverses a tree synchronously through a safe callback.
///
/// The callback's control results are mode-dependent: see
/// [`GitTreewalkCallback::call`]. A negative result stops the traversal and is
/// returned unchanged as the error.
pub fn git_tree_walk<C>(
    tree: GitTreeRef<'_>,
    mode: TreeWalkMode,
    callback: &mut C,
) -> Result<(), i32>
where
    C: GitTreewalkCallback,
{
    unsafe extern "C" fn trampoline<C: GitTreewalkCallback>(
        root: *const c_char,
        entry: *const ffi::git_tree_entry,
        payload: *mut c_void,
    ) -> i32 {
        if root.is_null() || entry.is_null() || payload.is_null() {
            return -1;
        }
        // SAFETY: the outer wrapper passes this exact callback pointer and C
        // invokes the trampoline only during the synchronous walk.
        let callback = unsafe { &mut *payload.cast::<C>() };
        // SAFETY: libgit2 supplies a live NUL-terminated path for this
        // invocation and retains ownership of its storage.
        let root = unsafe { CStr::from_ptr(root) };
        // SAFETY: libgit2 supplies a live tree-owned entry for this callback
        // invocation; the handle cannot escape the callback trait's lifetime.
        let entry = unsafe { GitTreeEntryRef::from_ptr(entry.cast_mut()) }
            .expect("the trampoline rejected null");
        callback.call(root, entry)
    }

    // SAFETY: all inputs remain live for this synchronous call; the
    // trampoline reconstructs the exact callback type from its payload.
    let status = unsafe {
        ffi::git_tree_walk(
            tree.as_ptr(),
            mode.into(),
            Some(trampoline::<C>),
            core::ptr::from_mut(callback).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_treebuilder_filter
/// Removes every builder entry selected by a synchronous callback.
pub fn git_treebuilder_filter<C>(
    builder: &mut TreeBuilderMut<'_>,
    callback: &mut C,
) -> Result<(), i32>
where
    C: GitTreebuilderFilterCallback,
{
    unsafe extern "C" fn trampoline<C: GitTreebuilderFilterCallback>(
        entry: *const ffi::git_tree_entry,
        payload: *mut c_void,
    ) -> i32 {
        if entry.is_null() || payload.is_null() {
            return 0;
        }
        // SAFETY: the outer wrapper passes this exact callback pointer and C
        // invokes it synchronously while the builder remains exclusive.
        let callback = unsafe { &mut *payload.cast::<C>() };
        // SAFETY: the builder owns this live entry for the invocation. The
        // callback's HRTB prevents the transient handle from escaping.
        let entry = unsafe { GitTreeEntryRef::from_ptr(entry.cast_mut()) }
            .expect("the trampoline rejected null");
        i32::from(callback.remove(entry))
    }

    // SAFETY: the builder and callback remain live and exclusive for this
    // synchronous call; the trampoline reconstructs the exact callback type.
    let status = unsafe {
        ffi::git_treebuilder_filter(
            builder.as_mut_ptr(),
            Some(trampoline::<C>),
            core::ptr::from_mut(callback).cast(),
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_treebuilder_new
/// Creates a builder tied to the repository pointer it retains, optionally
/// deep-copying entries from `source`.
pub fn git_treebuilder_new<'repo>(
    repository: GitRepositoryRef<'repo>,
    source: Option<GitTreeRef<'_>>,
) -> Result<RepositoryTreeBuilder<'repo>, i32> {
    let mut builder = core::ptr::null_mut();
    let source = source.map_or(core::ptr::null(), |tree| tree.as_ptr());
    // SAFETY: the output slot is writable, the repository remains live for
    // the returned wrapper's lifetime, and the optional source is null or
    // live while libgit2 copies its entries.
    let status = unsafe {
        ffi::git_treebuilder_new(
            addr_of_mut!(builder),
            repository.as_ptr().cast_mut(),
            source,
        )
    };
    if status != 0 {
        return Err(status);
    }
    // SAFETY: success writes one complete non-null caller-owned builder.
    let inner =
        unsafe { TreeBuilderOwned::from_raw(builder) }.ok_or(ffi::git_error_code_GIT_ERROR)?;
    Ok(RepositoryTreeBuilder {
        inner,
        _repository: PhantomData,
    })
}

#[cfg(test)]
mod callback_surface_tests {
    use core::mem::MaybeUninit;

    use super::*;

    #[test]
    fn walk_callback_preserves_path_entry_and_control_result() {
        let storage = Box::new(MaybeUninit::<ffi::git_tree_entry>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_tree_entry>();
        // SAFETY: `raw` addresses live aligned opaque entry storage for the
        // callback invocation.
        let entry = unsafe { GitTreeEntryRef::from_ptr(raw) }.unwrap();
        let mut callback = |root: &CStr, seen: GitTreeEntryRef<'_>| {
            assert_eq!(root, c"src/");
            assert_eq!(seen.as_ptr(), raw.cast_const());
            -7
        };
        assert_eq!(GitTreewalkCallback::call(&mut callback, c"src/", entry), -7);
        // SAFETY: no handle remains in use and this recovers the allocation's
        // original type from `Box::into_raw`.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_tree_entry>>()) });
    }

    #[test]
    fn builder_filter_callback_preserves_its_boolean_decision() {
        let storage = Box::new(MaybeUninit::<ffi::git_tree_entry>::zeroed());
        let raw = Box::into_raw(storage).cast::<ffi::git_tree_entry>();
        // SAFETY: `raw` addresses live aligned opaque entry storage for the
        // callback invocation.
        let entry = unsafe { GitTreeEntryRef::from_ptr(raw) }.unwrap();
        let mut callback = |seen: GitTreeEntryRef<'_>| {
            assert_eq!(seen.as_ptr(), raw.cast_const());
            true
        };
        assert!(GitTreebuilderFilterCallback::remove(&mut callback, entry));
        // SAFETY: no handle remains in use and this recovers the allocation's
        // original type from `Box::into_raw`.
        drop(unsafe { Box::from_raw(raw.cast::<MaybeUninit<ffi::git_tree_entry>>()) });
    }
}

#[cfg(test)]
mod live_repository_tests {
    use core::ptr::addr_of_mut;

    use super::*;
    use crate::object_api::{git_tree_id, git_tree_lookup};
    use crate::oid::{Oid, git_oid_equal, git_oid_fromstrn};
    use crate::repository::git_repository_open;

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
            // SAFETY: balances the successful initialization represented by
            // this guard; every repository opened under it is dropped first.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    /// A hand-built bare repository this test writes tree objects into.
    struct BareRepo(std::path::PathBuf);

    impl BareRepo {
        fn create(tag: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("crustify-tree-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(path.join("objects/info")).expect("a loose-object directory");
            std::fs::create_dir_all(path.join("objects/pack")).expect("a pack directory");
            std::fs::create_dir_all(path.join("refs/heads")).expect("a refs directory");
            std::fs::write(path.join("HEAD"), b"ref: refs/heads/main\n").expect("a HEAD file");
            std::fs::write(
                path.join("config"),
                b"[core]\n\trepositoryformatversion = 0\n\tbare = true\n",
            )
            .expect("a config file");
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

    /// A gitlink object ID. `check_entry` skips the object-database existence
    /// check for `GIT_FILEMODE_COMMIT`, so this tree can be built without ever
    /// writing a commit.
    fn gitlink_id() -> Oid {
        git_oid_fromstrn(b"1111111111111111111111111111111111111111").expect("a valid hex ID")
    }

    /// Writes `refs/heads`-free tree objects: a `sub` tree holding one gitlink
    /// named `link`, and a root tree holding that subtree plus a `top` gitlink.
    fn write_trees(repository: &crate::repository::GitRepositoryOwned) -> Oid {
        let mut link = gitlink_id();
        // SAFETY: `link` is a live initialized `git_oid` on this frame and
        // outlives the shared handle borrowed from it below.
        let link = unsafe { OidRef::from_ptr(addr_of_mut!(link).cast()) }
            .expect("the address of a stack value is non-null");

        let mut sub = git_treebuilder_new(repository.as_ref(), None).expect("a subtree builder");
        let mut sub_oid = {
            let mut handle = sub.as_mut();
            let inserted = git_treebuilder_insert(&mut handle, c"link", link, GitFileMode::COMMIT)
                .expect("the gitlink entry is accepted");
            assert_eq!(git_tree_entry_name(inserted), c"link");
            git_treebuilder_write(&mut handle).expect("the subtree is written")
        };
        // SAFETY: `sub_oid` is a live initialized `git_oid` on this frame and
        // outlives the shared handle borrowed from it below.
        let sub_oid = unsafe { OidRef::from_ptr(addr_of_mut!(sub_oid).cast()) }
            .expect("the address of a stack value is non-null");

        let mut root = git_treebuilder_new(repository.as_ref(), None).expect("a root builder");
        let mut handle = root.as_mut();
        git_treebuilder_insert(&mut handle, c"sub", sub_oid, GitFileMode::TREE)
            .expect("the written subtree is a valid tree entry");
        git_treebuilder_insert(&mut handle, c"top", link, GitFileMode::COMMIT)
            .expect("the gitlink entry is accepted");
        git_treebuilder_write(&mut handle).expect("the root tree is written")
    }

    #[test]
    fn tree_entry_getters_borrow_from_the_tree_that_owns_them() {
        let _init = Libgit2Init::acquire();
        let dir = BareRepo::create("entry-getters");
        let repository = git_repository_open(&dir.c_path()).expect("the bare repository opens");

        let mut root_oid = write_trees(&repository);
        // SAFETY: `root_oid` is live and initialized for the rest of this test.
        let root_oid = unsafe { OidRef::from_ptr(addr_of_mut!(root_oid).cast()) }
            .expect("the address of a stack value is non-null");

        let owned = git_tree_lookup(repository.as_ref(), root_oid).expect("the root tree loads");
        let tree = owned.as_ref();

        assert_eq!(git_tree_entrycount(tree), 2);
        assert!(git_oid_equal(git_tree_id(tree), root_oid));

        // Git sorts a tree entry as if its name carried a trailing slash, so
        // `sub/` precedes `top`.
        let names: Vec<_> = (0..git_tree_entrycount(tree))
            .map(|index| {
                let entry = git_tree_entry_byindex(tree, index).expect("an in-range entry");
                git_tree_entry_name(entry).to_owned()
            })
            .collect();
        assert_eq!(names, [c"sub".to_owned(), c"top".to_owned()]);
        assert!(
            git_tree_entry_byindex(tree, 2).is_none(),
            "the C getter bounds-checks the index rather than reading past the array"
        );

        let by_name = git_tree_entry_byname(tree, c"sub").expect("the named entry exists");
        assert_eq!(git_tree_entry_filemode(by_name), GitFileMode::TREE);
        assert!(git_tree_entry_byname(tree, c"absent").is_none());

        // Every getter hands back the same tree-owned storage, which is what
        // ties `GitTreeEntryRef` to the tree borrow rather than to an owner.
        let by_index = git_tree_entry_byindex(tree, 0).expect("the first entry");
        assert_eq!(by_index.as_ptr(), by_name.as_ptr());
        let by_id = git_tree_entry_byid(tree, git_tree_entry_id(by_name)).expect("a matching ID");
        assert_eq!(by_id.as_ptr(), by_name.as_ptr());

        let mut absent =
            git_oid_fromstrn(b"9999999999999999999999999999999999999999").expect("a valid hex ID");
        // SAFETY: `absent` is a live initialized `git_oid` on this frame.
        let absent = unsafe { OidRef::from_ptr(addr_of_mut!(absent).cast()) }
            .expect("the address of a stack value is non-null");
        assert!(git_tree_entry_byid(tree, absent).is_none());
    }

    #[test]
    fn bypath_returns_an_entry_that_outlives_the_tree_it_was_found_in() {
        let _init = Libgit2Init::acquire();
        let dir = BareRepo::create("bypath");
        let repository = git_repository_open(&dir.c_path()).expect("the bare repository opens");

        let mut root_oid = write_trees(&repository);
        // SAFETY: `root_oid` is live and initialized for the rest of this test.
        let root_oid = unsafe { OidRef::from_ptr(addr_of_mut!(root_oid).cast()) }
            .expect("the address of a stack value is non-null");

        let nested = {
            let owned =
                git_tree_lookup(repository.as_ref(), root_oid).expect("the root tree loads");
            let tree = owned.as_ref();
            assert_eq!(
                git_tree_entry_bypath(tree, c"sub/absent").unwrap_err(),
                ffi::git_error_code_GIT_ENOTFOUND
            );
            git_tree_entry_bypath(tree, c"sub/link").expect("the nested entry resolves")
        };

        // The tree the entry was found in is gone; `git_tree_entry_bypath`
        // returned a self-contained allocation, so this reads live storage and
        // the owning handle's drop is the only release of it.
        assert_eq!(git_tree_entry_name(nested.as_ref()), c"link");
        assert_eq!(
            git_tree_entry_filemode(nested.as_ref()),
            GitFileMode::COMMIT
        );

        let copy = nested.clone();
        assert_ne!(copy.as_ref().as_ptr(), nested.as_ref().as_ptr());
        assert_eq!(git_tree_entry_name(copy.as_ref()), c"link");
    }

    #[test]
    fn tree_walk_reports_the_relative_root_and_honours_its_control_results() {
        let _init = Libgit2Init::acquire();
        let dir = BareRepo::create("walk");
        let repository = git_repository_open(&dir.c_path()).expect("the bare repository opens");

        let mut root_oid = write_trees(&repository);
        // SAFETY: `root_oid` is live and initialized for the rest of this test.
        let root_oid = unsafe { OidRef::from_ptr(addr_of_mut!(root_oid).cast()) }
            .expect("the address of a stack value is non-null");
        let owned = git_tree_lookup(repository.as_ref(), root_oid).expect("the root tree loads");
        let tree = owned.as_ref();

        let mut visited = Vec::new();
        let mut record = |root: &CStr, entry: GitTreeEntryRef<'_>| {
            visited.push((root.to_owned(), git_tree_entry_name(entry).to_owned()));
            0
        };
        assert_eq!(git_tree_walk(tree, TreeWalkMode::Pre, &mut record), Ok(()));
        assert_eq!(
            visited,
            [
                (c"".to_owned(), c"sub".to_owned()),
                (c"sub/".to_owned(), c"link".to_owned()),
                (c"".to_owned(), c"top".to_owned()),
            ]
        );

        // A positive result skips the entry — including its children — but
        // only in pre-order.
        let mut pre = Vec::new();
        let mut skip_subtrees = |_: &CStr, entry: GitTreeEntryRef<'_>| {
            let name = git_tree_entry_name(entry).to_owned();
            let is_tree = git_tree_entry_filemode(entry) == GitFileMode::TREE;
            pre.push(name);
            i32::from(is_tree)
        };
        assert_eq!(
            git_tree_walk(tree, TreeWalkMode::Pre, &mut skip_subtrees),
            Ok(())
        );
        assert_eq!(pre, [c"sub".to_owned(), c"top".to_owned()]);

        let mut post = Vec::new();
        let mut always_skip = |_: &CStr, entry: GitTreeEntryRef<'_>| {
            post.push(git_tree_entry_name(entry).to_owned());
            1
        };
        assert_eq!(
            git_tree_walk(tree, TreeWalkMode::Post, &mut always_skip),
            Ok(())
        );
        assert_eq!(
            post,
            [c"link".to_owned(), c"sub".to_owned(), c"top".to_owned()],
            "post-order discards a positive result instead of skipping"
        );

        // A negative result stops the walk and reaches the caller unchanged.
        let mut seen = 0;
        let mut stop = |_: &CStr, _: GitTreeEntryRef<'_>| {
            seen += 1;
            -9
        };
        assert_eq!(git_tree_walk(tree, TreeWalkMode::Pre, &mut stop), Err(-9));
        assert_eq!(seen, 1);
    }

    #[test]
    fn a_builder_copies_its_source_tree_and_frees_the_entries_its_filter_drops() {
        let _init = Libgit2Init::acquire();
        let dir = BareRepo::create("builder");
        let repository = git_repository_open(&dir.c_path()).expect("the bare repository opens");

        let mut root_oid = write_trees(&repository);
        // SAFETY: `root_oid` is live and initialized for the rest of this test.
        let root_oid = unsafe { OidRef::from_ptr(addr_of_mut!(root_oid).cast()) }
            .expect("the address of a stack value is non-null");
        let owned = git_tree_lookup(repository.as_ref(), root_oid).expect("the root tree loads");

        let mut builder = git_treebuilder_new(repository.as_ref(), Some(owned.as_ref()))
            .expect("a builder seeded from the root tree");
        assert_eq!(git_treebuilder_entrycount(builder.as_ref()), 2);

        // The builder deep-copied the tree's entries: dropping the tree leaves
        // every builder entry readable, and the builder alone frees them.
        drop(owned);
        assert!(git_treebuilder_get(builder.as_ref(), c"sub").is_some());

        let mut removed = Vec::new();
        let mut drop_gitlinks = |entry: GitTreeEntryRef<'_>| {
            let is_gitlink = git_tree_entry_filemode(entry) == GitFileMode::COMMIT;
            if is_gitlink {
                removed.push(git_tree_entry_name(entry).to_owned());
            }
            is_gitlink
        };
        assert_eq!(
            git_treebuilder_filter(&mut builder.as_mut(), &mut drop_gitlinks),
            Ok(())
        );
        assert_eq!(removed, [c"top".to_owned()]);
        assert_eq!(git_treebuilder_entrycount(builder.as_ref()), 1);
        assert!(git_treebuilder_get(builder.as_ref(), c"top").is_none());
        assert!(git_treebuilder_get(builder.as_ref(), c"sub").is_some());

        assert_eq!(git_treebuilder_clear(&mut builder.as_mut()), Ok(()));
        assert_eq!(git_treebuilder_entrycount(builder.as_ref()), 0);
    }
}
