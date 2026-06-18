use std::fs;
use std::str::FromStr;
use sticks::build_config::{
	BuildConfig, CompilerFlags, CppStandard, IncludeDirs, LibraryDirs, PreprocessorDefs,
};
use tempfile::TempDir;

#[test]
fn test_cpp_standard_from_str() {
	assert_eq!(CppStandard::from_str("11").unwrap(), CppStandard::Cpp11);
	assert_eq!(CppStandard::from_str("cpp17").unwrap(), CppStandard::Cpp17);
	assert_eq!(CppStandard::from_str("C++20").unwrap(), CppStandard::Cpp20);
	assert!(CppStandard::from_str("invalid").is_err());
}

#[test]
fn test_cpp_standard_flags() {
	assert_eq!(CppStandard::Cpp11.to_cmake_flag(), "11");
	assert_eq!(CppStandard::Cpp17.to_cmake_flag(), "17");
	assert_eq!(CppStandard::Cpp11.to_gcc_flag(), "-std=c++11");
	assert_eq!(CppStandard::Cpp20.to_gcc_flag(), "-std=c++20");
}

#[test]
fn test_compiler_flags() {
	let mut flags = CompilerFlags::new();
	flags.add_flag("-Wall");
	flags.add_flag("-Wextra");
	assert_eq!(flags.flags.len(), 2);
	flags.remove_flag("-Wall");
	assert_eq!(flags.flags.len(), 1);
}

#[test]
fn test_compiler_flags_display() {
	let mut flags = CompilerFlags::new();
	flags.add_flag("-Wall");
	flags.add_flag("-Wextra");
	assert_eq!(format!("{}", flags), "-Wall -Wextra");
}

#[test]
fn test_preprocessor_defs() {
	let mut defs = PreprocessorDefs::new();
	defs.add_def("DEBUG");
	defs.add_def("NDEBUG");
	assert_eq!(defs.defs.len(), 2);
	defs.remove_def("DEBUG");
	assert_eq!(defs.defs.len(), 1);
}

#[test]
fn test_preprocessor_defs_cmake_string() {
	let mut defs = PreprocessorDefs::new();
	defs.add_def("DEBUG");
	defs.add_def("VERSION=1");
	assert!(defs.to_cmake_string().contains("DEBUG"));
	assert!(defs.to_cmake_string().contains("VERSION=1"));
}

#[test]
fn test_preprocessor_defs_gcc_string() {
	let mut defs = PreprocessorDefs::new();
	defs.add_def("DEBUG");
	assert_eq!(defs.to_gcc_string(), "-DDEBUG");
}

#[test]
fn test_include_dirs() {
	let mut dirs = IncludeDirs::new();
	dirs.add_dir("include");
	dirs.add_dir("src");
	assert_eq!(dirs.dirs.len(), 2);
	dirs.remove_dir("include");
	assert_eq!(dirs.dirs.len(), 1);
}

#[test]
fn test_include_dirs_deduplication() {
	let mut dirs = IncludeDirs::new();
	dirs.add_dir("include");
	dirs.add_dir("include");
	assert_eq!(dirs.dirs.len(), 1);
}

#[test]
fn test_library_dirs() {
	let mut dirs = LibraryDirs::new();
	dirs.add_dir("/usr/local/lib");
	dirs.add_dir("lib");
	assert_eq!(dirs.dirs.len(), 2);
	dirs.remove_dir("lib");
	assert_eq!(dirs.dirs.len(), 1);
}

#[test]
fn test_build_config_new() {
	let config = BuildConfig::new();
	assert!(config.cpp_standard.is_none());
	assert!(config.compiler_flags.flags.is_empty());
	assert!(config.preprocessor_defs.defs.is_empty());
	assert!(config.include_dirs.dirs.is_empty());
	assert!(config.library_dirs.dirs.is_empty());
}

#[test]
fn test_build_config_apply_to_cmake() {
	let temp_dir = TempDir::new().unwrap();
	let cmake_path = temp_dir.path().join("CMakeLists.txt");

	let cmake_content = r#"cmake_minimum_required(VERSION 3.10)
project(test_project)
add_executable(main src/main.cpp)
target_link_libraries(main)
"#;
	fs::write(&cmake_path, cmake_content).unwrap();

	let mut config = BuildConfig::new();
	config.cpp_standard = Some(CppStandard::Cpp17);
	config.compiler_flags.add_flag("-Wall");
	config.preprocessor_defs.add_def("DEBUG");
	config.include_dirs.add_dir("include");
	config.library_dirs.add_dir("lib");

	config.apply_to_cmake(&cmake_path).unwrap();

	let result = fs::read_to_string(&cmake_path).unwrap();
	assert!(result.contains("CMAKE_CXX_STANDARD 17"));
	assert!(result.contains("-Wall"));
	assert!(result.contains("DEBUG"));
	assert!(result.contains("include"));
	assert!(result.contains("lib"));
}

#[test]
fn test_build_config_apply_to_makefile() {
	let temp_dir = TempDir::new().unwrap();
	let makefile_path = temp_dir.path().join("Makefile");

	let makefile_content = r#"CXX = g++
CXXFLAGS = -Wall
LDFLAGS =

main: main.o
	$(CXX) $(LDFLAGS) -o main main.o
"#;
	fs::write(&makefile_path, makefile_content).unwrap();

	let mut config = BuildConfig::new();
	config.cpp_standard = Some(CppStandard::Cpp17);
	config.compiler_flags.add_flag("-O2");
	config.preprocessor_defs.add_def("DEBUG");
	config.include_dirs.add_dir("include");
	config.library_dirs.add_dir("lib");

	config.apply_to_makefile(&makefile_path).unwrap();

	let result = fs::read_to_string(&makefile_path).unwrap();
	assert!(result.contains("-std=c++17"));
	assert!(result.contains("-O2"));
	assert!(result.contains("-DDEBUG"));
	assert!(result.contains("-Iinclude"));
	assert!(result.contains("-Llib"));
}
