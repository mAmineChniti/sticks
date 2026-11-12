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
	fn generate_build_file_with_toolchain(
		&self,
		language: Language,
		project_name: &str,
		toolchain: &crate::toolchain::Toolchain,
	) -> String {
		let _ = toolchain;
		self.generate_build_file(language, project_name)
	}
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

	fn generate_build_file_with_toolchain(
		&self,
		language: Language,
		project_name: &str,
		toolchain: &crate::toolchain::Toolchain,
	) -> String {
		language.generate_makefile_content_with(project_name, Some(toolchain))
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
		self.generate_build_file_with_toolchain(language, project_name, &Default::default())
	}

	fn generate_build_file_with_toolchain(
		&self,
		language: Language,
		project_name: &str,
		toolchain: &crate::toolchain::Toolchain,
	) -> String {
		let project_name = crate::project_build_name(project_name);
		// Only explicit overrides are pinned; otherwise CMake picks the
		// platform toolchain itself.
		let (lang, compiler_var, compiler) = match language {
			Language::C => ("C", "CMAKE_C_COMPILER", toolchain.cc.as_deref()),
			Language::Cpp => ("CXX", "CMAKE_CXX_COMPILER", toolchain.cxx.as_deref()),
		};
		let compiler_prelude = compiler.map_or_else(String::new, |name| {
			format!("set({compiler_var} \"{name}\")\n")
		});
		let warning_flags = match language {
			Language::C => "CMAKE_C_FLAGS",
			Language::Cpp => "CMAKE_CXX_FLAGS",
		};
		let flags_guard = format!(
			"if(NOT MSVC)\n\tset({warning_flags} \"${{{warning_flags}}} -Wall -Wextra -Werror\")\nendif()\n"
		);
		match language {
			Language::C => format!(
				"cmake_minimum_required(VERSION 3.21)\n\
				{compiler_prelude}\
				project({project_name} {lang})\n\
				\n\
				set(CMAKE_C_STANDARD 11)\n\
				set(CMAKE_C_STANDARD_REQUIRED ON)\n\
				{flags_guard}\
				set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${{CMAKE_SOURCE_DIR}}/bin)\n\
				\n\
				file(GLOB_RECURSE SOURCES CONFIGURE_DEPENDS \"src/*.c\")\n\
				\n\
				add_executable(${{PROJECT_NAME}} ${{SOURCES}})\n\
				target_include_directories(${{PROJECT_NAME}} PRIVATE \"${{CMAKE_CURRENT_SOURCE_DIR}}/include\")\n\
				\n\
				# Optional: Installation\n\
				install(TARGETS ${{PROJECT_NAME}} DESTINATION bin)\n"
			),
			Language::Cpp => format!(
				"cmake_minimum_required(VERSION 3.21)\n\
				{compiler_prelude}\
				project({project_name} {lang})\n\
				\n\
				set(CMAKE_CXX_STANDARD 17)\n\
				set(CMAKE_CXX_STANDARD_REQUIRED ON)\n\
				{flags_guard}\
				set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${{CMAKE_SOURCE_DIR}}/bin)\n\
				\n\
				file(GLOB_RECURSE SOURCES CONFIGURE_DEPENDS \"src/*.cpp\" \"src/*.cc\" \"src/*.cxx\" \"src/*.c++\" \"src/*.C\" \"src/*.CPP\" \"src/*.CC\" \"src/*.CXX\")\n\
				\n\
				add_executable(${{PROJECT_NAME}} ${{SOURCES}})\n\
				target_include_directories(${{PROJECT_NAME}} PRIVATE \"${{CMAKE_CURRENT_SOURCE_DIR}}/include\")\n\
				\n\
				# Optional: Installation\n\
				install(TARGETS ${{PROJECT_NAME}} DESTINATION bin)\n"
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
set -e\n\
\n\
project_name=\"$(basename \"$PWD\")\"\n\
mkdir -p build bin\n\
cd build\n\
cmake -DCMAKE_BUILD_TYPE=Release ..\n\
cmake --build .\n\
echo \"Build complete. Run ./bin/$project_name to execute.\"\n"
}

pub fn generate_cmake_debug_script() -> &'static str {
	"#!/bin/bash\n\
set -e\n\
\n\
project_name=\"$(basename \"$PWD\")\"\n\
mkdir -p build-debug bin\n\
cd build-debug\n\
cmake -DCMAKE_BUILD_TYPE=Debug ..\n\
cmake --build .\n\
echo \"Debug build complete. Run ./bin/$project_name to execute.\"\n"
}
