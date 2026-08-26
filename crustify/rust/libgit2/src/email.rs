//! Safe wrappers for libgit2 email APIs.

use core::ffi::CStr;

use crate::api::buffer::GitBufMut;
use crate::api::email::{GitEmailCreateOptionsMut, GitEmailCreateOptionsRef};
use crate::api::types::GitSignatureRef;
use crate::commit::GitCommitMut;
use crate::diff::DiffMut;
use crate::ffi;
use crate::oid::OidRef;

/// Wraps: git_email_create_from_commit
/// Replaces `out` with an mbox-formatted patch for `commit`.
///
/// The commit is borrowed exclusively: the header is built from
/// `git_commit_summary` and `git_commit_body`, which fill `commit->summary`
/// and `commit->body` on first use, and the patch body is a generated diff
/// that this call then formats, writing the delta flags and object IDs the
/// generated diff records.
///
/// The repository is not an argument. C derives it with
/// `git_commit_owner(commit)` and generates the diff through it, so this call
/// also performs the lazy repository writes every generated-diff constructor
/// does — the configuration snapshot and the index re-read. No Rust handle
/// borrows that repository here, so a caller holding one concurrently must
/// keep that in view.
pub fn git_email_create_from_commit(
    out: &mut GitBufMut<'_>,
    commit: &mut GitCommitMut<'_>,
    options: Option<GitEmailCreateOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: `out` and `commit` are both exclusive for the writes this call
    // performs through them, and the optional options stay live and read-only
    // for the synchronous formatting operation.
    let status = unsafe {
        ffi::git_email_create_from_commit(out.as_mut_ptr(), commit.as_mut_ptr(), options)
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_email_create_from_diff
/// Replaces `out` with one mbox-formatted patch from `diff`.
///
/// The diff is borrowed exclusively. `append_diffstat` calls
/// `git_diff_get_stats` and `append_patches` calls `git_patch_from_diff` once
/// per delta; both generate patches, which writes the attribute session and
/// the per-delta file flags, sizes and object IDs held inside the diff. The
/// wrappers for those two functions already take the exclusive handle.
#[allow(clippy::too_many_arguments)]
pub fn git_email_create_from_diff(
    out: &mut GitBufMut<'_>,
    diff: &mut DiffMut<'_>,
    patch_index: usize,
    patch_count: usize,
    commit_id: OidRef<'_>,
    summary: &CStr,
    body: Option<&CStr>,
    author: GitSignatureRef<'_>,
    options: Option<GitEmailCreateOptionsRef<'_, '_>>,
) -> Result<(), i32> {
    let options = options.map_or(core::ptr::null(), |options| options.as_ptr());
    // SAFETY: `out` and `diff` are exclusive for the writes this call performs
    // through them; all other pointers come from live shared handles or C
    // strings and are retained only for this call.
    let status = unsafe {
        ffi::git_email_create_from_diff(
            out.as_mut_ptr(),
            diff.as_mut_ptr(),
            patch_index,
            patch_count,
            commit_id.as_ptr(),
            summary.as_ptr(),
            body.map_or(core::ptr::null(), CStr::as_ptr),
            author.as_ptr(),
            options,
        )
    };
    if status == 0 { Ok(()) } else { Err(status) }
}

/// Wraps: git_email_create_options_init
/// Initializes email-creation options for `version`.
pub fn git_email_create_options_init(
    options: &mut GitEmailCreateOptionsMut<'_, '_>,
    version: core::ffi::c_uint,
) -> Result<(), i32> {
    // SAFETY: the exclusive handle supplies writable layout-compatible
    // storage, and the initializer retains no pointer into it.
    let status = unsafe { ffi::git_email_create_options_init(options.as_mut_ptr(), version) };
    if status == 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod scheduled_email_tests {
    use super::*;
    use crate::api::buffer::GitBuf;
    use crate::diff_generate::git_diff_tree_to_tree;
    use crate::oid::Oid;
    use crate::repository::git_repository_open_bare;
    use crate::signature::git_signature_new;

    /// The smallest on-disk bare repository `git_repository_open_bare`
    /// accepts, so a generated diff exists without a working tree.
    struct BareRepo(std::path::PathBuf);

    impl BareRepo {
        fn create(tag: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("crustify-email-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(path.join("objects")).expect("a private temporary directory");
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

    #[test]
    fn formatting_a_diff_takes_it_exclusively_and_fills_the_buffer() {
        // SAFETY: process-global initialization is refcounted and balanced
        // after every owner created here has been released.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let directory = BareRepo::create("from-diff");
        let mut owner = git_repository_open_bare(&directory.c_path())
            .expect("the hand-built directory is a bare repository");
        {
            let mut repository = owner.as_mut();
            // The diff has to be handed over exclusively: `append_diffstat`
            // and `append_patches` generate patches from it, which writes its
            // attribute session and its per-delta file records.
            let mut diff = git_diff_tree_to_tree(&mut repository, None, None, None)
                .expect("two absent trees still diff");
            let author = git_signature_new(c"A U Thor", c"author@example.com", 1_000_000_000, 0)
                .expect("a well-formed signature");
            let id = Oid::zeroed();
            let raw = core::ptr::addr_of!(id).cast::<ffi::git_oid>().cast_mut();
            // SAFETY: `raw` addresses the live layout-compatible local OID for
            // the duration of the call below.
            let id = unsafe { OidRef::from_ptr(raw) }.expect("a local address is non-null");

            let mut buffer = GitBuf::new();
            git_email_create_from_diff(
                &mut buffer.as_mut(),
                &mut diff.as_mut(),
                1,
                1,
                id,
                c"a summary",
                None,
                author.as_ref(),
                None,
            )
            .expect("an empty diff still formats an mbox patch");

            let written = buffer.as_ref();
            let contents = written.contents().expect("a written buffer");
            let bytes: std::vec::Vec<u8> = contents.elems().collect();
            let text = std::string::String::from_utf8(bytes).expect("mbox output is ASCII");
            assert!(text.starts_with("From "), "unexpected mbox output: {text}");
            assert!(text.contains("Subject: [PATCH] a summary"));
            assert!(text.contains("author@example.com"));
        }
        drop(owner);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}

#[cfg(test)]
mod scheduled_initializer_tests {
    use super::*;
    use crate::api::email::GitEmailCreateOptions;

    #[test]
    fn the_published_initializer_restores_every_constructed_default() {
        // `GIT_EMAIL_CREATE_OPTIONS_INIT` spells its nested diff header out
        // by hand -- binary hunks shown, submodules unspecified and three
        // lines of context -- and the C initializer copies all of it over the
        // caller's storage, so reinitializing deliberately dirtied options
        // pins `GitEmailCreateOptions::new` to that template.
        let mut options = GitEmailCreateOptions::new();
        {
            let mut view = options.as_mut();
            view.set_version(0);
            view.set_flags(crate::api::email::GitEmailCreateFlags::OMIT_NUMBERS);
            view.set_reroll_number(4);
            view.set_start_number(2);
            view.set_subject_prefix(Some(c"RFC"));
            view.diff_options_mut().set_version(0);
            view.diff_options_mut().set_context_lines(9);
            view.diff_options_mut()
                .set_flags(crate::api::diff::DiffOptions::IGNORE_WHITESPACE);
            view.diff_find_options_mut().set_version(0);
            view.diff_find_options_mut()
                .set_flags(crate::api::diff::GitDiffFindFlags::COPIES);
        }

        git_email_create_options_init(&mut options.as_mut(), ffi::GIT_EMAIL_CREATE_OPTIONS_VERSION)
            .unwrap();

        let view = options.as_ref();
        assert_eq!(view.version(), ffi::GIT_EMAIL_CREATE_OPTIONS_VERSION);
        assert_eq!(
            view.flags(),
            Ok(crate::api::email::GitEmailCreateFlags::DEFAULT)
        );
        assert_eq!(view.reroll_number(), 0);
        assert_eq!(view.start_number(), 0);
        assert_eq!(view.subject_prefix(), None);
        assert_eq!(view.diff_options().version(), ffi::GIT_DIFF_OPTIONS_VERSION);
        assert_eq!(
            view.diff_options().flags(),
            Ok(crate::api::diff::DiffOptions::SHOW_BINARY)
        );
        assert_eq!(view.diff_options().context_lines(), 3);
        assert_eq!(
            view.diff_find_options().version(),
            ffi::GIT_DIFF_FIND_OPTIONS_VERSION
        );
        assert!(
            view.diff_find_options()
                .flags()
                .expect("a published rename-detection bit set")
                .is_empty()
        );
    }

    #[test]
    fn an_unsupported_version_is_rejected_without_writing() {
        // SAFETY: process-global initialization is reference counted and is
        // balanced below; the rejected version reaches `git_error_set`, which
        // allocates through the allocator only initialization installs.
        assert!(unsafe { ffi::git_libgit2_init() } > 0);
        let mut options = GitEmailCreateOptions::new();
        options.as_mut().set_reroll_number(4);
        assert!(git_email_create_options_init(&mut options.as_mut(), 0).is_err());
        assert_eq!(options.as_ref().reroll_number(), 4);
        // SAFETY: balances this test's successful initialization call.
        assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
    }
}
