//! Safe wrappers for libgit2 attr APIs.

use core::ffi::CStr;
use core::ptr::{NonNull, addr_of, addr_of_mut};

use ffibox::{CVal, CValued};

use crate::attr::Attribute;
use crate::ffi;
use crate::oid::{OidMut, OidRef};

/// Wraps: git_attr_foreach_cb
/// Safe callable surface for one transient attribute assignment.
pub trait GitAttrForeachCallback {
    /// Receives an attribute name and its classified value.
    ///
    /// Both borrows are valid only for this invocation. Returning nonzero
    /// stops iteration and propagates that value to the caller.
    fn call<'a>(&mut self, name: &'a CStr, value: Attribute<'a>) -> i32;
}

impl<F> GitAttrForeachCallback for F
where
    F: for<'a> FnMut(&'a CStr, Attribute<'a>) -> i32,
{
    fn call<'a>(&mut self, name: &'a CStr, value: Attribute<'a>) -> i32 {
        self(name, value)
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn closure_implements_attribute_callback() {
        let mut seen = |name: &CStr, value: Attribute<'_>| {
            i32::from(name == c"binary" && value == Attribute::True)
        };
        assert_eq!(
            GitAttrForeachCallback::call(&mut seen, c"binary", Attribute::True),
            1
        );
    }
}

ffibox::define_ctype!(
    /// Wraps: git_attr_options
    /// Layout-compatible options controlling attribute lookup.
    GitAttrOptions,
    GitAttrOptionsRef,
    GitAttrOptionsMut,
    ffi::git_attr_options
);

// SAFETY: attribute options only borrow an optional commit ID and own no
// resource, so disposing their inline storage requires no action.
unsafe impl CValued for GitAttrOptions {
    unsafe fn c_dispose(_this: NonNull<Self>) {}
}

impl GitAttrOptions {
    /// Constructs options equivalent to `GIT_ATTR_OPTIONS_INIT`.
    #[must_use]
    pub fn new() -> CVal<Self> {
        let mut options = CVal::new(Self::zeroed());
        options.as_mut().set_version(ffi::GIT_ATTR_OPTIONS_VERSION);
        options
    }
}

impl<'a> GitAttrOptionsRef<'a> {
    /// Field: git_attr_options.flags
    /// Returns the raw combination of `GIT_ATTR_CHECK_*` bits.
    #[must_use]
    pub fn flags(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).flags).read() }
    }

    /// Field: git_attr_options.version
    /// Returns the options ABI version.
    #[must_use]
    pub fn version(&self) -> core::ffi::c_uint {
        // SAFETY: this live shared handle permits a raw-place scalar read.
        unsafe { addr_of!((*self.as_ptr()).version).read() }
    }

    /// Field: git_attr_options.attr_commit_id
    /// Borrows the inline commit ID used by the current API.
    #[must_use]
    pub fn attr_commit_id(&self) -> OidRef<'a> {
        // SAFETY: raw-place projection reaches the live inline field without
        // forming a reference to the C-visible options object.
        let id = unsafe { addr_of!((*self.as_ptr()).attr_commit_id) }.cast_mut();
        // SAFETY: an inline field is non-null and remains live for the options
        // handle's borrow.
        unsafe { OidRef::from_ptr(id) }.expect("an inline commit ID is non-null")
    }

    /// Field: git_attr_options.commit_id
    /// Borrows the deprecated optional commit-ID pointer.
    #[must_use]
    pub fn commit_id(&self) -> Option<OidRef<'a>> {
        // SAFETY: this live shared handle permits the pointer-field read.
        let id = unsafe { addr_of!((*self.as_ptr()).commit_id).read() };
        // SAFETY: a non-null field is a borrowed ID that must remain live for
        // every use of the enclosing options value.
        unsafe { OidRef::from_ptr(id) }
    }

    /// Field: git_attr_options.reserved
    /// Reports whether the conditional reserved ABI slot is non-null.
    ///
    /// With deprecated APIs enabled, bindgen names this layout-compatible slot
    /// `commit_id`; a hard-deprecation build names the same pointer slot
    /// `reserved` and requires it to remain null.
    #[must_use]
    pub fn has_reserved_value(&self) -> bool {
        // SAFETY: the conditional fields occupy the same pointer-sized ABI
        // slot, and this shared handle permits reading its initialized value.
        !unsafe { addr_of!((*self.as_ptr()).commit_id).read() }.is_null()
    }
}

impl GitAttrOptionsMut<'_> {
    /// Replaces the raw combination of `GIT_ATTR_CHECK_*` bits.
    pub fn set_flags(&mut self, flags: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).flags).write(flags) }
    }

    /// Replaces the options ABI version.
    pub fn set_version(&mut self, version: core::ffi::c_uint) {
        // SAFETY: this exclusive handle permits the scalar write.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).version).write(version) }
    }

    /// Borrows the current inline commit ID exclusively.
    #[must_use]
    pub fn attr_commit_id_mut(&mut self) -> OidMut<'_> {
        // SAFETY: raw-place projection reaches the inline field through this
        // exclusive reborrow, and its address is non-null.
        let id = unsafe { addr_of_mut!((*self.as_mut_ptr()).attr_commit_id) };
        // SAFETY: this exclusive options reborrow uniquely covers the inline
        // field for the returned handle's lifetime.
        unsafe { OidMut::from_ptr(id) }.expect("an inline commit ID is non-null")
    }

    /// Stores a deprecated borrowed commit ID.
    ///
    /// # Safety
    ///
    /// A non-null `id` must remain live for every later use of the underlying
    /// options value, including uses after this mutable reborrow ends.
    pub unsafe fn set_borrowed_commit_id(&mut self, id: Option<OidRef<'_>>) {
        let id = id.map_or(core::ptr::null_mut(), |id| id.as_ptr().cast_mut());
        // SAFETY: this exclusive handle permits the pointer write; the caller
        // supplies the stored referent's unexpressible lifetime.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).commit_id).write(id) }
    }

    /// Clears the conditional reserved/deprecated pointer slot.
    pub fn clear_reserved_value(&mut self) {
        // SAFETY: the conditional fields share this ABI slot, and null is the
        // required reserved value as well as the deprecated default.
        unsafe { addr_of_mut!((*self.as_mut_ptr()).commit_id).write(core::ptr::null_mut()) }
    }
}

#[cfg(test)]
mod attr_options_tests {
    use core::mem::{align_of, size_of};

    use ffibox::{CCell, CValued};

    use super::*;

    #[test]
    fn options_preserve_layout_and_defaults() {
        fn assert_cell<T: CCell>() {}
        fn assert_valued<T: CValued>() {}

        assert_cell::<GitAttrOptions>();
        assert_valued::<GitAttrOptions>();
        assert_eq!(
            size_of::<GitAttrOptions>(),
            size_of::<ffi::git_attr_options>()
        );
        assert_eq!(
            align_of::<GitAttrOptions>(),
            align_of::<ffi::git_attr_options>()
        );

        let options = GitAttrOptions::new();
        assert_eq!(options.as_ref().version(), ffi::GIT_ATTR_OPTIONS_VERSION);
        assert_eq!(options.as_ref().flags(), 0);
        assert!(options.as_ref().commit_id().is_none());
        assert!(!options.as_ref().has_reserved_value());
    }

    #[test]
    fn scalar_and_inline_fields_are_mutable() {
        let mut options = GitAttrOptions::new();
        options.as_mut().set_flags(17);
        options
            .as_mut()
            .attr_commit_id_mut()
            .set_oid_type(crate::oid::OidType::Sha1);
        assert_eq!(options.as_ref().flags(), 17);
        assert_eq!(
            options.as_ref().attr_commit_id().oid_type(),
            Ok(crate::oid::OidType::Sha1)
        );
    }
}

#[cfg(test)]
mod attr_callback_traversal_tests {
    use super::*;

    use crate::attr::{git_attr_foreach, git_attr_foreach_ext};
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
            // SAFETY: balances this guard's initialization; every repository
            // opened under it is released first.
            assert!(unsafe { ffi::git_libgit2_shutdown() } >= 0);
        }
    }

    /// A hand-built repository whose working directory carries the one
    /// `.gitattributes` file the traversal reads. Attribute lookup consults
    /// the working directory directly, so no object or index entry is needed.
    struct WorkdirRepo(std::path::PathBuf);

    impl WorkdirRepo {
        fn create(tag: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("crustify-attr-cb-{}-{tag}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            let git = path.join(".git");
            std::fs::create_dir_all(git.join("objects")).expect("a loose-object directory");
            std::fs::create_dir_all(git.join("refs/heads")).expect("a refs directory");
            std::fs::write(git.join("HEAD"), b"ref: refs/heads/main\n").expect("a HEAD file");
            std::fs::write(
                git.join("config"),
                b"[core]\n\trepositoryformatversion = 0\n\tbare = false\n",
            )
            .expect("a config file");
            // One assignment of each classified shape: a string value, a set
            // flag, an unset flag and an explicitly unspecified one.
            std::fs::write(
                path.join(".gitattributes"),
                b"report.txt eol=lf text -merge !diff\n",
            )
            .expect("an attributes file");
            Self(path)
        }

        fn c_path(&self) -> std::ffi::CString {
            std::ffi::CString::new(self.0.to_str().expect("a UTF-8 temporary path"))
                .expect("a path without interior NUL")
        }
    }

    impl Drop for WorkdirRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn traversal_classifies_every_value_shape_for_the_callback() {
        let _init = Libgit2Init::acquire();
        let directory = WorkdirRepo::create("visit");
        let mut owner = git_repository_open(&directory.c_path())
            .expect("the hand-built directory is a repository");

        let mut seen = Vec::new();
        let mut collect = |name: &CStr, value: Attribute<'_>| {
            // Both borrows are call-scoped, so retaining them needs copies.
            let value = match value {
                Attribute::Unspecified => "unspecified".to_owned(),
                Attribute::True => "true".to_owned(),
                Attribute::False => "false".to_owned(),
                Attribute::String(value) => format!("string:{}", value.to_string_lossy()),
            };
            seen.push((name.to_owned(), value));
            0
        };
        git_attr_foreach(&mut owner.as_mut(), 0, c"report.txt", &mut collect)
            .expect("visiting the hand-built attributes");

        seen.sort();
        assert_eq!(
            seen,
            [
                (c"diff".to_owned(), "unspecified".to_owned()),
                (c"eol".to_owned(), "string:lf".to_owned()),
                (c"merge".to_owned(), "false".to_owned()),
                (c"text".to_owned(), "true".to_owned()),
            ]
        );
    }

    #[test]
    fn a_nonzero_callback_result_stops_the_traversal_and_propagates() {
        let _init = Libgit2Init::acquire();
        let directory = WorkdirRepo::create("stop");
        let mut owner = git_repository_open(&directory.c_path())
            .expect("the hand-built directory is a repository");

        let mut visits = 0;
        let mut stop = |_: &CStr, _: Attribute<'_>| {
            visits += 1;
            -37
        };
        assert_eq!(
            git_attr_foreach(&mut owner.as_mut(), 0, c"report.txt", &mut stop),
            Err(-37)
        );
        assert_eq!(visits, 1);

        // The extended entry point drives the same callback surface.
        let options = GitAttrOptions::new();
        let mut extended_visits = 0;
        let mut count = |_: &CStr, _: Attribute<'_>| {
            extended_visits += 1;
            0
        };
        git_attr_foreach_ext(
            &mut owner.as_mut(),
            options.as_ref(),
            c"report.txt",
            &mut count,
        )
        .expect("visiting the hand-built attributes through the options entry point");
        assert_eq!(extended_visits, 4);
    }
}
