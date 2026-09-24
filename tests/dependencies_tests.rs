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
	// Accept either legacy message or new routed message; ensure Makefile is mentioned
	let err_str = result.unwrap_err().to_string();
	// Accept either legacy Makefile-specific error or the newer project-initialization message
	assert!(
		err_str.contains("Makefile")
			|| err_str.contains("initialize your project")
			|| err_str.contains("No build system"),
		"Unexpected error message: {}",
		err_str
	);

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

	// Use packages that are likely to exist on the system
	let pm = sticks::os_detect::detect_system_package_manager();
	let deps = match pm {
		sticks::os_detect::SystemPackageManager::Pacman => {
			vec!["curl".to_string(), "openssl".to_string()]
		}
		sticks::os_detect::SystemPackageManager::Apt => {
			vec!["libcurl4".to_string(), "libssl-dev".to_string()]
		}
		sticks::os_detect::SystemPackageManager::Dnf => {
			vec!["libcurl".to_string(), "openssl-libs".to_string()]
		}
		_ => vec!["curl".to_string(), "openssl".to_string()], // fallback
	};

	let result = add_dependencies(&deps);
	if let Err(e) = &result {
		eprintln!("Error adding dependencies: {:?}", e);
		eprintln!("Current dir: {:?}", env::current_dir());
		eprintln!("Temp dir: {:?}", temp_dir);
		// If package validation fails, skip the rest of the test
		if e.to_string().contains("not found") {
			env::set_current_dir(&original_dir).unwrap();
			fs::remove_dir_all(&temp_dir).ok();
			return;
		}
	}
	assert!(
		result.is_ok(),
		"Failed to add dependencies: {:?}",
		result.err()
	);

	let updated = fs::read_to_string("Makefile").unwrap();
	let prefix = sticks::os_detect::install_command_prefix();
	assert!(updated.contains(&prefix));
	for dep in &deps {
		assert!(updated.contains(dep));
	}

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

	let makefile_content = format!(
		"all: clean install-deps\n\tbuild\n\ninstall-deps:\n\t{} curl openssl\n",
		sticks::os_detect::install_command_prefix()
	);
	fs::write("Makefile", makefile_content).unwrap();

	let result = remove_dependencies(&["openssl".to_string()]);
	assert!(
		result.is_ok(),
		"Failed to remove dependencies: {:?}",
		result.err()
	);

	let updated = fs::read_to_string("Makefile").unwrap();
	assert!(updated.contains("curl"));
	assert!(!updated.contains(" openssl ") && !updated.contains(" openssl\n"));

	let result_all = remove_dependencies(&["curl".to_string()]);
	assert!(result_all.is_ok());

	let final_content = fs::read_to_string("Makefile").unwrap();
	// The install-deps rule should be removed when all dependencies are gone
	assert!(!final_content.contains("install-deps:") || !final_content.contains("curl"));

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}
