use cfg_if::cfg_if;
use std::{
    ffi::OsString,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    string::FromUtf8Error,
};

#[inline(always)]
pub(crate) fn rerun_cargo_if_changed<P: ?Sized + AsRef<Path>>(path: &P) -> io::Result<()> {
    #[inline(never)]
    fn inner(path: &Path) -> io::Result<()> {
        if path.is_file() {
            return writeln!(io::stdout(), "cargo:rerun-if-changed={}", path.display());
        }

        for entry in fs::read_dir(path)? {
            entry.and_then(|e| inner(&e.path()))?;
        }

        Ok(())
    }

    inner(path.as_ref())
}

pub(crate) fn locate_project() -> io::Result<PathBuf> {
    fn handle_cargo_locate_project_output(output: Output) -> io::Result<PathBuf> {
        #[inline]
        fn make_osstring(bytes: Vec<u8>) -> Result<OsString, FromUtf8Error> {
            cfg_if! {
                if #[cfg(unix)] {
                    use std::os::unix::ffi::OsStringExt;
                    Ok(OsString::from_vec(bytes))
                } else if #[cfg(target_os = "wasi")] {
                    use std::os::wasi::ffi::OsStringExt;
                    Ok(OsString::from_vec(bytes))
                } else {
                    String::from_utf8(bytes).map(OsString::from)
                }
            }
        }

        if !output.status.success() {
            let msg = format!(
                "cargo locate-project failed: {}",
                String::from_utf8_lossy(&output.stderr),
            );
            return Err(io::Error::new(io::ErrorKind::Other, msg));
        }

        let mut stdout = output.stdout;
        if stdout
            .last()
            .map(|ch| ch.is_ascii_whitespace())
            .unwrap_or_default()
        {
            stdout.pop(); // probably a trailing '\n', pop it
        }

        let mut project_root = make_osstring(stdout)
            .map(PathBuf::from)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        if !project_root.is_dir() {
            project_root.pop(); // pop the "Cargo.toml"
        }

        Ok(project_root)
    }

    let mut cmnd = Command::new(env!("CARGO"));
    cmnd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(&["locate-project", "--message-format=plain", "--workspace"])
        .spawn()?
        .wait_with_output()
        .and_then(handle_cargo_locate_project_output)
}
