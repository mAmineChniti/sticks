use sticks::{
	generate_clang_format_config, generate_editorconfig, generate_gitattributes,
	generate_gitignore, generate_precommit_hook, generate_readme, generate_vscode_launch_config,
	generate_vscode_settings, generate_vscode_tasks_config, Language,
};

#[test]
fn test_generate_gitignore_c() {
	let gitignore = generate_gitignore(Language::C);
	assert!(gitignore.contains("build/"));
	assert!(gitignore.contains("*.o"));
	assert!(gitignore.contains("*.a"));
	assert!(gitignore.contains(".vscode/"));
	assert!(gitignore.contains("CMakeFiles/"));
}

#[test]
fn test_generate_gitignore_cpp() {
	let gitignore = generate_gitignore(Language::Cpp);
	assert!(gitignore.contains("build/"));
	assert!(gitignore.contains("*.so"));
	assert!(gitignore.contains(".idea/"));
	assert!(gitignore.contains("test_"));
}

#[test]
fn test_generate_editorconfig() {
	let config = generate_editorconfig();
	assert!(config.contains("root = true"));
	assert!(config.contains("indent_style = tab"));
	assert!(config.contains("indent_size = 4"));
	assert!(config.contains("end_of_line = lf"));
	assert!(config.contains("charset = utf-8"));
	assert!(config.contains("trim_trailing_whitespace = true"));
}

#[test]
fn test_generate_clang_format_c() {
	let config = generate_clang_format_config(Language::C);
	assert!(config.contains("Language: C"));
	assert!(config.contains("Standard: C11"));
	assert!(config.contains("IndentWidth: 4"));
	assert!(config.contains("ColumnLimit: 100"));
	assert!(config.contains("BreakBeforeBraces: Linux"));
}

#[test]
fn test_generate_clang_format_cpp() {
	let config = generate_clang_format_config(Language::Cpp);
	assert!(config.contains("Language: Cpp"));
	assert!(config.contains("Standard: C++17"));
	assert!(config.contains("IndentWidth: 4"));
	assert!(config.contains("ColumnLimit: 100"));
	assert!(config.contains("Cpp11BracedListStyle: true"));
}

#[test]
fn test_generate_vscode_settings_c() {
	let settings = generate_vscode_settings(Language::C);
	assert!(settings.contains("[c]"));
	assert!(settings.contains("ms-vscode.cpptools"));
	assert!(settings.contains("formatOnSave"));
	assert!(settings.contains("rulers"));
	assert!(settings.contains("[100]"));
}

#[test]
fn test_generate_vscode_settings_cpp() {
	let settings = generate_vscode_settings(Language::Cpp);
	assert!(settings.contains("\"[cpp]\""));
	assert!(settings.contains("ms-vscode.cpptools"));
	assert!(settings.contains("editor.formatOnSave"));
}

#[test]
fn test_generate_vscode_launch_config() {
	let launch = generate_vscode_launch_config("my_project");
	assert!(launch.contains("\"version\": \"0.2.0\""));
	assert!(launch.contains("C/C++ Debug"));
	assert!(launch.contains("my_project"));
	assert!(launch.contains("\"type\": \"cppdbg\""));
	assert!(launch.contains("\"MIMode\": \"gdb\""));
}

#[test]
fn test_generate_vscode_tasks_config() {
	let tasks = generate_vscode_tasks_config();
	assert!(tasks.contains("\"version\": \"2.0.0\""));
	assert!(tasks.contains("\"label\": \"build\""));
	assert!(tasks.contains("\"label\": \"rebuild\""));
	assert!(tasks.contains("cmake"));
	assert!(tasks.contains("problemMatcher"));
}

#[test]
fn test_generate_readme_c() {
	let readme = generate_readme("my_c_project", Language::C);
	assert!(readme.contains("# my_c_project"));
	assert!(readme.contains("C project"));
	assert!(readme.contains("Building"));
	assert!(readme.contains("CMake"));
	assert!(readme.contains("Makefile"));
}

#[test]
fn test_generate_readme_cpp() {
	let readme = generate_readme("my_cpp_project", Language::Cpp);
	assert!(readme.contains("# my_cpp_project"));
	assert!(readme.contains("C++ project"));
	assert!(readme.contains("Building"));
	assert!(readme.contains("Project Structure"));
}

#[test]
fn test_generate_precommit_hook() {
	let hook = generate_precommit_hook();
	assert!(hook.contains("#!/bin/bash"));
	assert!(hook.contains("clang-format"));
	assert!(hook.contains("Running pre-commit checks"));
}

#[test]
fn test_generate_gitattributes() {
	let attrs = generate_gitattributes();
	assert!(attrs.contains("*.c text eol=lf"));
	assert!(attrs.contains("*.cpp text eol=lf"));
	assert!(attrs.contains("*.h text eol=lf"));
	assert!(attrs.contains("Makefile text eol=lf"));
	assert!(attrs.contains("* text=auto"));
}

#[test]
fn test_generate_readme_includes_sticks_link() {
	let readme = generate_readme("test", Language::C);
	assert!(readme.contains("sticks"));
	assert!(readme.contains("github.com"));
}
