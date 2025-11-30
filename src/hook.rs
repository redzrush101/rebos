use std::io;
use crate::library;
use piglog::prelude::*;

pub fn run(hook_name: &str) -> Result<(), io::Error> {
    let hook_path = crate::places::base_user().join("hooks").join(hook_name);

    if hook_path.exists() {
        crate::info!("Running hook: {}", hook_name);

        match library::run_command(&hook_path.display().to_string()) {
            true => crate::info!("Successfully ran hook: {}", hook_name),
            false => {
                crate::error!("Failed to run hook: {}", hook_name);
                return Err(library::custom_error("Failed to run hook!"));
            }
        }
    }

    Ok(())
}
