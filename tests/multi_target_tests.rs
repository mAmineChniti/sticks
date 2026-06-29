use std::fs;
use std::str::FromStr;
use sticks::multi_target::{BuildTarget, MultiTargetManager, TargetType};
use tempfile::TempDir;

#[test]
fn test_target_type_from_str() {
	assert_eq!(TargetType::from_str("exe").unwrap(), TargetType::Executable);
	assert_eq!(
		TargetType::from_str("static").unwrap(),
		TargetType::StaticLibrary
	);
	assert_eq!(
		TargetType::from_str("shared").unwrap(),
		TargetType::SharedLibrary
	);
	assert!(TargetType::from_str("invalid").is_err());
}

#[test]
fn test_target_type_strings() {
	assert_eq!(TargetType::Executable.as_str(), "executable");
	assert_eq!(TargetType::StaticLibrary.as_str(), "static library");
	assert_eq!(TargetType::SharedLibrary.as_str(), "shared library");
}

#[test]
fn test_target_type_cmake_target_type() {
	assert_eq!(TargetType::Executable.cmake_target_type(), "add_executable");
	assert_eq!(TargetType::StaticLibrary.cmake_target_type(), "add_library");
	assert_eq!(TargetType::SharedLibrary.cmake_target_type(), "add_library");
}

#[test]
fn test_build_target_new() {
	let target = BuildTarget::new("main".to_string(), TargetType::Executable);
	assert_eq!(target.name, "main");
	assert_eq!(target.target_type, TargetType::Executable);
	assert!(target.sources.is_empty());
	assert!(target.dependencies.is_empty());
}

#[test]
fn test_build_target_add_source() {
	let mut target = BuildTarget::new("main".to_string(), TargetType::Executable);
	target.add_source("src/main.cpp".to_string());
	target.add_source("src/util.cpp".to_string());
	assert_eq!(target.sources.len(), 2);
}

#[test]
fn test_build_target_add_dependency() {
	let mut target = BuildTarget::new("main".to_string(), TargetType::Executable);
	target.add_dependency("pthread".to_string());
	target.add_dependency("m".to_string());
	assert_eq!(target.dependencies.len(), 2);
}

#[test]
fn test_multi_target_manager_new() {
	let manager = MultiTargetManager::new();
	assert!(manager.targets.is_empty());
}

#[test]
fn test_multi_target_manager_add_target() {
	let mut manager = MultiTargetManager::new();
	let target = BuildTarget::new("main".to_string(), TargetType::Executable);
	manager.add_target(target);
	assert_eq!(manager.targets.len(), 1);
}

#[test]
fn test_multi_target_manager_add_to_cmake() {
	let temp_dir = TempDir::new().unwrap();
	let cmake_path = temp_dir.path().join("CMakeLists.txt");

	let cmake_content = r#"cmake_minimum_required(VERSION 3.10)
project(test_project)
"#;
	fs::write(&cmake_path, cmake_content).unwrap();

	let mut manager = MultiTargetManager::new();
	let mut main_target = BuildTarget::new("main".to_string(), TargetType::Executable);
	main_target.add_source("src/main.cpp".to_string());
	main_target.add_dependency("pthread".to_string());
	manager.add_target(main_target);

	let mut lib_target = BuildTarget::new("utils".to_string(), TargetType::StaticLibrary);
	lib_target.add_source("src/utils.cpp".to_string());
	manager.add_target(lib_target);

	manager.add_to_cmake(&cmake_path).unwrap();

	let result = fs::read_to_string(&cmake_path).unwrap();
	assert!(result.contains("add_executable(main"));
	assert!(result.contains("add_library(utils"));
	assert!(result.contains("target_link_libraries(main"));
	assert!(result.contains("pthread"));
}

#[test]
fn test_multi_target_manager_add_to_makefile() {
	let temp_dir = TempDir::new().unwrap();
	let makefile_path = temp_dir.path().join("Makefile");

	let makefile_content = r#"CXX = g++
CXXFLAGS = -Wall
"#;
	fs::write(&makefile_path, makefile_content).unwrap();

	let mut manager = MultiTargetManager::new();
	let mut main_target = BuildTarget::new("main".to_string(), TargetType::Executable);
	main_target.add_source("src/main.cpp".to_string());
	manager.add_target(main_target);

	manager.add_to_makefile(&makefile_path).unwrap();

	let result = fs::read_to_string(&makefile_path).unwrap();
	assert!(result.contains("main:"));
	assert!(result.contains("src/main.cpp"));
}
