#![deny(unsafe_code)]
#![warn(unused)]

use crate::cargo_vcs_info::CargoVcsInfo;
use git::has_git_folder;
use std::{
    cell::Cell,
    env,
    error::Error,
    ffi::OsStr,
    fs::{self, File},
    io::{self, Write, stdout},
    path::{Path, PathBuf},
};
use util::rerun_cargo_if_changed;

mod cargo_vcs_info;
mod git;
mod util;

#[derive(Debug)]
pub struct VersionControlDetection {
    detected: DetectedInfo,
    project_dir: PathBuf,
}

impl VersionControlDetection {
    #[inline]
    #[must_use]
    pub fn version_control_info(&self) -> Option<&Info> {
        match self.detected {
            DetectedInfo::VersionControl(ref info) => Some(info),
            _ => None,
        }
    }

    #[inline]
    #[must_use]
    pub fn project_dir(&self) -> &Path {
        self.project_dir.as_path()
    }
}

pub fn detect() -> Result<VersionControlDetection, Box<dyn Error + Send + Sync + 'static>> {
    writeln!(stdout(), "cargo::rustc-cfg=VERSION_CONTROL_INFO_BUILD")?;

    let project_dir = util::locate_project()?;

    // prefer using the git folder directly if available, as it is probably
    // more correct.
    if has_git_folder(&project_dir)? {
        return VersionControlDetection::detect_git_directory(&project_dir);
    }

    let vcs_info_file = project_dir.join(CargoVcsInfo::FILE_NAME);
    if vcs_info_file.exists() {
        let file = fs::File::open(&vcs_info_file).map(io::BufReader::new)?;
        let vcs_info: CargoVcsInfo = serde_json::from_reader(file)?;

        if let Some(ref git_info) = vcs_info.git {
            return Ok(VersionControlDetection {
                detected: DetectedInfo::VersionControl(Info {
                    specific: SpecificInfo::Git {
                        commit_hash: git_info.sha1.clone(),
                        extra: None,
                    },
                    source: Source::CargoVcsInfoFile,
                }),
                project_dir,
            });
        }
    }

    Ok(VersionControlDetection {
        detected: DetectedInfo::NotFound,
        project_dir,
    })
}

macro_rules! writeln_indented {
    ($indent:expr, $out:expr, $msg:literal $(,)?) => {
        writeln!($out, concat!("{__space:__indent$}", $msg), __space = ' ', __indent = $indent.indent.get() * 4)
    };
    ($indent:expr, $out:expr, $msg:literal, $($arg:tt)+ $(,)?) => {
        writeln!($out, concat!("{__space:__indent$}", $msg), $($arg)+, __space = ' ', __indent = $indent.indent.get() * 4)
    };
}

pub fn generate_redacted_version_control_info() -> io::Result<()> {
    let indent = Indenter::new(0);
    let indent = indent.auto_indent();

    {
        let mut bindings_file = create_get_vcs_info_file()?;

        write_header_comment(&mut bindings_file)?;

        writeln_indented!(indent, bindings_file, "const {{")?;
        {
            let _indent = indent.increment();
            writeln_indented!(indent, bindings_file, "compile_error!(concat!(")?;
            {
                let _indent = indent.increment();
                writeln_indented!(
                    indent,
                    bindings_file,
                    "\"version control info has been redacted. use the `try_get!()` macro\","
                )?;
                writeln_indented!(
                    indent,
                    bindings_file,
                    "\"to fallibly acces version control info.\","
                )?;
            }
            writeln_indented!(indent, bindings_file, "{:<4}))", ' ')?;
        }
        writeln_indented!(indent, bindings_file, "}}")?;

        bindings_file.flush()?;
    }

    {
        let mut bindings_file = create_try_get_vcs_info_file()?;

        writeln_indented!(indent, bindings_file, "const {{")?;
        {
            let _indent = indent.increment();
            writeln_indented!(indent, bindings_file, "::core::result::Result::<")?;
            {
                let _indent = indent.increment();
                writeln_indented!(indent, bindings_file, "version_control_info::Info<'_>,")?;
                writeln_indented!(indent, bindings_file, "version_control_info::Error,")?;
            }
            writeln_indented!(
                indent,
                bindings_file,
                ">::Err(version_control_info::Error::Redacted)",
            )?;
        }
        writeln_indented!(indent, bindings_file, "}}")?;

        bindings_file.flush()?;
    }

    Ok(())
}

pub fn generate_version_control_info(detection: &VersionControlDetection) -> io::Result<()> {
    fn generate_git_vcs_info(
        file: &mut dyn Write,
        commit: &str,
        extra: Option<&GitExtraInfo>,
        source: &Source,
        indent: &AutoIndent<'_>,
    ) -> io::Result<()> {
        writeln_indented!(indent, file, "version_control_info::Info {{")?;
        {
            let _indent = indent.increment();
            writeln_indented!(
                indent,
                file,
                "specific: version_control_info::SpecificInfo::Git {{"
            )?;
            {
                let _indent = indent.increment();
                writeln_indented!(indent, file, "commit_hash: \"{}\",", commit)?;
                match extra {
                    Some(extra) => {
                        writeln_indented!(
                            indent,
                            file,
                            "extra: Some(&version_control_info::GitExtraData {{"
                        )?;
                        {
                            let _indent = indent.increment();
                            writeln_indented!(indent, file, "branch: \"{}\",", extra.branch)?;
                            writeln_indented!(indent, file, "tags: &[")?;
                            {
                                let _indent = indent.increment();
                                for tag in &extra.tags {
                                    writeln_indented!(indent, file, "\"{}\",", tag)?;
                                }
                            }
                            writeln_indented!(indent, file, "],")?;
                        }
                        writeln_indented!(indent, file, "}}),")?;
                    }
                    None => {
                        writeln_indented!(indent, file, "extra: None,",)?;
                    }
                }
            }
            writeln_indented!(indent, file, "}},")?;

            let source = match *source {
                Source::Repository => "Repository",
                Source::CargoVcsInfoFile => "CargoVcsInfoFile",
            };
            writeln_indented!(
                indent,
                file,
                "source: version_control_info::Source::{},",
                source
            )?;
        }
        writeln_indented!(indent, file, "}}")?;
        Ok(())
    }

    fn generate_git_get(
        get_info_file: &mut dyn Write,
        commit: &str,
        extra: Option<&GitExtraInfo>,
        source: &Source,
    ) -> io::Result<()> {
        write_header_comment(get_info_file)?;
        let indent = Indenter::new(0);
        let indent = indent.auto_indent();

        writeln_indented!(indent, get_info_file, "const {{")?;
        {
            let _indent = indent.increment();
            generate_git_vcs_info(get_info_file, commit, extra, source, &indent)?;
        }
        writeln_indented!(indent, get_info_file, "}}")?;
        Ok(())
    }

    fn generate_git_try_get(
        try_get_info_file: &mut dyn Write,
        commit: &str,
        extra: Option<&GitExtraInfo>,
        source: &Source,
    ) -> io::Result<()> {
        write_header_comment(try_get_info_file)?;
        let indent = Indenter::new(0);
        let indent = indent.auto_indent();

        writeln_indented!(indent, try_get_info_file, "const {{")?;
        {
            let _indent = indent.increment();
            writeln_indented!(indent, try_get_info_file, "::core::result::Result::<")?;
            {
                let _indent = indent.increment();
                writeln_indented!(
                    indent,
                    try_get_info_file,
                    "version_control_info::Info<'_>,"
                )?;
                writeln_indented!(indent, try_get_info_file, "version_control_info::Error,")?;
            }
            writeln_indented!(indent, try_get_info_file, ">::Ok(")?;
            {
                let _indent = indent.increment();
                generate_git_vcs_info(try_get_info_file, commit, extra, source, &indent)?;
            }
            writeln_indented!(indent, try_get_info_file, ")")?;
        }
        writeln_indented!(indent, try_get_info_file, "}}")?;

        Ok(())
    }

    let mut get_info_file = create_get_vcs_info_file()?;
    let mut try_get_info_file = create_try_get_vcs_info_file()?;

    match detection.detected {
        DetectedInfo::NotFound => {
            write_header_comment(&mut get_info_file)?;
            writeln!(
                get_info_file,
                "{{ compile_error!(\"could not find version control info for {}\"); }}",
                detection.project_dir.display()
            )?;

            write_header_comment(&mut try_get_info_file)?;
            writeln!(try_get_info_file, "const {{")?;
            {
                writeln!(get_info_file, "{:<4}::core::result::Result::<", ' ')?;
                {
                    writeln!(get_info_file, "{:<8}version_control_info::Info<'_>,", ' ')?;
                    writeln!(get_info_file, "{:<8}version_control_info::Error,", ' ')?;
                }
                writeln!(
                    get_info_file,
                    "{:<4}>::Err(version_control_info::Error::NoVersionControl)",
                    ' '
                )?;
            }
            writeln!(try_get_info_file, "}}")?;
        }
        DetectedInfo::VersionControl(ref vcs_info) => match vcs_info.specific {
            SpecificInfo::Git {
                ref commit_hash,
                ref extra,
            } => {
                let vcs_info_path = {
                    let final_comp = match vcs_info.source {
                        Source::Repository => ".git",
                        Source::CargoVcsInfoFile => CargoVcsInfo::FILE_NAME,
                    };
                    detection.project_dir.join(final_comp)
                };
                rerun_cargo_if_changed(&vcs_info_path)?;
                generate_git_get(
                    &mut get_info_file,
                    &commit_hash,
                    extra.as_ref(),
                    &vcs_info.source,
                )?;
                generate_git_try_get(
                    &mut try_get_info_file,
                    &commit_hash,
                    extra.as_ref(),
                    &vcs_info.source,
                )?;
            }
        },
    }

    get_info_file.flush()?;
    try_get_info_file.flush()?;

    Ok(())
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum DetectedInfo {
    NotFound,
    VersionControl(Info),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Info {
    pub specific: SpecificInfo,
    pub source: Source,
}

impl Info {
    #[inline]
    #[must_use]
    pub fn commit(&self) -> &str {
        self.specific.commit()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Source {
    CargoVcsInfoFile,
    Repository,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SpecificInfo {
    Git {
        commit_hash: String,
        extra: Option<GitExtraInfo>,
    },
}

impl SpecificInfo {
    #[inline]
    pub fn commit(&self) -> &str {
        match *self {
            SpecificInfo::Git {
                ref commit_hash, ..
            } => &commit_hash,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GitExtraInfo {
    pub branch: String,
    pub tags: Vec<String>,
}

#[inline]
fn create_get_vcs_info_file() -> io::Result<io::BufWriter<File>> {
    create_bindings_file("version_control_info_get_generated")
}

#[inline]
fn create_try_get_vcs_info_file() -> io::Result<io::BufWriter<File>> {
    create_bindings_file("version_control_info_try_get_generated")
}

#[inline]
fn create_bindings_file<S: ?Sized + AsRef<OsStr>>(
    file_name: &S,
) -> io::Result<io::BufWriter<File>> {
    #[inline(never)]
    fn inner(file_name: &OsStr) -> io::Result<io::BufWriter<File>> {
        fn out_dir() -> io::Result<PathBuf> {
            env::var_os("OUT_DIR")
                .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "could not get out directory"))
                .map(PathBuf::from)
        }

        let out_dir = out_dir()?;
        fs::create_dir_all(&out_dir)?;

        let path = out_dir.join(file_name).with_extension("rs");

        File::create(&path).map(io::BufWriter::new)
    }

    inner(file_name.as_ref())
}

fn write_header_comment(file: &mut dyn Write) -> io::Result<()> {
    writeln!(
        file,
        "// This file has been generated by the `version_control_info` crate."
    )?;
    writeln!(file, "// Do not edit it manually.")?;
    writeln!(file)?;
    Ok(())
}

struct Indenter {
    indent: Cell<usize>,
}

impl Indenter {
    #[must_use]
    #[inline]
    fn new(initial_indent: usize) -> Self {
        Self {
            indent: Cell::new(initial_indent),
        }
    }

    fn auto_indent(&self) -> AutoIndent<'_> {
        AutoIndent {
            indent: &self.indent,
        }
    }
}

struct AutoIndent<'a> {
    indent: &'a Cell<usize>,
}

impl<'a> AutoIndent<'a> {
    #[inline]
    fn increment(&self) -> Self {
        self.indent.set(self.indent.get().saturating_add(1));
        Self {
            indent: self.indent,
        }
    }

    #[inline]
    fn decrement(&self) {
        self.indent.set(self.indent.get().saturating_sub(1));
    }
}

impl<'a> Drop for AutoIndent<'a> {
    fn drop(&mut self) {
        self.decrement();
    }
}
