use serde::{Serialize, Deserialize};
use std::path::PathBuf;

/// Example file
///
/// ```json
/// {
///  "git": {
///    "sha1": "aac20b6e7e543e6dd4118b246c77225e3a3a1302",
///    "dirty": true
///  },
///  "path_in_vcs": ""
/// }
/// ```
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct CargoVcsInfo {
    #[serde(default)]
    pub(crate) git: Option<GitVcsInfo>,
    pub(crate) path_in_vcs: PathBuf,
}

impl CargoVcsInfo {
    pub(crate) const FILE_NAME: &'static str = ".cargo_vcs_info.json";
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct GitVcsInfo {
    pub(crate) sha1: String,
    pub(crate) dirty: bool,
}
