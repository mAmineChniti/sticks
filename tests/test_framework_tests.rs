use std::fs;
use std::str::FromStr;
use sticks::test_framework::{TestFramework, TestFrameworkManager};
use tempfile::TempDir;

#[test]
fn test_test_framework_from_str() {
	assert_eq!(
		TestFramework::from_str("gtest").unwrap(),
		TestFramework::GoogleTest
	);
	assert_eq!(
		TestFramework::from_str("catch2").unwrap(),
		TestFramework::Catch2
	);
	assert_eq!(
		TestFramework::from_str("doctest").unwrap(),
		TestFramework::Doctest
	);
	assert!(TestFramework::from_str("invalid").is_err());
}

#[test]
fn test_test_framework_strings() {
	assert_eq!(TestFramework::GoogleTest.as_str(), "GoogleTest");
	assert_eq!(TestFramework::Catch2.as_str(), "Catch2");
	assert_eq!(TestFramework::Doctest.as_str(), "Doctest");
}

#[test]
fn test_test_framework_cmake_find_package() {
	assert_eq!(TestFramework::GoogleTest.cmake_find_package(), "GTest");
	assert_eq!(TestFramework::Catch2.cmake_find_package(), "Catch2");
	assert_eq!(TestFramework::Doctest.cmake_find_package(), "doctest");
}

#[test]
fn test_test_framework_add_to_cmake() {
	let temp_dir = TempDir::new().unwrap();
	let cmake_path = temp_dir.path().join("CMakeLists.txt");

	let cmake_content = r#"cmake_minimum_required(VERSION 3.10)
project(test_project)
add_executable(main src/main.cpp)
"#;
	fs::write(&cmake_path, cmake_content).unwrap();

	let manager = TestFrameworkManager::new(TestFramework::GoogleTest);
	manager.add_to_cmake(&cmake_path, "test_project").unwrap();

	let result = fs::read_to_string(&cmake_path).unwrap();
	assert!(result.contains("find_package(GTest REQUIRED)"));
	assert!(result.contains("enable_testing()"));
	assert!(result.contains("test_project_test"));
}

#[test]
fn test_test_framework_add_to_makefile() {
	let temp_dir = TempDir::new().unwrap();
	let makefile_path = temp_dir.path().join("Makefile");

	let makefile_content = r#"CXX = g++
CXXFLAGS = -Wall

main: main.o
	$(CXX) -o main main.o
"#;
	fs::write(&makefile_path, makefile_content).unwrap();

	let manager = TestFrameworkManager::new(TestFramework::Catch2);
	manager
		.add_to_makefile(&makefile_path, "test_project")
		.unwrap();

	let result = fs::read_to_string(&makefile_path).unwrap();
	assert!(result.contains("test:"));
	assert!(result.contains("test_project_test"));
	assert!(result.contains("Catch2"));
}

#[test]
fn test_test_framework_generate_test_file() {
	let manager = TestFrameworkManager::new(TestFramework::GoogleTest);
	let test_content = manager.generate_test_file("my_project", "cpp");

	assert!(test_content.contains("gtest/gtest.h"));
	assert!(test_content.contains("MY_PROJECTTest"));
	assert!(test_content.contains("EXPECT_EQ"));
}

#[test]
fn test_test_framework_setup_test_files() {
	let temp_dir = TempDir::new().unwrap();
	std::env::set_current_dir(temp_dir.path()).unwrap();

	let manager = TestFrameworkManager::new(TestFramework::Doctest);
	manager.setup_test_files("test_project", "cpp").unwrap();

	assert!(temp_dir.path().join("tests").exists());
	assert!(temp_dir.path().join("tests/test_test_project.cpp").exists());

	let test_content =
		fs::read_to_string(temp_dir.path().join("tests/test_test_project.cpp")).unwrap();
	assert!(test_content.contains("doctest/doctest.h"));
}
