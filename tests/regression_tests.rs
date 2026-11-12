//! Regression tests for the build-file corruption bugs fixed in 0.4.2.
//!
//! Every test here reproduces a scenario that previously produced a broken
//! build file while still reporting success.

use serial_test::serial;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn project() -> TempDir {
	tempfile::tempdir().expect("tempdir")
}

fn write(dir: &Path, name: &str, content: &str) {
	fs::write(dir.join(name), content).expect("write fixture");
}

fn read(dir: &Path, name: &str) -> String {
	fs::read_to_string(dir.join(name)).expect("read file")
}

// ---------------------------------------------------------------------------
// test_framework: injection into a multiline `project()` call
// ---------------------------------------------------------------------------

#[test]
fn test_add_preserves_multiline_project_declaration() {
	let dir = project();
	write(
		dir.path(),
		"CMakeLists.txt",
		"cmake_minimum_required(VERSION 3.10)\nproject(\n  demo\n  LANGUAGES CXX\n)\nadd_executable(app main.cpp)\n",
	);

	sticks::test_framework::TestFrameworkManager::new(
		sticks::test_framework::TestFramework::GoogleTest,
	)
	.add_to_cmake(&dir.path().join("CMakeLists.txt"), "demo")
	.expect("add gtest");

	let content = read(dir.path(), "CMakeLists.txt");

	// The `project(` call must be byte-identical: nothing may be inserted
	// between the command name and its closing paren.
	assert!(
		content.contains("project(\n  demo\n  LANGUAGES CXX\n)"),
		"multiline project() was split:\n{content}"
	);
	// Directives go on their own lines after the call.
	assert!(
		content.contains(")\nenable_testing()\n"),
		"directives must follow the closing paren:\n{content}"
	);
	assert!(content.contains("find_package(GTest REQUIRED)"));
	assert!(content.contains("add_executable(demo_test tests/test_demo.cpp)"));
}

#[test]
fn test_add_does_not_duplicate_versioned_find_package() {
	let dir = project();
	write(
		dir.path(),
		"CMakeLists.txt",
		"project(demo LANGUAGES CXX)\nfind_package(GTest 1.14.0 REQUIRED)\nadd_executable(app main.cpp)\n",
	);

	sticks::test_framework::TestFrameworkManager::new(
		sticks::test_framework::TestFramework::GoogleTest,
	)
	.add_to_cmake(&dir.path().join("CMakeLists.txt"), "demo")
	.expect("add gtest");

	let content = read(dir.path(), "CMakeLists.txt");
	assert_eq!(
		content.matches("find_package(GTest").count(),
		1,
		"a pinned find_package must not be duplicated:\n{content}"
	);
}

#[test]
fn test_find_package_inside_string_literal_is_ignored() {
	let dir = project();
	// A doc string that merely mentions the command must not satisfy (or
	// block) the real insertion.
	write(
		dir.path(),
		"CMakeLists.txt",
		"project(demo LANGUAGES CXX)\nset(DOC \"see find_package(Foo REQUIRED) here\")\nadd_executable(app main.cpp)\n",
	);

	sticks::test_framework::TestFrameworkManager::new(
		sticks::test_framework::TestFramework::Catch2,
	)
	.add_to_cmake(&dir.path().join("CMakeLists.txt"), "demo")
	.expect("add catch2");

	let content = read(dir.path(), "CMakeLists.txt");
	assert!(
		content.contains("set(DOC \"see find_package(Foo REQUIRED) here\")"),
		"string literal must be untouched:\n{content}"
	);
	assert!(
		content.contains("find_package(Catch2 3 REQUIRED)"),
		"the real find_package must still be added:\n{content}"
	);
}

// ---------------------------------------------------------------------------
// dependency_manager: multiline `requires = (...)` in conanfile.py
// ---------------------------------------------------------------------------

// Changes the process-wide CWD, so it must not run concurrently.
#[test]
#[serial]
fn conanfile_py_multiline_requires_survive_removal() {
	let dir = project();
	write(
		dir.path(),
		"conanfile.py",
		"from conan import ConanFile\n\nclass Pkg(ConanFile):\n    requires = (\n        \"zlib/1.2.13\",\n        \"boost/1.85.0\",\n    )\n",
	);

	let previous = std::env::current_dir().expect("cwd");
	std::env::set_current_dir(dir.path()).expect("enter tempdir");
	let result = sticks::DependencyManager::remove(&["boost".to_string()]);
	std::env::set_current_dir(previous).expect("restore cwd");
	result.expect("remove boost");

	let content = read(dir.path(), "conanfile.py");
	// The surviving requirement must appear exactly once and the continuation
	// lines must be gone, otherwise the file is not valid Python.
	assert_eq!(
		content.matches("zlib/1.2.13").count(),
		1,
		"kept requirement duplicated:\n{content}"
	);
	assert!(
		!content.contains("boost"),
		"removed requirement still present"
	);
	assert!(
		content.contains("requires = (\"zlib/1.2.13\",)"),
		"expected a single-line tuple:\n{content}"
	);
}

// Changes the process-wide CWD, so it must not run concurrently.
#[test]
#[serial]
fn conanfile_py_non_ascii_requirement_does_not_panic() {
	let dir = project();
	// `ü` is two bytes; slicing the string at a byte offset inside it used to
	// abort the whole process.
	write(
		dir.path(),
		"conanfile.py",
		"from conan import ConanFile\n\nclass Pkg(ConanFile):\n    requires = \"zlib/1.2.13 \u{fc}\"\n",
	);

	let previous = std::env::current_dir().expect("cwd");
	std::env::set_current_dir(dir.path()).expect("enter tempdir");
	let result = sticks::DependencyManager::add(&["boost/1.85.0".to_string()]);
	std::env::set_current_dir(previous).expect("restore cwd");
	result.expect("add must not panic");

	let content = read(dir.path(), "conanfile.py");
	assert!(
		content.contains('\u{fc}'),
		"non-ASCII must round-trip: {content}"
	);
	assert!(content.contains("boost/1.85.0"));
}

// ---------------------------------------------------------------------------
// makefile_parser: `.RECIPEPREFIX` and multi-line install-deps recipes
// ---------------------------------------------------------------------------

#[test]
fn makefile_recipe_prefix_is_preserved() {
	let dir = project();
	write(
		dir.path(),
		"Makefile",
		".RECIPEPREFIX = >\nall: main\n>echo building\n",
	);

	let mut makefile = sticks::Makefile::parse(&read(dir.path(), "Makefile")).expect("parse");
	makefile
		.add_dependencies(&["libcurl".to_string()])
		.expect("add dep");

	let rendered = makefile.to_makefile_string();
	for line in rendered.lines() {
		assert!(
			!line.starts_with('\t'),
			"a TAB recipe would make the Makefile unparseable:\n{rendered}"
		);
	}
	assert!(rendered.contains(">sudo"), "{rendered}");
}

#[test]
fn install_deps_keeps_unrelated_recipe_lines() {
	let dir = project();
	write(
		dir.path(),
		"Makefile",
		"all: install-deps\n\ninstall-deps:\n\techo installing dependencies\n\tsudo apt install -y libcurl\n",
	);

	let mut makefile = sticks::Makefile::parse(&read(dir.path(), "Makefile")).expect("parse");
	makefile
		.add_dependencies(&["libssl".to_string()])
		.expect("add dep");

	let rendered = makefile.to_makefile_string();
	assert!(
		rendered.contains("echo installing dependencies"),
		"non-package-manager recipe line was dropped:\n{rendered}"
	);
	assert!(rendered.contains("libcurl") && rendered.contains("libssl"));
}

#[test]
fn install_deps_joins_backslash_continuations() {
	let dir = project();
	write(
		dir.path(),
		"Makefile",
		"all: install-deps\n\ninstall-deps:\n\tsudo apt install -y \\\n\t\tlibcurl\n",
	);

	let mut makefile = sticks::Makefile::parse(&read(dir.path(), "Makefile")).expect("parse");
	makefile
		.add_dependencies(&["libssl".to_string()])
		.expect("add dep");

	let rendered = makefile.to_makefile_string();
	assert!(
		rendered.contains("libcurl"),
		"existing dep lost:\n{rendered}"
	);
	assert!(rendered.contains("libssl"), "new dep lost:\n{rendered}");
	assert!(
		!rendered.contains("\\\n"),
		"continuation must be collapsed so the backslash cannot swallow a package:\n{rendered}"
	);
}

// ---------------------------------------------------------------------------
// build_config: unrelated edits must not rewrite other commands
// ---------------------------------------------------------------------------

#[test]
fn config_edit_does_not_touch_include_directories() {
	let dir = project();
	let original = "cmake_minimum_required(VERSION 3.16)\nproject(demo LANGUAGES CXX)\nadd_executable(app src/main.cpp)\ntarget_include_directories(app SYSTEM PRIVATE \"$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>\")\n";
	write(dir.path(), "CMakeLists.txt", original);

	let path = dir.path().join("CMakeLists.txt");
	let mut config = sticks::BuildConfig::parse_from_cmake(&path).expect("parse");
	config.add_preprocessor_def("MYAPP");
	config.apply_to_cmake(&path).expect("apply");

	let content = read(dir.path(), "CMakeLists.txt");
	assert!(
		content.contains("MYAPP"),
		"the def was not added:\n{content}"
	);
	assert!(
		content.contains(
			"target_include_directories(app SYSTEM PRIVATE \"$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>\")"
		),
		"an unrelated edit rewrote target_include_directories:\n{content}"
	);
}

#[test]
fn config_include_edit_preserves_keywords_and_generator_expressions() {
	let dir = project();
	write(
		dir.path(),
		"CMakeLists.txt",
		"cmake_minimum_required(VERSION 3.16)\nproject(demo LANGUAGES CXX)\nadd_executable(app src/main.cpp)\ntarget_include_directories(app SYSTEM PRIVATE \"$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>\")\n",
	);

	let path = dir.path().join("CMakeLists.txt");
	let mut config = sticks::BuildConfig::parse_from_cmake(&path).expect("parse");
	config.add_include_dir("third_party");
	config.apply_to_cmake(&path).expect("apply");

	let content = read(dir.path(), "CMakeLists.txt");
	// `SYSTEM` must stay a keyword, never become a directory name.
	assert!(
		content.contains("SYSTEM PRIVATE"),
		"SYSTEM keyword lost:\n{content}"
	);
	assert!(
		!content.contains("/SYSTEM"),
		"SYSTEM was turned into a path:\n{content}"
	);
	assert!(
		content.contains("$<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>"),
		"generator expression was mangled:\n{content}"
	);
	assert!(
		content.contains("third_party"),
		"new dir missing:\n{content}"
	);
}

#[test]
fn config_flag_edit_preserves_cache_arguments_and_generator_expressions() {
	let dir = project();
	write(
		dir.path(),
		"CMakeLists.txt",
		"project(demo LANGUAGES CXX)\nadd_executable(app src/main.cpp)\nset(CMAKE_CXX_FLAGS \"-Wall $<$<CONFIG:Debug>:-g>\" CACHE STRING \"flags\" FORCE)\nset(CMAKE_CXX_STANDARD 17 CACHE STRING \"std\")\n",
	);

	let path = dir.path().join("CMakeLists.txt");
	let mut config = sticks::BuildConfig::parse_from_cmake(&path).expect("parse");
	config.add_compiler_flag("-O2");
	config.set_cpp_standard(sticks::CppStandard::Cpp20);
	config.apply_to_cmake(&path).expect("apply");

	let content = read(dir.path(), "CMakeLists.txt");
	assert!(
		content.contains("$<$<CONFIG:Debug>:-g>"),
		"generator expression was dropped from the flags:\n{content}"
	);
	assert!(content.contains("-O2"), "new flag missing:\n{content}");
	assert!(
		content.contains("CACHE STRING") && content.contains("FORCE"),
		"CACHE/FORCE arguments were dropped, changing variable scope:\n{content}"
	);
	assert!(
		content.contains("set(CMAKE_CXX_STANDARD 20"),
		"standard was not updated:\n{content}"
	);
}

#[test]
fn config_does_not_emit_a_noop_flags_set() {
	let dir = project();
	write(
		dir.path(),
		"CMakeLists.txt",
		"project(demo LANGUAGES CXX)\nadd_executable(app src/main.cpp)\n",
	);

	let path = dir.path().join("CMakeLists.txt");
	let mut config = sticks::BuildConfig::parse_from_cmake(&path).expect("parse");
	config.add_preprocessor_def("X");
	config.apply_to_cmake(&path).expect("apply");

	let content = read(dir.path(), "CMakeLists.txt");
	assert!(
		!content.contains("set(CMAKE_CXX_FLAGS \"${CMAKE_CXX_FLAGS}\")"),
		"a self-referential no-op set() was written:\n{content}"
	);
}

// ---------------------------------------------------------------------------
// cmake_dependencies: argument quoting must survive a rewrite
// ---------------------------------------------------------------------------

#[test]
fn find_package_rewrite_preserves_quoted_arguments() {
	let dir = project();
	write(
		dir.path(),
		"CMakeLists.txt",
		"project(demo LANGUAGES CXX)\nfind_package(Pkg PATHS \"/some path/lib\")\nadd_executable(app src/main.cpp)\n",
	);

	let cmake_path = dir.path().join("CMakeLists.txt");
	let mut manager = sticks::CMakeDependencyManager::from_file(&cmake_path).expect("load");
	manager
		.add_dependencies(&["Pkg:comp".to_string()])
		.expect("add component");
	manager.write_to_file(&cmake_path).expect("write");

	let content = read(dir.path(), "CMakeLists.txt");
	assert!(
		content.contains("\"/some path/lib\""),
		"a quoted path was split into separate arguments:\n{content}"
	);
}
