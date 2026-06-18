use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::str::FromStr;

/// Represents static analysis tool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticAnalysisTool {
	ClangTidy,
	Cppcheck,
}

impl FromStr for StaticAnalysisTool {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self> {
		match s.to_lowercase().as_str() {
			"clang-tidy" | "clangtidy" | "tidy" => Ok(StaticAnalysisTool::ClangTidy),
			"cppcheck" => Ok(StaticAnalysisTool::Cppcheck),
			_ => bail!(
				"Invalid static analysis tool: {}. Valid options: clang-tidy, cppcheck",
				s
			),
		}
	}
}

impl StaticAnalysisTool {
	pub fn as_str(&self) -> &'static str {
		match self {
			StaticAnalysisTool::ClangTidy => "clang-tidy",
			StaticAnalysisTool::Cppcheck => "cppcheck",
		}
	}
}

/// Static analysis configuration generator
pub struct StaticAnalysisGenerator {
	pub tool: StaticAnalysisTool,
}

impl StaticAnalysisGenerator {
	pub fn new(tool: StaticAnalysisTool) -> Self {
		StaticAnalysisGenerator { tool }
	}

	/// Generate .clang-tidy configuration
	fn generate_clang_tidy_config(&self) -> String {
		r#"---
Checks: >
  -*,
  bugprone-*,
  -bugprone-narrowing-conversions,
  clang-analyzer-*,
  modernize-*,
  -modernize-use-trailing-return-type,
  performance-*,
  portability-*,
  readability-*,
  -readability-magic-numbers,
  -readability-identifier-length
WarningsAsErrors: ''
HeaderFilterRegex: ''
FormatStyle: file
"#
		.to_string()
	}

	/// Generate cppcheck configuration
	fn generate_cppcheck_config(&self) -> String {
		r#"<?xml version="1.0"?>
<cppcheck>
  <suppressions>
    <suppression>
      <id>missingIncludeSystem</id>
    </suppression>
  </suppressions>
</cppcheck>
"#
		.to_string()
	}

	/// Generate configuration file
	pub fn generate_config(&self) -> String {
		match self.tool {
			StaticAnalysisTool::ClangTidy => self.generate_clang_tidy_config(),
			StaticAnalysisTool::Cppcheck => self.generate_cppcheck_config(),
		}
	}

	/// Write configuration to file
	pub fn write_config(&self) -> Result<()> {
		let content = self.generate_config();

		match self.tool {
			StaticAnalysisTool::ClangTidy => {
				fs::write(".clang-tidy", content).context("Failed to write .clang-tidy")?;
			}
			StaticAnalysisTool::Cppcheck => {
				fs::write("cppcheck.xml", content).context("Failed to write cppcheck.xml")?;
			}
		}

		Ok(())
	}

	/// Add static analysis target to CMakeLists.txt
	pub fn add_to_cmake(&self, cmake_path: &Path) -> Result<()> {
		let mut content =
			fs::read_to_string(cmake_path).context("Failed to read CMakeLists.txt")?;

		let analysis_target = match self.tool {
			StaticAnalysisTool::ClangTidy => {
				r#"
find_program(CLANG_TIDY clang-tidy)
if(CLANG_TIDY)
    set_target_properties(${PROJECT_NAME} PROPERTIES
        CXX_CLANG_TIDY "${CLANG_TIDY}"
    )
endif()
"#
			}
			StaticAnalysisTool::Cppcheck => {
				r#"
find_program(CPPCHECK cppcheck)
if(CPPCHECK)
    add_custom_target(cppcheck
        COMMAND ${CPPCHECK} --enable=all --inconclusive --xml --xml-version=2 src 2> cppcheck-report.xml
        COMMENT "Running cppcheck"
        VERBATIM
    )
endif()
"#
			}
		};

		if !content.contains("CLANG_TIDY") && !content.contains("CPPCHECK") {
			content.push_str(analysis_target);
		}

		fs::write(cmake_path, content).context("Failed to write CMakeLists.txt")?;
		Ok(())
	}

	/// Add static analysis target to Makefile
	pub fn add_to_makefile(&self, makefile_path: &Path) -> Result<()> {
		let mut content = fs::read_to_string(makefile_path).context("Failed to read Makefile")?;

		let analysis_target = match self.tool {
			StaticAnalysisTool::ClangTidy => {
				r#"
lint:
	clang-tidy src/*.cpp -- -std=c++17
"#
			}
			StaticAnalysisTool::Cppcheck => {
				r#"
lint:
	cppcheck --enable=all src/
"#
			}
		};

		if !content.contains("lint:") {
			content.push_str(analysis_target);
		}

		fs::write(makefile_path, content).context("Failed to write Makefile")?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

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
}
