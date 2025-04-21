# version_control_info

This crate provides a way to embed version control info into
a crate, and provide that to other crates which depend on your
crate.

## Examples

To use this crate, you must use the `version_control_info_build` crate
in a build script to generate the version control info:

```rust,ignore
// build.rs

use version_control_info_build::generate_version_control_info;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let info = version_control_info_build::detect()?;
    // You can access information from the info in your build script.
    let commit = info.version_control_info().commit();
    println!("{:?}", commit);

    generate_version_control_info(&info)?;
    Ok(())
}
```

Then, in your crate, you can use the [`get!()`] macro to get the generated
version control info as a constant:

```rust,ignore
// main.rs
use version_control_info::Info;
const VCS_INFO: Info<'_> = version_control_info::get!();

fn main() {
    println!("{:#?}", VCS_INFO);
}
```

The [`Info`] type contains the version control information which can be queried.

## Notes

At the moment, this crate only supports `git` and `mercurial` repositories. Feel free to
open a pull request to add support for other repositories.

Note that `cargo` has been built primarily with support for `git` - external repository
dependencies must be specified as `git` repositories, and only `git` commit info is
bundled with the crate when it is published using the `cargo package` command. Therefore,
this crate works best when using `git`. See the [`Source`] type documentation for further
details.

## Features

* `std`: Links to `std`. This feature is enabled by default.
* `serde`: Implements the [`Serialize`] and [`Deseiralize`] traits on types in this crate.
  This feature is disabled by default.

[`Serialize`]: https://docs.rs/serde/latest/serde/trait.Serialize.html
[`Deseiralize`]: https://docs.rs/serde/latest/serde/trait.Deserialize.html
