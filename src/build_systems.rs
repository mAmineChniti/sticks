use crate::languages::{Language, LanguageConsts};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BuildSystem {
	Makefile,
	CMake,
}

impl std::fmt::Display for BuildSystem {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			BuildSystem::Makefile => write!(f, "Makefile"),
			BuildSystem::CMake => write!(f, "CMake"),
		}
	}
}

impl FromStr for BuildSystem {
	type Err = anyhow::Error;

	fn from_str(input: &str) -> Result<BuildSystem, Self::Err> {
		match input.to_lowercase().as_str() {
			"makefile" | "make" => Ok(BuildSystem::Makefile),
			"cmake" => Ok(BuildSystem::CMake),
			_ => anyhow::bail!(
				"Unsupported build system: {}. Use 'makefile' or 'cmake'",
				input
			),
		}
	}
}

pub trait BuildSystemGenerator {
	fn name(&self) -> &'static str;
	fn generate_build_file(&self, language: Language, project_name: &str) -> String;
	fn extension(&self) -> &'static str;
}

pub struct MakefileGenerator;

impl BuildSystemGenerator for MakefileGenerator {
	fn name(&self) -> &'static str {
		"Makefile"
	}

	fn generate_build_file(&self, language: Language, project_name: &str) -> String {
		language.generate_makefile_content(project_name)
	}

	fn extension(&self) -> &'static str {
		"Makefile"
	}
}

pub struct CMakeGenerator;

impl BuildSystemGenerator for CMakeGenerator {
	fn name(&self) -> &'static str {
		"CMakeLists.txt"
	}

	fn generate_build_file(&self, language: Language, project_name: &str) -> String {
		match language {
			Language::C => format!(
				"cmake_minimum_required(VERSION 3.15)\n\
				project({} C)\n\
				\n\
				set(CMAKE_C_STANDARD 11)\n\
				set(CMAKE_C_STANDARD_REQUIRED ON)\n\
				set(CMAKE_C_FLAGS \"${{CMAKE_C_FLAGS}} -Wall -Wextra -Werror\")\n\
				set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${{CMAKE_SOURCE_DIR}}/bin)\n\
				\n\
				file(GLOB_RECURSE SOURCES \"src/*.c\")\n\
				\n\
				add_executable(${{PROJECT_NAME}} ${{SOURCES}})\n\
				target_include_directories(${{PROJECT_NAME}} PRIVATE \"${{CMAKE_CURRENT_SOURCE_DIR}}/include\")\n\
				\n\
				# Optional: Installation\n\
				install(TARGETS ${{PROJECT_NAME}} DESTINATION bin)\n",
				project_name
			),
			Language::Cpp => format!(
				"cmake_minimum_required(VERSION 3.15)\n\
				project({} CXX)\n\
				\n\
				set(CMAKE_CXX_STANDARD 17)\n\
				set(CMAKE_CXX_STANDARD_REQUIRED ON)\n\
				set(CMAKE_CXX_FLAGS \"${{CMAKE_CXX_FLAGS}} -Wall -Wextra -Werror\")\n\
				set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${{CMAKE_SOURCE_DIR}}/bin)\n\
				\n\
				file(GLOB_RECURSE SOURCES \"src/*.cpp\")\n\
				\n\
				add_executable(${{PROJECT_NAME}} ${{SOURCES}})\n\
				target_include_directories(${{PROJECT_NAME}} PRIVATE \"${{CMAKE_CURRENT_SOURCE_DIR}}/include\")\n\
				\n\
				# Optional: Installation\n\
				install(TARGETS ${{PROJECT_NAME}} DESTINATION bin)\n",
				project_name
			),
		}
	}

	fn extension(&self) -> &'static str {
		"CMakeLists.txt"
	}
}

pub fn get_generator(build_system: BuildSystem) -> Box<dyn BuildSystemGenerator> {
	match build_system {
		BuildSystem::Makefile => Box::new(MakefileGenerator),
		BuildSystem::CMake => Box::new(CMakeGenerator),
	}
}

pub fn generate_cmake_build_script() -> &'static str {
	"#!/bin/bash\n\
	\n\
	set -e\n\
	\n\
	mkdir -p build\n\
	cd build\n\
	cmake -DCMAKE_BUILD_TYPE=Release ..\n\
	cmake --build .\n\
	\n\
	echo \"Build complete. Run ./build/$(basename $(pwd)/../) to execute.\"\n"
}

pub fn generate_cmake_debug_script() -> &'static str {
	"#!/bin/bash\n\
	\n\
	set -e\n\
	\n\
	mkdir -p build-debug\n\
	cd build-debug\n\
	cmake -DCMAKE_BUILD_TYPE=Debug ..\n\
	cmake --build .\n\
	\n\
	echo \"Debug build complete. Run ./build-debug/$(basename $(pwd)/../) to execute.\"\n"
}
