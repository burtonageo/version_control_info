use std::error::Error;

use version_control_info_build::generate_version_control_info;

fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let vcs_info = version_control_info_build::detect()?;
    generate_version_control_info(&vcs_info)?;

    Ok(())
}
