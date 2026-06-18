use std::fs;
use std::str::FromStr;
use sticks::static_analysis::{StaticAnalysisGenerator, StaticAnalysisTool};
use tempfile::TempDir;

#[test]
fn test_static_analysis_tool_from_str() {
	assert_eq!(
		StaticAnalysisTool::from_str("clang-tidy").unwrap(),
		StaticAnalysisTool::ClangTidy
	);
	assert_eq!(
		StaticAnalysisTool::from_str("cppcheck").unwrap(),
		StaticAnalysisTool::Cppcheck
	);
	assert!(StaticAnalysisTool::from_str("invalid").is_err());
}

#[test]
fn test_static_analysis_tool_strings() {
	assert_eq!(StaticAnalysisTool::ClangTidy.as_str(), "clang-tidy");
	assert_eq!(StaticAnalysisTool::Cppcheck.as_str(), "cppcheck");
}

#[test]
fn test_static_analysis_generator_new() {
	let generator = StaticAnalysisGenerator::new(StaticAnalysisTool::ClangTidy);
	assert_eq!(generator.tool, StaticAnalysisTool::ClangTidy);
}

#[test]
fn test_static_analysis_generate_clang_tidy_config() {
	let generator = StaticAnalysisGenerator::new(StaticAnalysisTool::ClangTidy);
	let config = generator.generate_config();

	assert!(config.contains("Checks:"));
	assert!(config.contains("bugprone-"));
	assert!(config.contains("modernize-"));
	assert!(config.contains("performance-"));
}

#[test]
fn test_static_analysis_generate_cppcheck_config() {
	let generator = StaticAnalysisGenerator::new(StaticAnalysisTool::Cppcheck);
	let config = generator.generate_config();

	assert!(config.contains("<cppcheck>"));
	assert!(config.contains("<suppressions>"));
}

#[test]
fn test_static_analysis_write_config() {
	let temp_dir = TempDir::new().unwrap();
	let config_path = temp_dir.path().join(".clang-tidy");

	let generator = StaticAnalysisGenerator::new(StaticAnalysisTool::ClangTidy);
	let content = generator.generate_config();
	fs::write(&config_path, content).unwrap();

	assert!(config_path.exists());
}

#[test]
fn test_static_analysis_write_cppcheck_config() {
	let temp_dir = TempDir::new().unwrap();
	let config_path = temp_dir.path().join("cppcheck.xml");

	let generator = StaticAnalysisGenerator::new(StaticAnalysisTool::Cppcheck);
	let content = generator.generate_config();
	fs::write(&config_path, content).unwrap();

	assert!(config_path.exists());
}

#[test]
fn test_static_analysis_add_to_cmake() {
	let temp_dir = TempDir::new().unwrap();
	let cmake_path = temp_dir.path().join("CMakeLists.txt");

	let cmake_content = r#"cmake_minimum_required(VERSION 3.10)
project(test_project)
add_executable(main src/main.cpp)
"#;
	fs::write(&cmake_path, cmake_content).unwrap();

	let generator = StaticAnalysisGenerator::new(StaticAnalysisTool::ClangTidy);
	generator.add_to_cmake(&cmake_path).unwrap();

	let result = fs::read_to_string(&cmake_path).unwrap();
	assert!(result.contains("find_program(CLANG_TIDY"));
	assert!(result.contains("CXX_CLANG_TIDY"));
}

#[test]
fn test_static_analysis_cppcheck_add_to_cmake() {
	let temp_dir = TempDir::new().unwrap();
	let cmake_path = temp_dir.path().join("CMakeLists.txt");

	let cmake_content = r#"cmake_minimum_required(VERSION 3.10)
project(test_project)
add_executable(main src/main.cpp)
"#;
	fs::write(&cmake_path, cmake_content).unwrap();

	let generator = StaticAnalysisGenerator::new(StaticAnalysisTool::Cppcheck);
	generator.add_to_cmake(&cmake_path).unwrap();

	let result = fs::read_to_string(&cmake_path).unwrap();
	assert!(result.contains("find_program(CPPCHECK"));
	assert!(result.contains("add_custom_target(cppcheck"));
}

#[test]
fn test_static_analysis_add_to_makefile() {
	let temp_dir = TempDir::new().unwrap();
	let makefile_path = temp_dir.path().join("Makefile");

	let makefile_content = r#"CXX = g++
CXXFLAGS = -Wall

main: main.o
	$(CXX) -o main main.o
"#;
	fs::write(&makefile_path, makefile_content).unwrap();

	let generator = StaticAnalysisGenerator::new(StaticAnalysisTool::ClangTidy);
	generator.add_to_makefile(&makefile_path).unwrap();

	let result = fs::read_to_string(&makefile_path).unwrap();
	assert!(result.contains("lint:"));
	assert!(result.contains("clang-tidy"));
}

#[test]
fn test_static_analysis_cppcheck_add_to_makefile() {
	let temp_dir = TempDir::new().unwrap();
	let makefile_path = temp_dir.path().join("Makefile");

	let makefile_content = r#"CXX = g++
CXXFLAGS = -Wall

main: main.o
	$(CXX) -o main main.o
"#;
	fs::write(&makefile_path, makefile_content).unwrap();

	let generator = StaticAnalysisGenerator::new(StaticAnalysisTool::Cppcheck);
	generator.add_to_makefile(&makefile_path).unwrap();

	let result = fs::read_to_string(&makefile_path).unwrap();
	assert!(result.contains("lint:"));
	assert!(result.contains("cppcheck"));
}
