use crate::file_handler::write_atomic;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::str::FromStr;

/// Represents test framework options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestFramework {
	GoogleTest,
	Catch2,
	Doctest,
}

impl FromStr for TestFramework {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self> {
		match s.to_lowercase().as_str() {
			"gtest" | "googletest" | "google-test" => Ok(TestFramework::GoogleTest),
			"catch2" | "catch" => Ok(TestFramework::Catch2),
			"doctest" => Ok(TestFramework::Doctest),
			_ => bail!(
				"Invalid test framework: {}. Valid options: gtest, catch2, doctest",
				s
			),
		}
	}
}

impl TestFramework {
	pub fn as_str(&self) -> &'static str {
		match self {
			TestFramework::GoogleTest => "GoogleTest",
			TestFramework::Catch2 => "Catch2",
			TestFramework::Doctest => "Doctest",
		}
	}

	pub fn cmake_find_package(&self) -> &'static str {
		match self {
			TestFramework::GoogleTest => "GTest",
			TestFramework::Catch2 => "Catch2",
			TestFramework::Doctest => "doctest",
		}
	}
}

/// Test framework manager
pub struct TestFrameworkManager {
	pub framework: TestFramework,
}

/// Inserts `block` on its own line directly after the closing paren of the
/// `project()` call. Uses the paren-aware scanner so a multiline `project(` is
/// never split. Errors if no `project()` call exists.
fn insert_after_project_call(content: &mut String, block: &str) -> Result<()> {
	let call = crate::cmake_parser::find_calls(content, "project")
		.into_iter()
		.next()
		.ok_or_else(|| {
			anyhow::anyhow!("No project() call found; add the directive to CMakeLists.txt manually")
		})?;
	if !content.is_empty() && !content.ends_with('\n') {
		content.push('\n');
	}
	let insert_pos = match content[call.end_byte..].find('\n') {
		Some(offset) => call.end_byte + offset + 1,
		// No trailing newline: append on a fresh line.
		None => content.len(),
	};
	content.insert_str(insert_pos, block);
	Ok(())
}

impl TestFrameworkManager {
	pub fn new(framework: TestFramework) -> Self {
		TestFrameworkManager { framework }
	}

	/// Add test framework to CMakeLists.txt
	pub fn add_to_cmake(&self, cmake_path: &Path, project_name: &str) -> Result<()> {
		let mut content =
			fs::read_to_string(cmake_path).context("Failed to read CMakeLists.txt")?;
		ensure_cmake_test_framework_compatible(&content, self.framework, project_name, cmake_path)?;

		let project_name = crate::project_build_name(project_name);
		let package = self.framework.cmake_find_package();
		let version = if self.framework == TestFramework::Catch2 {
			" 3"
		} else {
			""
		};

		// Add find_package, unless the package is already requested (with or
		// without a pinned version). Reuse the paren-aware scanner so a
		// multiline `project(` or an existing call is never mangled.
		let already_declared = crate::cmake_parser::find_calls(&content, "find_package")
			.into_iter()
			.any(|call| {
				call.args
					.first()
					.is_some_and(|arg| arg.eq_ignore_ascii_case(package))
			});
		if !already_declared {
			let find_package = format!("find_package({}{} REQUIRED)\n", package, version);
			insert_after_project_call(&mut content, &find_package)?;
		}

		// Add enable_testing
		let has_enable_testing = crate::cmake_parser::find_calls(&content, "enable_testing")
			.into_iter()
			.any(|call| call.args.is_empty());
		if !has_enable_testing {
			insert_after_project_call(&mut content, "enable_testing()\n")?;
		}

		// Add test executable
		let test_link = match self.framework {
			TestFramework::GoogleTest => "GTest::gtest",
			TestFramework::Catch2 => "Catch2::Catch2WithMain",
			TestFramework::Doctest => "doctest::doctest",
		};
		let test_content = format!(
			"add_executable({0}_test tests/test_{0}.cpp)\n\
			 target_link_libraries({0}_test {1})\n\
			 add_test(NAME {0} COMMAND {0}_test)\n",
			project_name, test_link
		);

		if !content.contains(&format!("{}_test", project_name)) {
			content.push_str(&test_content);
		}

		write_atomic(cmake_path, &content).context("Failed to write CMakeLists.txt")?;
		Ok(())
	}

	/// Add test framework to Makefile
	pub fn add_to_makefile(&self, makefile_path: &Path, project_name: &str) -> Result<()> {
		let mut content = fs::read_to_string(makefile_path).context("Failed to read Makefile")?;
		ensure_make_test_framework_compatible(
			&content,
			self.framework,
			project_name,
			makefile_path,
		)?;

		// Add test target
		let test_content = match self.framework {
			TestFramework::GoogleTest => format!(
				r#"
test: {0}_test
	./{0}_test

{0}_test: tests/test_{0}.cpp
	$(CXX) $(CXXFLAGS) $(CPPFLAGS) -c tests/test_{0}.cpp -o test_{0}.o
	$(CXX) test_{0}.o -o {0}_test $(LDFLAGS) $(LDLIBS) -lgtest -lgtest_main -lpthread
"#,
				project_name
			),
			TestFramework::Catch2 => format!(
				r#"
test: {0}_test
	./{0}_test

{0}_test: tests/test_{0}.cpp
	$(CXX) $(CXXFLAGS) $(CPPFLAGS) -c tests/test_{0}.cpp -o test_{0}.o
	$(CXX) test_{0}.o -o {0}_test $(LDFLAGS) $(LDLIBS) -lCatch2WithMain
"#,
				project_name
			),
			TestFramework::Doctest => format!(
				r#"
test: {0}_test
	./{0}_test

{0}_test: tests/test_{0}.cpp
	$(CXX) $(CXXFLAGS) $(CPPFLAGS) -c tests/test_{0}.cpp -o test_{0}.o
	$(CXX) test_{0}.o -o {0}_test $(LDFLAGS) $(LDLIBS) -ldoctest
"#,
				project_name
			),
		};

		let has_test_target = content
			.lines()
			.any(|line| line.trim_start().starts_with("test:"));
		if !has_test_target
			&& !content.lines().any(|line| {
				line.trim_start()
					.starts_with(&format!("{}_test:", project_name))
			}) {
			content.push_str(&test_content);
			if let Some(position) = content.find(".PHONY:") {
				let line_end = content[position..]
					.find('\n')
					.map(|offset| position + offset)
					.unwrap_or(content.len());
				content.insert_str(line_end, " test");
			} else {
				content.push_str("\n.PHONY: test\n");
			}
		}

		write_atomic(makefile_path, &content).context("Failed to write Makefile")?;
		Ok(())
	}

	/// Generate test file template
	pub fn generate_test_file(&self, project_name: &str, _language: &str) -> String {
		let project_name = crate::project_build_name(project_name);
		match self.framework {
			TestFramework::GoogleTest => format!(
				r#"#include <gtest/gtest.h>

int main(int argc, char **argv) {{
    ::testing::InitGoogleTest(&argc, argv);
    return RUN_ALL_TESTS();
}}

TEST({}Test, BasicTest) {{
    EXPECT_EQ(1, 1);
}}
"#,
				project_name.to_uppercase().replace("-", "_")
			),
			TestFramework::Catch2 => format!(
				r#"#include <catch2/catch_test_macros.hpp>

TEST_CASE("Basic test", "[{}]") {{
    REQUIRE(1 == 1);
}}
"#,
				project_name
			),
			TestFramework::Doctest => r#"#define DOCTEST_CONFIG_IMPLEMENT_WITH_MAIN
#include <doctest/doctest.h>

TEST_CASE("Basic test") {
    CHECK(1 == 1);
}
"#
			.to_string(),
		}
	}

	/// Create test directory and file
	pub fn setup_test_files(&self, project_name: &str, language: &str) -> Result<()> {
		crate::validate_project_name(project_name)?;
		fs::create_dir_all("tests").context("Failed to create tests directory")?;

		let test_file = format!("tests/test_{}.cpp", project_name);
		let test_path = Path::new(&test_file);
		if test_path.exists() {
			ensure_test_file_framework(test_path, self.framework)?;
			return Ok(());
		}
		let test_content = self.generate_test_file(project_name, language);
		write_atomic(test_path, &test_content)
			.with_context(|| format!("Failed to write test file: {}", test_file))?;

		Ok(())
	}
}

fn configured_test_frameworks(content: &str) -> Vec<TestFramework> {
	let content = content.to_ascii_lowercase();
	[
		(
			TestFramework::GoogleTest,
			["find_package(gtest", "gtest::", "-lgtest", "gtest/gtest.h"],
		),
		(
			TestFramework::Catch2,
			["find_package(catch2", "catch2::", "-lcatch2", "catch2/"],
		),
		(
			TestFramework::Doctest,
			["find_package(doctest", "doctest::", "-ldoctest", "doctest/"],
		),
	]
	.into_iter()
	.filter(|(_, markers)| markers.iter().any(|marker| content.contains(*marker)))
	.map(|(framework, _)| framework)
	.collect()
}

fn test_file_framework(path: &Path) -> Option<TestFramework> {
	let content = fs::read_to_string(path).ok()?;
	configured_test_frameworks(&content).into_iter().next()
}

fn ensure_configured_test_framework(content: &str, requested: TestFramework) -> Result<()> {
	let configured = configured_test_frameworks(content);
	if configured.len() > 1 {
		bail!("Multiple test frameworks are already configured");
	}
	if let Some(existing) = configured.first()
		&& *existing != requested
	{
		bail!(
			"Cannot add {}: {} is already configured",
			requested.as_str(),
			existing.as_str()
		);
	}
	Ok(())
}

fn ensure_test_file_framework(path: &Path, requested: TestFramework) -> Result<()> {
	match test_file_framework(path) {
		Some(existing) if existing == requested => Ok(()),
		Some(existing) => bail!(
			"Cannot add {}: test file {} already uses {}",
			requested.as_str(),
			path.display(),
			existing.as_str()
		),
		None => bail!(
			"Cannot add {}: test file {} already exists and its framework is not recognized",
			requested.as_str(),
			path.display()
		),
	}
}

fn ensure_cmake_test_framework_compatible(
	content: &str,
	requested: TestFramework,
	project_name: &str,
	cmake_path: &Path,
) -> Result<()> {
	ensure_configured_test_framework(content, requested)?;
	let test_path = cmake_path
		.parent()
		.unwrap_or_else(|| Path::new("."))
		.join(format!("tests/test_{}.cpp", project_name));
	if test_path.is_file()
		&& let Some(existing) = test_file_framework(&test_path)
		&& existing != requested
	{
		bail!(
			"Cannot add {}: test file {} already uses {}",
			requested.as_str(),
			test_path.display(),
			existing.as_str()
		);
	}
	Ok(())
}

fn ensure_make_test_framework_compatible(
	content: &str,
	requested: TestFramework,
	project_name: &str,
	makefile_path: &Path,
) -> Result<()> {
	ensure_configured_test_framework(content, requested)?;
	let test_path = makefile_path
		.parent()
		.unwrap_or_else(|| Path::new("."))
		.join(format!("tests/test_{}.cpp", project_name));
	if test_path.is_file()
		&& let Some(existing) = test_file_framework(&test_path)
		&& existing != requested
	{
		bail!(
			"Cannot add {}: test file {} already uses {}",
			requested.as_str(),
			test_path.display(),
			existing.as_str()
		);
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn inserts_after_multiline_project_call() {
		let mut content = String::from(
			"cmake_minimum_required(VERSION 3.10)\nproject(\n  demo\n  LANGUAGES CXX\n)\nadd_executable(app main.cpp)\n",
		);
		insert_after_project_call(&mut content, "enable_testing()\n").unwrap();
		assert!(
			content
				.contains("project(\n  demo\n  LANGUAGES CXX\n)\nenable_testing()\nadd_executable")
		);
	}

	#[test]
	fn inserts_after_single_line_project_call() {
		let mut content =
			String::from("project(demo LANGUAGES CXX)\nadd_executable(app main.cpp)\n");
		insert_after_project_call(&mut content, "enable_testing()\n").unwrap();
		assert!(content.contains("project(demo LANGUAGES CXX)\nenable_testing()\n"));
	}

	#[test]
	fn errors_when_no_project_call() {
		let mut content = String::from("add_executable(app main.cpp)\n");
		assert!(insert_after_project_call(&mut content, "enable_testing()\n").is_err());
	}

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
		assert!(TestFramework::from_str("invalid").is_err());
	}

	#[test]
	fn test_test_framework_strings() {
		assert_eq!(TestFramework::GoogleTest.as_str(), "GoogleTest");
		assert_eq!(TestFramework::Catch2.cmake_find_package(), "Catch2");
	}
}
