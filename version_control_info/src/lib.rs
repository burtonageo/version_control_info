#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code, missing_docs)]

//! # version_control_info
//!
//! This crate provides a way to embed version control info into
//! a crate, and provide that to other crates which depend on your
//! crate.
//!
//! ## Examples
//!
//! To use this crate, you must use the `version_control_info_build` crate
//! in a build script to generate the version control info:
//!
//! ```rust
//! // build.rs
//!
//! use version_control_info_build::generate_version_control_info;
//! use std::error::Error;
//!
//! fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
//!     let info = version_control_info_build::detect()?;
//!     // You can access information from the info in your build script.
//!     match info.version_control_info().map(|info| info.commit()) {
//!         Some(commit) => println!("{:?}", commit),
//!         None => println!("Could not get commit info"),
//!     }
//!
//!     # // don't actually generate vcs info in the test
//!     # if false {
//!     // Then, generate a vcs info file.
//!     generate_version_control_info(&info)?;
//!     # }
//!     Ok(())
//! }
//! ```
//!
//! Then, in your crate, you can use the [`get!()`] macro to get the generated
//! version control info as a constant:
//!
//! ```rust,ignore
//! // main.rs
//! use version_control_info::Info;
//! const VCS_INFO: Info<'_> = version_control_info::get!();
//!
//! fn main() {
//!     println!("{:#?}", VCS_INFO);
//! }
//! ```
//!
//! The [`Info`] type contains the version control information which can be queried.
//!
//! ## Notes
//!
//! At the moment, this crate only supports `git` and `mercurial` repositories. Feel free to
//! open a pull request to add support for other repositories.
//!
//! Note that `cargo` has been built primarily with support for `git` - external repository
//! dependencies must be specified as `git` repositories, and only `git` commit info is
//! bundled with the crate when it is published using the `cargo package` command. Therefore,
//! this crate works best when using `git`. See the [`Source`] type documentation for further
//! details.
//!
//! ## Features
//!
//! * `std`: Links to `std`. This feature is enabled by default.
//! * `serde`: Implements the [`Serialize`] and [`Deseiralize`] traits on types in this crate.
//!   This feature is disabled by default.
//!
//! [`Serialize`]: https://docs.rs/serde/latest/serde/trait.Serialize.html
//! [`Deseiralize`]: https://docs.rs/serde/latest/serde/trait.Deserialize.html

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
extern crate std as core;

use core::{error::Error as ErrorTrait, fmt};

/// Represents version control info for a crate.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct Info<'a> {
    /// Contains specific information about the version control system.
    pub specific: SpecificInfo<'a>,
    /// Records the source from which this `Info` was generated.
    ///
    /// See the defintion of [`Source`] for more details.
    pub source: Source,
}

impl<'a> Info<'a> {
    /// Returns the full remote commit hash string from the `SpecificInfo` for the
    /// current commit which this crate was built from.
    #[inline]
    #[must_use]
    #[doc(alias = "revision")]
    pub const fn commit(&self) -> &str {
        self.specific.commit()
    }

    /// Returns the list of tags associated with the current commit.
    ///
    /// * Returns `None` if the tag information could not be found.
    /// * Returns `Some(&[])` if there are no tags associated with the commit.
    #[inline]
    #[must_use]
    pub const fn tags(&self) -> Option<&[&str]> {
        self.specific.tags()
    }
}

/// Contains information which is specific to a version control program.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum SpecificInfo<'a> {
    /// Contains information about a git repository.
    Git {
        /// The full commit hash.
        commit_hash: &'a str,
        /// Extra metadata about the git repository, if available.
        ///
        /// See the definition of [`GitExtraData`] for more details.
        extra: Option<&'a git::ExtraData<'a>>,
    },
    /// Contains information about a Mercurial repository.
    Mercurial {
        /// Global revision number.
        global_revision: &'a str,
        /// Extra metadata about the Mercurial repository.
        ///
        /// See the definition of [`MercurialExtraData`] for more details.
        extra: Option<&'a mercurial::ExtraData<'a>>,
    },
}

impl<'a> SpecificInfo<'a> {
    /// Returns the full remote commit hash string from the `SpecificInfo` for the
    /// current commit which this crate was built from.
    #[inline]
    #[must_use]
    #[doc(alias = "revision")]
    pub const fn commit(&self) -> &str {
        match *self {
            Self::Git { commit_hash, .. } => commit_hash,
            Self::Mercurial {
                global_revision, ..
            } => global_revision,
        }
    }

    /// Returns the list of tags associated with the current commit.
    ///
    /// * Returns `None` if the tag information could not be found.
    /// * Returns `Some(&[])` if there are no tags associated with the commit.
    #[inline]
    #[must_use]
    pub const fn tags(&self) -> Option<&[&str]> {
        match *self {
            Self::Git { extra, .. } => match extra {
                Some(extra) => Some(extra.tags),
                None => None,
            },
            Self::Mercurial { extra, .. } => match extra {
                Some(extra) => Some(extra.tags),
                None => None,
            },
        }
    }
}

/// Module containing types and functionality specific to git repositories.
pub mod git {
    /// Contains extra data about the git repository.
    ///
    /// # Note
    ///
    /// This will only be present if the dependency is specified as a git
    /// dependency when used in the `Cargo.toml` file. When a dependency
    /// is downloaded from [`crates.io`](https://crates.io), the git
    /// history and repository metadata is not downloaded, and so this
    /// information will not be available.
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    #[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
    pub struct ExtraData<'a> {
        /// The name of the branch.
        pub branch: &'a str,
        /// Tags associated with the current commit.
        pub tags: &'a [&'a str],
    }
}

#[doc(inline)]
#[deprecated]
pub use git::ExtraData as GitExtraData;

/// Module containing types and functionality specific to mercurial repositories.
pub mod mercurial {
    /// Contains extra data about the Mercurial repository.
    ///
    /// # Notes
    ///
    /// At the moment, this will never be available when building the dependency
    /// from `crates.io`.
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    #[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
    pub struct ExtraData<'a> {
        /// Local revision number.
        pub local_revision: &'a str,
        /// The branch of the current revision.
        pub branch: &'a str,
        /// The list of tags for the current revision.
        pub tags: &'a [&'a str],
        /// The list of bookmarks for the current revision.
        pub bookmarks: &'a [&'a str],
    }
}

#[doc(inline)]
#[deprecated]
pub use mercurial::ExtraData as MercurialExtraData;

/// The source from which the version control information was read.
#[non_exhaustive]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub enum Source {
    /// The version control info was read directly from the repository data.
    Repository,
    /// The version control information was read from the `.cargo_vcs_info.json` file.
    ///
    /// This is only present in packages published to `crates.io`. It is also only
    /// present in repositories which use `git`, and is unreliable, as there is no
    /// guarantee that this value corresponds to an actual commit in the repository.
    ///
    /// If the `source` is set to this value, then the version control info
    /// should be seen as potentially unreliable.
    ///
    /// For more info, see [The Cargo Book][book].
    ///
    /// [book]: https://doc.rust-lang.org/cargo/commands/cargo-package.html#cargo_vcs_infojson-format.
    CargoVcsInfoFile,
}

impl fmt::Debug for Source {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        let source = match *self {
            Self::CargoVcsInfoFile => ".cargo_vcs_info.json",
            Self::Repository => "Repository",
        };
        fmtr.write_str(source)
    }
}

/// An error representing that no version control information was found.
#[non_exhaustive]
pub enum Error {
    /// There was no version control information found in the project directory.
    NoVersionControl,
    /// The version control information was explicitly redacted in the build script.
    Redacted,
    /// An uncategorised error occurred.
    Other {
        /// The message associated with the error.
        reason: &'static str,
    },
}

impl fmt::Debug for Error {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NoVersionControl => fmtr.debug_struct("NoVersionControl").finish(),
            Self::Redacted => fmtr.debug_struct("Redacted").finish(),
            Self::Other { ref reason } => {
                fmtr.debug_struct("Other").field("reason", reason).finish()
            }
        }
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match *self {
            Self::NoVersionControl => "version control not found",
            Self::Redacted => "version control information is redacted",
            Self::Other { reason } => reason,
        };
        fmtr.write_str(message)
    }
}

impl ErrorTrait for Error {}

/// Retrieves the version control info.
///
/// If the `version_control_info_build::detect()` function has not been run in a build
/// script, this macro will fail.
///
/// If the build-stage vcs detection has failed, then this will result in a compile error.
/// If you need to handle failures gracefully, use the [`try_get!()`] macro.
///
/// # Example
/// 
/// ```rust,ignore
/// # fn main() {
/// use version_control_info::Info;
/// const INFO: version_control_info::Info<'_> = version_control_info::get!();
/// println!("commit = {}", INFO.commit());
/// # }
/// ```
#[macro_export]
macro_rules! get {
    () => {
        include!(concat!(
            env!("OUT_DIR"),
            "/version_control_info_get_generated.rs"
        ))
    };
}

/// Attempt to retrieve the version control info.
///
/// If the `version_control_info_build::detect()` function has not been run in a build
/// script, this macro will fail.
///
/// # Example
/// 
/// ```rust,ignore
/// # fn main() {
/// use version_control_info::{Info, Error};
/// const MAYBE_INFO: Result<Info<'_>, Error> = version_control_info::try_get!();
/// match MAYBE_INFO.as_ref() {
///     Ok(info) => println!("version control commit = {}", info.commit()),
///     Err(e) => println!("could not get vcs info: {}", e),
/// }
/// # }
/// ```
#[macro_export]
macro_rules! try_get {
    () => {
        include!(concat!(
            env!("OUT_DIR"),
            "/version_control_info_try_get_generated.rs"
        ))
    };
}
