//! Shared support for raw-C versus safe-wrapper I/O equivalence tests.

#![allow(clippy::undocumented_unsafe_blocks)]

use crate::ffi;

use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Holds one process-global libgit2 initialization count.
pub(crate) struct Libgit2Init;

impl Libgit2Init {
    pub(crate) fn acquire() -> Self {
        // SAFETY: libgit2 initialization is process-global and refcounted;
        // `Drop` balances this successful acquisition.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        Self
    }
}

impl Drop for Libgit2Init {
    fn drop(&mut self) {
        // SAFETY: balances the acquisition represented by this guard. Tests
        // declare it before every raw or safe owner, so those owners drop first.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}

/// Caller-owned raw `git_buf` header with automatic field disposal.
pub(crate) struct RawBuf(pub(crate) ffi::git_buf);

impl RawBuf {
    pub(crate) fn new() -> Self {
        Self(ffi::git_buf {
            ptr: core::ptr::null_mut(),
            reserved: 0,
            size: 0,
        })
    }

    pub(crate) fn bytes(&self) -> Vec<u8> {
        if self.0.size == 0 {
            return Vec::new();
        }
        assert!(!self.0.ptr.is_null());
        // SAFETY: a successful libgit2 buffer-producing call initializes
        // exactly `size` content bytes at its non-null pointer.
        unsafe { core::slice::from_raw_parts(self.0.ptr.cast::<u8>(), self.0.size) }.to_vec()
    }
}

impl Drop for RawBuf {
    fn drop(&mut self) {
        // SAFETY: this is a valid caller-owned header and disposal releases
        // only its fields. It is called exactly once.
        unsafe { ffi::git_buf_dispose(&mut self.0) }
    }
}

pub(crate) fn safe_buf_bytes(buffer: crate::api::buffer::GitBufRef<'_>) -> Vec<u8> {
    buffer
        .contents()
        .map(|contents| contents.elems().collect())
        .unwrap_or_default()
}

/// Raw owned diff used by the reference leg of differential tests.
pub(crate) struct RawDiff(*mut ffi::git_diff);

impl RawDiff {
    pub(crate) fn from_buffer(content: &[u8]) -> Result<Self, i32> {
        let mut output = core::ptr::null_mut();
        let input = if content.is_empty() {
            b"".as_ptr()
        } else {
            content.as_ptr()
        };
        // SAFETY: `content` supplies the counted input bytes, and `output` is
        // a writable owner slot. The function retains no input pointer.
        let status = unsafe { ffi::git_diff_from_buffer(&mut output, input.cast(), content.len()) };
        if status != 0 {
            return Err(status);
        }
        if output.is_null() {
            return Err(ffi::git_error_code_GIT_ERROR);
        }
        Ok(Self(output))
    }

    pub(crate) fn as_ptr(&self) -> *mut ffi::git_diff {
        self.0
    }
}

impl Drop for RawDiff {
    fn drop(&mut self) {
        // SAFETY: successful construction transfers one complete diff owner;
        // this is its only release.
        unsafe { ffi::git_diff_free(self.0) }
    }
}

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

/// A uniquely named temporary directory removed when the test finishes.
pub(crate) struct TempDir(PathBuf);

impl TempDir {
    pub(crate) fn new(label: &str) -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "crustify-io-equiv-{label}-{}-{sequence}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create I/O-equivalence fixture directory");
        Self(path)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }

    pub(crate) fn c_path(&self) -> CString {
        CString::new(self.0.to_str().expect("temporary path is UTF-8"))
            .expect("temporary path has no NUL")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Raw repository owner used to prepare identical fixtures for both legs.
pub(crate) struct RawRepository(*mut ffi::git_repository);

impl RawRepository {
    pub(crate) fn init(path: &std::ffi::CStr, bare: bool) -> Result<Self, i32> {
        let mut output = core::ptr::null_mut();
        // SAFETY: the output slot and path are live; success transfers one
        // repository owner into this guard.
        let status =
            unsafe { ffi::git_repository_init(&mut output, path.as_ptr(), u32::from(bare)) };
        if status != 0 {
            return Err(status);
        }
        assert!(!output.is_null());
        Ok(Self(output))
    }

    pub(crate) fn as_ptr(&self) -> *mut ffi::git_repository {
        self.0
    }
}

impl Drop for RawRepository {
    fn drop(&mut self) {
        // SAFETY: successful initialization transferred this sole owner.
        unsafe { ffi::git_repository_free(self.0) }
    }
}

/// A small deterministic repository history used by end-to-end comparisons.
///
/// Fixture construction deliberately goes through the raw API. This keeps the
/// setup identical for both legs and leaves each test free to compare one raw
/// observation with its safe-wrapper equivalent.
pub(crate) struct HistoryFixture {
    pub(crate) repository: RawRepository,
    pub(crate) directory: TempDir,
}

impl HistoryFixture {
    pub(crate) fn new(label: &str) -> Self {
        let directory = TempDir::new(label);
        let path = directory.c_path();
        let repository = RawRepository::init(&path, false).expect("initialize history fixture");

        std::fs::create_dir_all(directory.path().join("src")).unwrap();
        std::fs::create_dir_all(directory.path().join("docs")).unwrap();
        std::fs::write(directory.path().join("README.md"), b"fixture\n").unwrap();
        std::fs::write(
            directory.path().join("src/alpha.c"),
            b"int alpha(void) { return 1; }\n",
        )
        .unwrap();
        std::fs::write(directory.path().join("docs/guide.txt"), b"first guide\n").unwrap();
        let first = raw_commit(
            repository.as_ptr(),
            &[c"README.md", c"src/alpha.c", c"docs/guide.txt"],
            &[],
            c"initial fixture commit",
            None,
        );

        std::fs::write(
            directory.path().join("src/alpha.c"),
            b"int alpha(void) { return 2; }\n",
        )
        .unwrap();
        std::fs::write(
            directory.path().join("src/beta.c"),
            b"int beta(int x) { return x * x; }\n",
        )
        .unwrap();
        std::fs::remove_file(directory.path().join("docs/guide.txt")).unwrap();
        let second = raw_commit(
            repository.as_ptr(),
            &[c"src/alpha.c", c"src/beta.c"],
            &[c"docs/guide.txt"],
            c"second fixture commit",
            Some(&first),
        );

        std::fs::write(
            directory.path().join("README.md"),
            b"fixture\nwith a third revision\n",
        )
        .unwrap();
        let third = raw_commit(
            repository.as_ptr(),
            &[c"README.md"],
            &[],
            c"third fixture commit",
            Some(&second),
        );

        // Populate common reference shapes for refs, revwalk, tag and remote
        // tests. All identifiers remain deterministic across twin fixtures.
        let mut head_commit = core::ptr::null_mut();
        // SAFETY: the repository and ID are live and the output is writable.
        assert_eq!(
            unsafe { ffi::git_commit_lookup(&mut head_commit, repository.as_ptr(), &third) },
            0
        );
        let mut branch = core::ptr::null_mut();
        // SAFETY: all pointers are live and success transfers an owned ref.
        assert_eq!(
            unsafe {
                ffi::git_branch_create(
                    &mut branch,
                    repository.as_ptr(),
                    c"topic".as_ptr(),
                    head_commit,
                    0,
                )
            },
            0
        );
        // SAFETY: successful creation transferred this reference owner.
        unsafe { ffi::git_reference_free(branch) };

        let mut tag_id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
        let mut tagger = core::ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::git_signature_new(
                    &mut tagger,
                    c"Crustify".as_ptr(),
                    c"crustify@example.com".as_ptr(),
                    1_700_000_100,
                    0,
                )
            },
            0
        );
        // SAFETY: all arguments are live and the output is writable.
        assert_eq!(
            unsafe {
                ffi::git_tag_create(
                    &mut tag_id,
                    repository.as_ptr(),
                    c"v1.0".as_ptr(),
                    head_commit.cast(),
                    tagger,
                    c"fixture release".as_ptr(),
                    0,
                )
            },
            0
        );
        unsafe { ffi::git_signature_free(tagger) };
        // SAFETY: successful lookup transferred this commit owner.
        unsafe { ffi::git_commit_free(head_commit) };

        Self {
            repository,
            directory,
        }
    }
}

fn raw_commit(
    repository: *mut ffi::git_repository,
    additions: &[&std::ffi::CStr],
    removals: &[&std::ffi::CStr],
    message: &std::ffi::CStr,
    parent_id: Option<&ffi::git_oid>,
) -> ffi::git_oid {
    let mut index = core::ptr::null_mut();
    // SAFETY: repository is live and output is writable.
    assert_eq!(
        unsafe { ffi::git_repository_index(&mut index, repository) },
        0
    );
    for path in additions {
        // SAFETY: index and path are live for the synchronous call.
        assert_eq!(
            unsafe { ffi::git_index_add_bypath(index, path.as_ptr()) },
            0
        );
    }
    for path in removals {
        // SAFETY: index and path are live for the synchronous call.
        assert_eq!(
            unsafe { ffi::git_index_remove_bypath(index, path.as_ptr()) },
            0
        );
    }
    // SAFETY: index is a live exclusive owner.
    assert_eq!(unsafe { ffi::git_index_write(index) }, 0);
    let mut tree_id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
    // SAFETY: output and index are live.
    assert_eq!(unsafe { ffi::git_index_write_tree(&mut tree_id, index) }, 0);
    // SAFETY: index owner is no longer used.
    unsafe { ffi::git_index_free(index) };

    let mut tree = core::ptr::null_mut();
    // SAFETY: repository, ID and output are live.
    assert_eq!(
        unsafe { ffi::git_tree_lookup(&mut tree, repository, &tree_id) },
        0
    );
    let mut signature = core::ptr::null_mut();
    // SAFETY: output and static C strings are live.
    assert_eq!(
        unsafe {
            ffi::git_signature_new(
                &mut signature,
                c"Crustify".as_ptr(),
                c"crustify@example.com".as_ptr(),
                1_700_000_000,
                0,
            )
        },
        0
    );

    let mut parent = core::ptr::null_mut();
    let mut parents: [*const ffi::git_commit; 1] = [core::ptr::null()];
    let parent_count = if let Some(parent_id) = parent_id {
        // SAFETY: repository, parent ID and output are live.
        assert_eq!(
            unsafe { ffi::git_commit_lookup(&mut parent, repository, parent_id) },
            0
        );
        parents[0] = parent;
        1
    } else {
        0
    };
    let mut commit_id = unsafe { core::mem::zeroed::<ffi::git_oid>() };
    // SAFETY: all pointed-to values are live for this synchronous creation.
    assert_eq!(
        unsafe {
            ffi::git_commit_create(
                &mut commit_id,
                repository,
                c"HEAD".as_ptr(),
                signature,
                signature,
                core::ptr::null(),
                message.as_ptr(),
                tree,
                parent_count,
                parents.as_mut_ptr(),
            )
        },
        0
    );
    if !parent.is_null() {
        // SAFETY: lookup transferred this owner.
        unsafe { ffi::git_commit_free(parent) };
    }
    // SAFETY: constructors transferred these owners.
    unsafe {
        ffi::git_signature_free(signature);
        ffi::git_tree_free(tree);
    }
    commit_id
}
