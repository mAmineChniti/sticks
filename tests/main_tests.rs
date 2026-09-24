use std::process::Command;
use tempfile::TempDir;

fn sticks_binary() -> &'static str {
	env!("CARGO_BIN_EXE_sticks")
}

#[test]
fn test_cli_c_command() {
	let temp_dir = TempDir::new().unwrap();
	let output = Command::new(sticks_binary())
		.args(["c", "test_project"])
		.current_dir(temp_dir.path())
		.output()
		.expect("Failed to execute command");

	assert!(
		output.status.success(),
		"CLI failed: {}",
		String::from_utf8_lossy(&output.stderr)
	);
	assert!(temp_dir.path().join("test_project").exists());
}

#[test]
fn test_cli_cpp_command() {
	let temp_dir = TempDir::new().unwrap();
	let output = Command::new(sticks_binary())
		.args(["cpp", "test_project"])
		.current_dir(temp_dir.path())
		.output()
		.expect("Failed to execute command");

	assert!(
		output.status.success(),
		"CLI failed: {}",
		String::from_utf8_lossy(&output.stderr)
	);
	assert!(temp_dir.path().join("test_project").exists());
}

#[test]
fn test_cli_cmake_build() {
	let temp_dir = TempDir::new().unwrap();
	let output = Command::new(sticks_binary())
		.args(["cpp", "test_project", "--build", "cmake"])
		.current_dir(temp_dir.path())
		.output()
		.expect("Failed to execute command");

	assert!(
		output.status.success(),
		"CLI failed: {}",
		String::from_utf8_lossy(&output.stderr)
	);
	assert!(temp_dir.path().join("test_project/CMakeLists.txt").exists());
}

#[test]
fn test_cli_init_command() {
	let temp_dir = TempDir::new().unwrap();
	let output = Command::new(sticks_binary())
		.args(["init", "c"])
		.current_dir(temp_dir.path())
		.output()
		.expect("Failed to execute command");

	assert!(
		output.status.success(),
		"CLI failed: {}",
		String::from_utf8_lossy(&output.stderr)
	);
	assert!(temp_dir.path().join("src/main.c").exists());
}

#[test]
fn test_cli_version_flag() {
	let output = Command::new(sticks_binary())
		.arg("--version")
		.output()
		.expect("Failed to execute command");

	assert!(output.status.success());
	let stdout = String::from_utf8_lossy(&output.stdout);
	assert!(stdout.contains("sticks"));
}

#[test]
fn test_cli_help_flag() {
	let output = Command::new(sticks_binary())
		.arg("--help")
		.output()
		.expect("Failed to execute command");

	assert!(output.status.success());
	let stdout = String::from_utf8_lossy(&output.stdout);
	assert!(stdout.contains("sticks"));
	assert!(stdout.contains("A tool for managing C and C++ projects"));
}

#[test]
fn test_cli_shortcut_aliases() {
	let temp_dir = TempDir::new().unwrap();
	let output = Command::new(sticks_binary())
		.args(["i", "c"])
		.current_dir(temp_dir.path())
		.output()
		.expect("Failed to execute command");

	assert!(
		output.status.success(),
		"Shortcut 'i' failed: {}",
		String::from_utf8_lossy(&output.stderr)
	);
	assert!(temp_dir.path().join("src/main.c").exists());
}
