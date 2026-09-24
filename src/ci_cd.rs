use crate::build_systems::BuildSystem;
use crate::file_handler::{ensure_path_available, write_atomic};
use crate::test_framework::TestFramework;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiPlatform {
	GitHubActions,
	GitLabCI,
}

impl FromStr for CiPlatform {
	type Err = anyhow::Error;

	fn from_str(value: &str) -> Result<Self> {
		match value.to_lowercase().as_str() {
			"github" | "github-actions" | "gha" => Ok(Self::GitHubActions),
			"gitlab" | "gitlab-ci" | "gl" => Ok(Self::GitLabCI),
			_ => bail!(
				"Invalid CI platform: {}. Valid options: github, gitlab",
				value
			),
		}
	}
}

impl CiPlatform {
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::GitHubActions => "GitHub Actions",
			Self::GitLabCI => "GitLab CI",
		}
	}
}

pub struct CiCdGenerator {
	pub platform: CiPlatform,
}

impl CiCdGenerator {
	pub fn new(platform: CiPlatform) -> Self {
		Self { platform }
	}

	pub fn generate(&self, project_name: &str, language: &str) -> String {
		let build_system = detect_build_system();
		self.generate_for(project_name, language, build_system)
	}

	pub fn generate_for(
		&self,
		_project_name: &str,
		language: &str,
		build_system: BuildSystem,
	) -> String {
		let package_setup = package_manager_setup();
		let cmake_configure = cmake_configure_command();
		let test_configuration = detect_test_configuration(build_system);
		let test_install = test_framework_packages(&test_configuration.frameworks);
		let github_test_install = github_test_install_step(&test_install);
		let github_test_step = github_test_step(build_system, &test_configuration);
		let gitlab_test_install = gitlab_test_install_step(&test_install);
		let gitlab_test_step = gitlab_test_step(build_system, &test_configuration);
		let clang_tidy = clang_tidy_command(build_system, language);
		match (self.platform, build_system) {
			(CiPlatform::GitHubActions, BuildSystem::CMake) => format!(
				r#"name: CI

on:
  push:
    branches: [ master, main ]
  pull_request:
    branches: [ master, main ]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - name: Install build tools
      run: sudo apt-get update && sudo apt-get install -y cmake make gcc g++ clang-tidy python3-venv
{github_test_install}    - name: Install project dependencies
      run: |
        {package_setup}
    - name: Configure
      run: {cmake_configure}
    - name: Build
      run: cmake --build build
{github_test_step}  lint:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - name: Install clang-tidy
      run: sudo apt-get update && sudo apt-get install -y clang-tidy
{github_test_install}    - name: Configure build context
      run: |
        {package_setup}
        {cmake_configure}
    - name: Run clang-tidy
      run: |
        {clang_tidy}
"#,
				package_setup = package_setup,
				cmake_configure = cmake_configure,
				github_test_install = github_test_install,
				github_test_step = github_test_step,
				clang_tidy = clang_tidy
			),
			(CiPlatform::GitHubActions, BuildSystem::Makefile) => format!(
				r#"name: CI

on:
  push:
    branches: [ master, main ]
  pull_request:
    branches: [ master, main ]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - name: Install build tools
      run: sudo apt-get update && sudo apt-get install -y make gcc g++ python3-venv
{github_test_install}    - name: Install project dependencies
      run: |
        {package_setup}
    - name: Build
      run: |
        if make -n install-deps >/dev/null 2>&1; then make install-deps; fi
        make
{github_test_step}  lint:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - name: Install clang-tidy
      run: sudo apt-get update && sudo apt-get install -y clang-tidy
    - name: Run clang-tidy
      run: |
        {clang_tidy}
"#,
				package_setup = package_setup,
				github_test_install = github_test_install,
				github_test_step = github_test_step,
				clang_tidy = clang_tidy
			),
			(CiPlatform::GitLabCI, BuildSystem::CMake) => format!(
				r#"image: ubuntu:latest

stages:
  - build
  - lint

before_script:
  - apt-get update && apt-get install -y cmake make gcc g++ clang-tidy python3-venv

build:
  stage: build
  script:
    - |
      {package_setup}
{gitlab_test_install}    - {cmake_configure}
    - cmake --build build
{gitlab_test_step}  artifacts:
    paths:
      - build/

lint:
  stage: lint
  script:
    - |
      {package_setup}
{gitlab_test_install}    - {cmake_configure}
    - |
      {clang_tidy}
"#,
				package_setup = package_setup,
				cmake_configure = cmake_configure,
				gitlab_test_install = gitlab_test_install,
				gitlab_test_step = gitlab_test_step,
				clang_tidy = clang_tidy
			),
			(CiPlatform::GitLabCI, BuildSystem::Makefile) => format!(
				r#"image: ubuntu:latest

stages:
  - build
  - lint

before_script:
  - apt-get update && apt-get install -y make gcc g++ python3-venv clang-tidy

build:
  stage: build
  script:
    - |
      {package_setup}
{gitlab_test_install}    - if make -n install-deps >/dev/null 2>&1; then make install-deps; fi
    - make
{gitlab_test_step}

lint:
  stage: lint
  script:
    - |
      {clang_tidy}
"#,
				package_setup = package_setup,
				gitlab_test_install = gitlab_test_install,
				gitlab_test_step = gitlab_test_step,
				clang_tidy = clang_tidy
			),
		}
	}

	pub fn write_to_file(&self, project_name: &str, language: &str) -> Result<()> {
		self.write_to_file_for(project_name, language, detect_build_system())
	}

	pub fn write_to_file_for(
		&self,
		project_name: &str,
		language: &str,
		build_system: BuildSystem,
	) -> Result<()> {
		let content = self.generate_for(project_name, language, build_system);
		match self.platform {
			CiPlatform::GitHubActions => {
				ensure_path_available(Path::new(".github/workflows/ci.yml"))?;
				fs::create_dir_all(".github/workflows")
					.context("Failed to create .github/workflows directory")?;
				write_atomic(Path::new(".github/workflows/ci.yml"), &content)
					.context("Failed to write GitHub Actions workflow")?;
			}
			CiPlatform::GitLabCI => {
				ensure_path_available(Path::new(".gitlab-ci.yml"))?;
				write_atomic(Path::new(".gitlab-ci.yml"), &content)
					.context("Failed to write GitLab CI configuration")?;
			}
		}
		Ok(())
	}
}

fn detect_build_system() -> BuildSystem {
	crate::detect_build_system()
		.ok()
		.flatten()
		.unwrap_or(BuildSystem::CMake)
}

fn package_manager_setup() -> String {
	if Path::new("vcpkg.json").is_file() {
		"if [ ! -d vcpkg ]; then git clone https://github.com/microsoft/vcpkg.git vcpkg; fi; ./vcpkg/bootstrap-vcpkg.sh; arch=\"$(uname -m)\"; if [ -n \"${VCPKG_TRIPLET:-}\" ]; then triplet=\"$VCPKG_TRIPLET\"; elif [ \"$arch\" = x86_64 ]; then triplet=x64-linux; elif [ \"$arch\" = aarch64 ]; then triplet=arm64-linux; elif [ \"$arch\" = armv7l ]; then triplet=arm-linux; else triplet=\"$arch-linux\"; fi; ./vcpkg/vcpkg install --triplet \"$triplet\""
			.to_string()
	} else if Path::new("conanfile.txt").is_file() || Path::new("conanfile.py").is_file() {
		"python3 -m venv .conan-venv && .conan-venv/bin/python -m pip install --upgrade pip conan && .conan-venv/bin/conan profile detect --force && .conan-venv/bin/conan install . --output-folder=build --build=missing"
			.to_string()
	} else {
		"true".to_string()
	}
}

fn cmake_configure_command() -> String {
	let command = if Path::new("conanfile.txt").is_file() || Path::new("conanfile.py").is_file() {
		"cmake -S . -B build -DCMAKE_TOOLCHAIN_FILE=build/conan_toolchain.cmake".to_string()
	} else if Path::new("vcpkg.json").is_file() {
		"cmake -S . -B build -DCMAKE_TOOLCHAIN_FILE=vcpkg/scripts/buildsystems/vcpkg.cmake"
			.to_string()
	} else {
		"cmake -S . -B build".to_string()
	};
	format!("{command} -DCMAKE_EXPORT_COMPILE_COMMANDS=ON")
}

#[derive(Default)]
struct TestConfiguration {
	frameworks: Vec<TestFramework>,
	has_tests: bool,
}

fn detect_test_configuration(build_system: BuildSystem) -> TestConfiguration {
	let build_content = match build_system {
		BuildSystem::CMake => fs::read_to_string(Path::new("CMakeLists.txt")).unwrap_or_default(),
		BuildSystem::Makefile => ["Makefile", "makefile", "GNUmakefile"]
			.iter()
			.find_map(|path| fs::read_to_string(*path).ok())
			.unwrap_or_default(),
	};
	let mut test_sources = Vec::new();
	collect_test_source_contents(Path::new("tests"), &mut test_sources);

	let mut configuration = TestConfiguration {
		has_tests: !test_sources.is_empty()
			|| build_content.contains("enable_testing()")
			|| build_content.contains("add_test(")
			|| make_target_exists(&build_content, "test"),
		frameworks: Vec::new(),
	};
	let contents =
		std::iter::once(build_content.as_str()).chain(test_sources.iter().map(String::as_str));
	for content in contents {
		let content = content.to_ascii_lowercase();
		for (framework, markers) in [
			(
				TestFramework::GoogleTest,
				&["find_package(gtest", "gtest::", "-lgtest", "gtest/gtest.h"][..],
			),
			(
				TestFramework::Catch2,
				&["find_package(catch2", "catch2::", "-lcatch2", "catch2/"][..],
			),
			(
				TestFramework::Doctest,
				&["find_package(doctest", "doctest::", "-ldoctest", "doctest/"][..],
			),
		] {
			if markers.iter().any(|marker| content.contains(*marker))
				&& !configuration.frameworks.contains(&framework)
			{
				configuration.frameworks.push(framework);
			}
		}
	}
	if !configuration.frameworks.is_empty() {
		configuration.has_tests = true;
	}
	configuration
}

fn collect_test_source_contents(directory: &Path, contents: &mut Vec<String>) {
	let Ok(entries) = fs::read_dir(directory) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		if path.is_dir() {
			collect_test_source_contents(&path, contents);
		} else if path
			.extension()
			.and_then(|extension| extension.to_str())
			.is_some_and(|extension| matches!(extension, "cpp" | "cc" | "cxx" | "c++"))
			&& let Ok(content) = fs::read_to_string(path)
		{
			contents.push(content);
		}
	}
}

fn make_target_exists(content: &str, target: &str) -> bool {
	let prefix = format!("{target}:");
	content
		.lines()
		.any(|line| line.trim_start().starts_with(&prefix))
}

fn test_framework_packages(frameworks: &[TestFramework]) -> Vec<&'static str> {
	let mut packages = Vec::new();
	for framework in frameworks {
		let package = match framework {
			TestFramework::GoogleTest => "libgtest-dev",
			TestFramework::Catch2 => "catch2",
			TestFramework::Doctest => "doctest-dev",
		};
		if !packages.contains(&package) {
			packages.push(package);
		}
	}
	packages
}

fn github_test_install_step(packages: &[&str]) -> String {
	if packages.is_empty() {
		String::new()
	} else {
		format!(
			"    - name: Install test framework\n      run: sudo apt-get install -y {}\n",
			packages.join(" ")
		)
	}
}

fn github_test_step(build_system: BuildSystem, configuration: &TestConfiguration) -> String {
	if !configuration.has_tests {
		return String::new();
	}
	let command = match build_system {
		BuildSystem::CMake => "ctest --test-dir build --no-tests=error --output-on-failure",
		BuildSystem::Makefile => "make test",
	};
	format!("    - name: Test\n      run: {command}\n")
}

fn gitlab_test_install_step(packages: &[&str]) -> String {
	if packages.is_empty() {
		String::new()
	} else {
		format!("    - sudo apt-get install -y {}\n", packages.join(" "))
	}
}

fn gitlab_test_step(build_system: BuildSystem, configuration: &TestConfiguration) -> String {
	if !configuration.has_tests {
		return String::new();
	}
	match build_system {
		BuildSystem::CMake => {
			"    - ctest --test-dir build --no-tests=error --output-on-failure\n".to_string()
		}
		BuildSystem::Makefile => "    - make test\n".to_string(),
	}
}

fn clang_tidy_command(build_system: BuildSystem, language: &str) -> String {
	let files = r"find src -type f \( -iname '*.c' -o -iname '*.cpp' -o -iname '*.cc' -o -iname '*.cxx' -o -iname '*.c++' \)";
	let standard = if language.eq_ignore_ascii_case("cpp") || language.eq_ignore_ascii_case("c++") {
		"-std=c++17"
	} else {
		"-std=c11"
	};
	match build_system {
		BuildSystem::CMake => format!(
			"if [ -f build/compile_commands.json ]; then {files} -exec clang-tidy -p build {{}} +; else {files} -exec clang-tidy {{}} -- -Iinclude {standard} +; fi"
		),
		BuildSystem::Makefile => format!(
			"if make -n lint-clang-tidy >/dev/null 2>&1; then make lint-clang-tidy; else {files} -exec clang-tidy {{}} -- -Iinclude {standard} +; fi"
		),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_ci_platform_from_str() {
		assert_eq!(
			CiPlatform::from_str("github").unwrap(),
			CiPlatform::GitHubActions
		);
		assert_eq!(
			CiPlatform::from_str("gitlab").unwrap(),
			CiPlatform::GitLabCI
		);
	}
}
