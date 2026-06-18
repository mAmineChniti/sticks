use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemPackageManager {
	Apt,
	Dnf,
	Pacman,
	Apk,
	Zypper,
	Unknown,
}

/// Detects the system package manager by probing common commands
pub fn detect_system_package_manager() -> SystemPackageManager {
	if command_exists("apt") || command_exists("apt-get") {
		SystemPackageManager::Apt
	} else if command_exists("dnf") {
		SystemPackageManager::Dnf
	} else if command_exists("pacman") {
		SystemPackageManager::Pacman
	} else if command_exists("apk") {
		SystemPackageManager::Apk
	} else if command_exists("zypper") {
		SystemPackageManager::Zypper
	} else {
		SystemPackageManager::Unknown
	}
}

fn command_exists(cmd: &str) -> bool {
	Command::new(cmd)
		.arg("--version")
		.status()
		.map(|s| s.success())
		.unwrap_or(false)
}

/// Returns the preferred install command prefix for the system package manager
/// e.g. "sudo apt install -y"
pub fn install_command_prefix() -> String {
	match detect_system_package_manager() {
		SystemPackageManager::Apt => "sudo apt install -y".to_string(),
		SystemPackageManager::Dnf => "sudo dnf install -y".to_string(),
		SystemPackageManager::Pacman => "sudo pacman -S --noconfirm".to_string(),
		SystemPackageManager::Apk => "sudo apk add".to_string(),
		SystemPackageManager::Zypper => "sudo zypper install -y".to_string(),
		SystemPackageManager::Unknown => "sudo apt install -y".to_string(),
	}
}
