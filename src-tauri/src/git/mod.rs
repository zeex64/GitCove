//! Git engine: everything runs through the user's `git` CLI.

pub mod diff;
pub mod error;
pub mod log;
pub mod operation;
pub mod oplog;
pub mod ops;
pub mod preview;
pub mod refs;
pub mod repo;
pub mod runner;
pub mod snapshot;
pub mod status;
pub mod types;

pub use error::{GitError, GitErrorKind, GitResult};
