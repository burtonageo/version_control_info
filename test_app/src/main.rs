use version_control_info;

fn main() {
    let maybe_vcs_info = version_control_info::try_get!();
    match maybe_vcs_info.as_ref() {
        Ok(vcs_info) => {
            println!("I am definitely on commit: {:.8}", vcs_info.commit());
        }
        Err(e) => {
            println!("Could not get commit info: {}", e);
        }
    };

    let vcs_info = version_control_info::get!();
    println!("I am definitely on commit {:.8}, or this would be a compile error", vcs_info.commit());
}
