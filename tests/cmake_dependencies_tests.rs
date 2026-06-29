use serial_test::serial;
use std::env;
use std::fs;
use sticks::CMakeDependencyManager;

#[test]
#[serial]
fn test_cmake_parse_basic() {
	let content = "cmake_minimum_required(VERSION 3.15)\nproject(myapp)\n";
	let cmake = CMakeDependencyManager::parse(content);
	assert!(!cmake.content.is_empty());
}

#[test]
#[serial]
fn test_cmake_add_dependencies_basic() {
	let content =
		"cmake_minimum_required(VERSION 3.15)\nproject(myapp)\nadd_executable(myapp main.cpp)\n";
	let mut cmake = CMakeDependencyManager::parse(content);

	let added = cmake.add_dependencies(&["libcurl".to_string()]).unwrap();
	assert_eq!(added.len(), 1);
	assert!(cmake.content.contains("find_package(CURL"));
}

#[test]
#[serial]
fn test_cmake_add_multiple_dependencies() {
	let content = "cmake_minimum_required(VERSION 3.15)\nproject(myapp)\nadd_executable(myapp main.cpp)\ntarget_link_libraries(myapp)\n";
	let mut cmake = CMakeDependencyManager::parse(content);

	let added = cmake
		.add_dependencies(&["libcurl".to_string(), "openssl".to_string()])
		.unwrap();

	assert_eq!(added.len(), 2);
	assert!(cmake.content.contains("find_package(CURL"));
	assert!(cmake.content.contains("find_package(OPENSSL"));
}

#[test]
#[serial]
fn test_cmake_deduplicate_dependencies() {
	let content = "cmake_minimum_required(VERSION 3.15)\nproject(myapp)\nfind_package(CURL REQUIRED)\nadd_executable(myapp main.cpp)\n";
	let mut cmake = CMakeDependencyManager::parse(content);

	let added = cmake.add_dependencies(&["libcurl".to_string()]).unwrap();
	assert_eq!(added.len(), 0); // Already present (CURL is found from libcurl)
}

#[test]
#[serial]
fn test_cmake_remove_dependencies() {
	let content = "cmake_minimum_required(VERSION 3.15)\nproject(myapp)\nfind_package(CURL REQUIRED)\nfind_package(OPENSSL REQUIRED)\n";
	let mut cmake = CMakeDependencyManager::parse(content);

	let removed = cmake.remove_dependencies(&["libcurl".to_string()]).unwrap();
	assert_eq!(removed.len(), 1);
	assert!(!cmake.content.contains("find_package(CURL"));
	assert!(cmake.content.contains("find_package(OPENSSL"));
}

#[test]
#[serial]
fn test_cmake_write_to_file() {
	let temp_dir = env::temp_dir().join(format!(
		"sticks_test_cmake_write_{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos()
	));
	let original_dir = env::current_dir().unwrap();

	fs::remove_dir_all(&temp_dir).ok();
	fs::create_dir_all(&temp_dir).unwrap();
	env::set_current_dir(&temp_dir).unwrap();

	let content = "cmake_minimum_required(VERSION 3.15)\nproject(myapp)\n";
	let mut cmake = CMakeDependencyManager::parse(content);
	cmake.add_dependencies(&["libcurl".to_string()]).unwrap();

	cmake.write_to_file("CMakeLists.txt").unwrap();

	let written = fs::read_to_string("CMakeLists.txt").unwrap();
	assert!(written.contains("find_package(CURL"));

	env::set_current_dir(&original_dir).unwrap();
	fs::remove_dir_all(&temp_dir).ok();
}

#[test]
#[serial]
fn test_cmake_format_package_name() {
	// These are tested indirectly through add_dependencies
	// but we verify the transformations work correctly
	let content = "cmake_minimum_required(VERSION 3.15)\nproject(myapp)\n";
	let mut cmake = CMakeDependencyManager::parse(content);

	cmake.add_dependencies(&["libcurl".to_string()]).unwrap();
	assert!(cmake.content.contains("CURL")); // libcurl -> CURL

	let mut cmake2 = CMakeDependencyManager::parse(content);
	cmake2.add_dependencies(&["openssl".to_string()]).unwrap();
	assert!(cmake2.content.contains("OPENSSL")); // openssl -> OPENSSL
}
