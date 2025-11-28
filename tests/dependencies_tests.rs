use serial_test::serial;
use std::env;
use std::fs;
use sticks::{add_dependencies, remove_dependencies};

#[test]
#[serial]
fn test_add_dependencies_no_makefile() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_deps_{}_{}",
		std::process::id(),
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let original_dir = env::current_dir().unwrap();

	fs::remove_dir_all(&temp_dir).ok();
	fs::create_dir_all(&temp_dir).unwrap();
	env::set_current_dir(&temp_dir).unwrap();

	let result = add_dependencies(&["libcurl".to_string()]);
	assert!(result.is_err());
	assert!(result
		.unwrap_err()
		.to_string()
		.contains("Makefile not found"));

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}

#[test]
#[serial]
fn test_add_dependencies_success() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_add_{}_{}",
		std::process::id(),
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let original_dir = env::current_dir().unwrap();

	fs::remove_dir_all(&temp_dir).ok();
	fs::create_dir_all(&temp_dir).unwrap();
	env::set_current_dir(&temp_dir).unwrap();

	let makefile_content = "all: clean\n\tbuild\n";
	fs::write("Makefile", makefile_content).unwrap();

	let result = add_dependencies(&["libcurl".to_string(), "openssl".to_string()]);
	if let Err(e) = &result {
		eprintln!("Error adding dependencies: {:?}", e);
		eprintln!("Current dir: {:?}", env::current_dir());
		eprintln!("Temp dir: {:?}", temp_dir);
	}
	assert!(
		result.is_ok(),
		"Failed to add dependencies: {:?}",
		result.err()
	);

	let updated = fs::read_to_string("Makefile").unwrap();
	assert!(updated.contains("sudo apt install"));
	assert!(updated.contains("libcurl"));
	assert!(updated.contains("openssl"));

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}

#[test]
#[serial]
fn test_remove_dependencies_no_makefile() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_remove_{}_{}",
		std::process::id(),
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let original_dir = env::current_dir().unwrap();

	fs::remove_dir_all(&temp_dir).ok();
	fs::create_dir_all(&temp_dir).unwrap();
	env::set_current_dir(&temp_dir).unwrap();

	let result = remove_dependencies(&["libcurl".to_string()]);
	assert!(result.is_err());

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}

#[test]
#[serial]
fn test_remove_dependencies_success() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_rm_{}_{}",
		std::process::id(),
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let original_dir = env::current_dir().unwrap();

	fs::remove_dir_all(&temp_dir).ok();
	fs::create_dir_all(&temp_dir).unwrap();
	env::set_current_dir(&temp_dir).unwrap();

	let makefile_content = "all: clean install-deps\n\tbuild\n\ninstall-deps:\n\tsudo apt install -y libcurl openssl libssl-dev\n";
	fs::write("Makefile", makefile_content).unwrap();

	let result = remove_dependencies(&["openssl".to_string()]);
	assert!(
		result.is_ok(),
		"Failed to remove dependencies: {:?}",
		result.err()
	);

	let updated = fs::read_to_string("Makefile").unwrap();
	assert!(updated.contains("libcurl"));
	assert!(updated.contains("libssl-dev"));
	assert!(!updated.contains(" openssl ") && !updated.contains(" openssl\n"));

	let result_all = remove_dependencies(&["libcurl".to_string(), "libssl-dev".to_string()]);
	assert!(result_all.is_ok());

	let final_content = fs::read_to_string("Makefile").unwrap();
	assert!(
		!final_content.contains("install-deps:") || !final_content.contains("sudo apt install")
	);

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}
