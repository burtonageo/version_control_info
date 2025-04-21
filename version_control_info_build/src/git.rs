use crate::{DetectedInfo, GitExtraInfo, Info, Source, SpecificInfo};
use std::{
    error::Error,
    io,
    path::Path,
    process::{Child, Command, Output, Stdio},
};

use crate::VersionControlDetection;

#[inline(always)]
pub(crate) fn has_git_folder<P: ?Sized + AsRef<Path>>(project_path: &P) -> io::Result<bool> {
    #[inline(never)]
    fn inner(project_path: &Path) -> io::Result<bool> {
        let mut git = git(project_path);
        let output = git.arg("status").spawn()?.wait_with_output()?;

        Ok(output.status.success() && !output.stdout.starts_with(b"fatal:"))
    }

    inner(project_path.as_ref())
}

impl VersionControlDetection {
    pub(crate) fn detect_git_directory(
        project_dir: &Path,
    ) -> Result<Self, Box<dyn Error + Send + Sync + 'static>> {
        fn handle_output(output: Output) -> Result<String, Box<dyn Error + Send + Sync + 'static>> {
            #[inline]
            fn user_io_error<E: Into<Box<dyn Error + Send + Sync + 'static>>>(
                error: E,
            ) -> io::Error {
                io::Error::new(io::ErrorKind::Other, error)
            }

            if !output.status.success() || output.stderr.starts_with(b"fatal:") {
                let msg = format!("git failed: {}", String::from_utf8_lossy(&output.stderr),);
                return Err(From::from(user_io_error(msg)));
            }

            let mut stdout = output.stdout;
            if stdout
                .last()
                .map(|ch| ch.is_ascii_whitespace())
                .unwrap_or_default()
            {
                stdout.pop(); // probably a trailing '\n', pop it
            }

            let mut result = String::from_utf8(stdout).map_err(From::from);
            if let Ok(ref mut s) = result {
                s.retain(|c| !c.is_whitespace());
            };
            result
        }

        let git_rev_parse = || {
            let mut cmnd = git(project_dir);
            cmnd.arg("rev-parse");
            cmnd
        };

        let hash = git_rev_parse().args(&["--verify", "HEAD"]).spawn()?;

        let branch = git_rev_parse()
            .args(&["--abbrev-ref", "--verify", "HEAD"])
            .spawn()?;

        #[inline]
        fn wait_for_child(child: Child) -> Result<String, Box<dyn Error + Send + Sync + 'static>> {
            child
                .wait_with_output()
                .map_err(From::from)
                .and_then(handle_output)
        }

        let tags = git(project_dir)
            .args(&["tag", "--points-at", "HEAD"])
            .spawn()?;

        let (commit_hash, branch, tags) = (
            wait_for_child(hash)?,
            wait_for_child(branch)?,
            wait_for_child(tags)?,
        );

        Ok(Self {
            detected: DetectedInfo::VersionControl(Info {
                specific: SpecificInfo::Git {
                    commit_hash,
                    extra: Some(GitExtraInfo {
                        branch,
                        tags: tags.lines().map(String::from).collect(),
                    }),
                },
                source: Source::Repository,
            }),
            project_dir: project_dir.to_owned(),
        })
    }
}

#[inline]
fn git<P: ?Sized + AsRef<Path>>(cwd: &P) -> Command {
    #[inline(never)]
    fn inner(cwd: &Path) -> Command {
        let mut cmnd = Command::new("git");
        cmnd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(cwd);
        cmnd
    }

    inner(cwd.as_ref())
}
