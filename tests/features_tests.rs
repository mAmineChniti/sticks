use serial_test::serial;
use std::env;
use std::fs;
use std::path::Path;
use sticks::{add_package_manager_to_project, detect_package_manager, PackageManager};
use sticks::{convert_build_system, detect_build_system};

#[test]
#[serial]
fn test_detect_build_system_makefile() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_detect_make_{}_{}",
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
	fs::write("Makefile", "all:\n\tbuild").unwrap();

	let result = detect_build_system().unwrap();
	assert_eq!(result, Some(sticks::BuildSystem::Makefile));

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}

#[test]
#[serial]
fn test_detect_build_system_cmake() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_detect_cmake_{}_{}",
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
	fs::write("CMakeLists.txt", "cmake_minimum_required(VERSION 3.15)").unwrap();

	let result = detect_build_system().unwrap();
	assert_eq!(result, Some(sticks::BuildSystem::CMake));

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}

#[test]
#[serial]
fn test_detect_no_build_system() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_detect_none_{}_{}",
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

	let result = detect_build_system().unwrap();
	assert_eq!(result, None);

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}

#[test]
#[serial]
fn test_detect_package_manager_conan() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_pm_conan_{}_{}",
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
	fs::write("conanfile.txt", "[requires]").unwrap();

	let result = detect_package_manager().unwrap();
	assert_eq!(result, Some(PackageManager::Conan));

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}

#[test]
#[serial]
fn test_detect_package_manager_vcpkg() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_pm_vcpkg_{}_{}",
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
	fs::write("vcpkg.json", "{}").unwrap();

	let result = detect_package_manager().unwrap();
	assert_eq!(result, Some(PackageManager::Vcpkg));

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}

#[test]
#[serial]
fn test_convert_build_system_makefile_to_cmake() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_conv_m2c_{}_{}",
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

	// Create a Makefile project
	fs::create_dir("src").unwrap();
	fs::write("src/main.c", "int main() {}").unwrap();
	fs::write("Makefile", "all:\n\tbuild").unwrap();

	// Verify Makefile exists
	assert!(Path::new("Makefile").exists());

	// Convert to CMake
	let result = convert_build_system(
		sticks::BuildSystem::Makefile,
		sticks::BuildSystem::CMake,
		"test_project",
	);
	assert!(result.is_ok(), "Failed to convert: {:?}", result.err());

	// Verify conversion
	assert!(
		!Path::new("Makefile").exists(),
		"Makefile should be removed"
	);
	assert!(
		Path::new("CMakeLists.txt").exists(),
		"CMakeLists.txt should exist"
	);

	let cmake_content = fs::read_to_string("CMakeLists.txt").unwrap();
	assert!(cmake_content.contains("cmake_minimum_required"));
	assert!(cmake_content.contains("test_project"));

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}

#[test]
#[serial]
fn test_convert_same_build_system_fails() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_conv_same_{}_{}",
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
	fs::write("Makefile", "all:\n\tbuild").unwrap();

	let result = convert_build_system(
		sticks::BuildSystem::Makefile,
		sticks::BuildSystem::Makefile,
		"test_project",
	);
	assert!(
		result.is_err(),
		"Should fail when converting to same system"
	);

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}

#[test]
#[serial]
fn test_add_package_manager_to_project() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_add_pm_{}_{}",
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

	// Create basic project structure
	fs::create_dir("src").unwrap();
	fs::write("src/main.c", "int main() {}").unwrap();

	// Add Conan
	let result = add_package_manager_to_project(PackageManager::Conan, "my_project");
	assert!(result.is_ok(), "Failed to add Conan: {:?}", result.err());
	assert!(Path::new("conanfile.txt").exists());

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}
