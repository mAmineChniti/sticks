use std::fs;
use std::str::FromStr;
use sticks::docs::{DocGenerator, DocTool};
use tempfile::TempDir;

#[test]
fn test_doc_tool_from_str() {
	assert_eq!(DocTool::from_str("doxygen").unwrap(), DocTool::Doxygen);
	assert_eq!(DocTool::from_str("sphinx").unwrap(), DocTool::Sphinx);
	assert!(DocTool::from_str("invalid").is_err());
}

#[test]
fn test_doc_tool_strings() {
	assert_eq!(DocTool::Doxygen.as_str(), "Doxygen");
	assert_eq!(DocTool::Sphinx.as_str(), "Sphinx");
}

#[test]
fn test_doc_generator_new() {
	let generator = DocGenerator::new(DocTool::Doxygen);
	assert_eq!(generator.tool, DocTool::Doxygen);
}

#[test]
fn test_doc_generator_generate_doxygen_config() {
	let generator = DocGenerator::new(DocTool::Doxygen);
	let config = generator.generate_config("test_project");

	assert!(config.contains("PROJECT_NAME"));
	assert!(config.contains("INPUT"));
	assert!(config.contains("GENERATE_HTML"));
}

#[test]
fn test_doc_generator_generate_sphinx_config() {
	let generator = DocGenerator::new(DocTool::Sphinx);
	let config = generator.generate_config("test_project");

	assert!(config.contains("project"));
	assert!(config.contains("extensions"));
	assert!(config.contains("html_theme"));
}

#[test]
fn test_doc_generator_write_config() {
	let temp_dir = TempDir::new().unwrap();

	let generator = DocGenerator::new(DocTool::Doxygen);
	let content = generator.generate_config("test_project");
	fs::write(temp_dir.path().join("Doxyfile"), content).unwrap();

	assert!(temp_dir.path().join("Doxyfile").exists());
}

#[test]
fn test_doc_generator_write_sphinx_config() {
	let temp_dir = TempDir::new().unwrap();

	let generator = DocGenerator::new(DocTool::Sphinx);
	let content = generator.generate_config("test_project");
	fs::create_dir_all(temp_dir.path().join("docs")).unwrap();
	fs::write(temp_dir.path().join("docs/conf.py"), content).unwrap();

	assert!(temp_dir.path().join("docs").exists());
	assert!(temp_dir.path().join("docs/conf.py").exists());
}

#[test]
fn test_doc_generator_add_to_cmake() {
	let temp_dir = TempDir::new().unwrap();
	let cmake_path = temp_dir.path().join("CMakeLists.txt");

	let cmake_content = r#"cmake_minimum_required(VERSION 3.10)
project(test_project)
add_executable(main src/main.cpp)
"#;
	fs::write(&cmake_path, cmake_content).unwrap();

	let generator = DocGenerator::new(DocTool::Doxygen);
	generator.add_to_cmake(&cmake_path, "test_project").unwrap();

	let result = fs::read_to_string(&cmake_path).unwrap();
	assert!(result.contains("find_package(Doxygen)"));
	assert!(result.contains("add_custom_target(docs"));
}

#[test]
fn test_doc_generator_add_to_makefile() {
	let temp_dir = TempDir::new().unwrap();
	let makefile_path = temp_dir.path().join("Makefile");

	let makefile_content = r#"CXX = g++
CXXFLAGS = -Wall

main: main.o
	$(CXX) -o main main.o
"#;
	fs::write(&makefile_path, makefile_content).unwrap();

	let generator = DocGenerator::new(DocTool::Sphinx);
	generator.add_to_makefile(&makefile_path).unwrap();

	let result = fs::read_to_string(&makefile_path).unwrap();
	assert!(result.contains("docs:"));
	assert!(result.contains("sphinx-build"));
}
