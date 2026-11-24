use sticks::{BuildSystem, BuildSystemGenerator, CMakeGenerator, Language, MakefileGenerator};

#[test]
fn test_build_system_display() {
	assert_eq!(format!("{}", BuildSystem::Makefile), "Makefile");
	assert_eq!(format!("{}", BuildSystem::CMake), "CMake");
}

#[test]
fn test_build_system_from_str() {
	assert!(matches!(
		"makefile".parse::<BuildSystem>(),
		Ok(BuildSystem::Makefile)
	));
	assert!(matches!(
		"make".parse::<BuildSystem>(),
		Ok(BuildSystem::Makefile)
	));
	assert!(matches!(
		"cmake".parse::<BuildSystem>(),
		Ok(BuildSystem::CMake)
	));
	assert!(matches!(
		"MAKEFILE".parse::<BuildSystem>(),
		Ok(BuildSystem::Makefile)
	));
	assert!(matches!(
		"CMAKE".parse::<BuildSystem>(),
		Ok(BuildSystem::CMake)
	));
	assert!("ninja".parse::<BuildSystem>().is_err());
	assert!("bazel".parse::<BuildSystem>().is_err());
}

#[test]
fn test_makefile_generator() {
	let generator = MakefileGenerator;
	assert_eq!(generator.name(), "Makefile");
	assert_eq!(generator.extension(), "Makefile");

	let makefile = generator.generate_build_file(Language::C, "test_project");
	assert!(makefile.contains("CC ?="));
	assert!(makefile.contains("TARGET = $(BIN_DIR)/test_project"));
	assert!(makefile.contains("BIN_DIR = bin"));
	assert!(makefile.contains("all:"));
	assert!(makefile.contains("clean:"));
	assert!(makefile.contains("format:"));
	assert!(makefile.contains("format-check:"));
	assert!(makefile.contains("lint: format-check lint-clang-tidy"));
	assert!(makefile.contains("lint-clang-tidy:"));
	assert!(makefile.contains("CLANG_FORMAT ?= clang-format"));
	assert!(makefile.contains("CLANG_TIDY ?= clang-tidy"));
	assert!(makefile.contains("FORMAT_SOURCES"));
	assert!(makefile.contains("--dry-run --Werror"));
	assert!(
		sticks::Makefile::parse(&makefile).is_ok(),
		"generated Makefile must stay parseable"
	);
}

#[test]
fn test_cmake_generator() {
	let generator = CMakeGenerator;
	assert_eq!(generator.name(), "CMakeLists.txt");
	assert_eq!(generator.extension(), "CMakeLists.txt");

	let cmake_c = generator.generate_build_file(Language::C, "test_project");
	assert!(cmake_c.contains("project(test_project C)"));
	assert!(cmake_c.contains("CMAKE_C_STANDARD 11"));
	assert!(cmake_c.contains("file(GLOB_RECURSE SOURCES"));
	assert!(cmake_c.contains("add_custom_target(format"));
	assert!(cmake_c.contains("add_custom_target(format-check"));
	assert!(cmake_c.contains("add_custom_target(lint)"));
	assert!(cmake_c.contains("add_custom_target(lint-clang-tidy"));
	assert!(cmake_c.contains("add_dependencies(lint format-check lint-clang-tidy)"));
	assert!(cmake_c.contains("--dry-run --Werror"));
	assert!(cmake_c.contains("CMAKE_EXPORT_COMPILE_COMMANDS ON"));

	let cmake_cpp = generator.generate_build_file(Language::Cpp, "test_project");
	assert!(cmake_cpp.contains("project(test_project CXX)"));
	assert!(cmake_cpp.contains("CMAKE_CXX_STANDARD 20"));
	assert!(cmake_cpp.contains("file(GLOB_RECURSE SOURCES"));
	assert!(cmake_cpp.contains("add_custom_target(format"));
	assert!(cmake_cpp.contains("add_custom_target(lint)"));
}

#[test]
fn test_makefile_generator_cpp() {
	let generator = MakefileGenerator;
	let makefile = generator.generate_build_file(Language::Cpp, "my_cpp_project");
	assert!(makefile.contains("CXX ?="));
	assert!(makefile.contains("TARGET = $(BIN_DIR)/my_cpp_project"));
	assert!(makefile.contains("BIN_DIR = bin"));
	assert!(makefile.contains("*.cpp"));
}

#[test]
fn test_cmake_generator_content() {
	let generator = CMakeGenerator;

	let cmake_c = generator.generate_build_file(Language::C, "my_c_app");
	assert!(cmake_c.contains("project(my_c_app C)"));
	assert!(cmake_c.contains("src/*.c"));

	let cmake_cpp = generator.generate_build_file(Language::Cpp, "my_cpp_app");
	assert!(cmake_cpp.contains("project(my_cpp_app CXX)"));
	assert!(cmake_cpp.contains("src/*.cpp"));
}

#[test]
fn test_build_system_equality() {
	assert_eq!(BuildSystem::Makefile, BuildSystem::Makefile);
	assert_eq!(BuildSystem::CMake, BuildSystem::CMake);
	assert_ne!(BuildSystem::Makefile, BuildSystem::CMake);
}
