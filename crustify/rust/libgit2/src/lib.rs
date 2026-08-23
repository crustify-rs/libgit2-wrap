//! Safe Rust wrappers for libgit2.

/// Raw libgit2 bindings.
#[allow(unused_imports)]
pub mod ffi {
    pub use libgit2_sys::*;
}

pub mod util;

pub mod annotated_commit;
pub mod apply;
pub mod attr;
pub mod blame;
pub mod blob;
pub mod branch;
pub mod buf;
pub mod cache;
pub mod checkout;
pub mod clone;
pub mod commit;
pub mod config;
pub mod describe;
pub mod diff;
pub mod diff_parse;
pub mod diff_print;
pub mod diff_stats;
pub mod diff_tform;
pub mod graph;
pub mod ignore;
pub mod index;
pub mod indexer;
pub mod iterator;
pub mod libgit2;
pub mod mailmap;
pub mod merge;
pub mod merge_file;
pub mod message;
pub mod notes;
pub mod object;
pub mod odb;
pub mod oid;
pub mod oidarray;
pub mod pack;
pub mod pack_objects;
pub mod patch;
pub mod pathspec;
pub mod proxy;
pub mod push;
pub mod rebase;
pub mod refdb;
pub mod reflog;
pub mod refs;
pub mod refspec;
pub mod remote;
pub mod repository;
pub mod reset;
pub mod revparse;
pub mod revwalk;
pub mod settings;
pub mod stash;
pub mod status;
pub mod strarray;
pub mod submodule;
pub mod tag;
pub mod trace;
pub mod transaction;
pub mod transport;
pub mod tree;
pub mod worktree;

pub mod api;
pub mod sys;
pub mod transports;
