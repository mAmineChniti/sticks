use crate::file_handler::{ensure_path_available, write_atomic};
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticAnalysisTool {
	ClangTidy,
	Cppcheck,
}

impl FromStr for StaticAnalysisTool {
	type Err = anyhow::Error;

	fn from_str(value: &str) -> Result<Self> {
		match value.to_lowercase().as_str() {
			"clang-tidy" | "clangtidy" | "tidy" => Ok(Self::ClangTidy),
			"cppcheck" => Ok(Self::Cppcheck),
			_ => bail!(
				"Invalid static analysis tool: {}. Valid options: clang-tidy, cppcheck",
				value
			),
		}
	}
}

impl StaticAnalysisTool {
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::ClangTidy => "clang-tidy",
			Self::Cppcheck => "cppcheck",
		}
	}
}

pub struct StaticAnalysisGenerator {
	pub tool: StaticAnalysisTool,
}

impl StaticAnalysisGenerator {
	pub fn new(tool: StaticAnalysisTool) -> Self {
		Self { tool }
	}

	pub fn generate_config(&self) -> String {
		match self.tool {
			StaticAnalysisTool::ClangTidy => r#"---
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
			.to_string(),
			StaticAnalysisTool::Cppcheck => "missingIncludeSystem\n".to_string(),
		}
	}

	pub fn write_config(&self) -> Result<()> {
		let path = match self.tool {
			StaticAnalysisTool::ClangTidy => Path::new(".clang-tidy"),
			StaticAnalysisTool::Cppcheck => Path::new("cppcheck.xml"),
		};
		ensure_path_available(path)?;
		write_atomic(path, &self.generate_config())
			.context("Failed to write static analysis configuration")
	}

	pub fn add_to_cmake(&self, cmake_path: &Path) -> Result<()> {
		let mut content =
			fs::read_to_string(cmake_path).context("Failed to read CMakeLists.txt")?;
		let configured = match self.tool {
			StaticAnalysisTool::ClangTidy => {
				content.contains("CMAKE_C_CLANG_TIDY") || content.contains("C_CLANG_TIDY")
			}
			StaticAnalysisTool::Cppcheck => {
				content.contains("CPPCHECK") || content.contains("add_custom_target(cppcheck")
			}
		};
		if !configured {
			let target = match self.tool {
				StaticAnalysisTool::ClangTidy => {
					"\nfind_program(CLANG_TIDY clang-tidy)\nif(CLANG_TIDY)\n    set(CMAKE_EXPORT_COMPILE_COMMANDS ON)\n    set(CMAKE_C_CLANG_TIDY \"${CLANG_TIDY}\")\n    set(CMAKE_CXX_CLANG_TIDY \"${CLANG_TIDY}\")\nendif()\n"
				}
				StaticAnalysisTool::Cppcheck => {
					"\nfind_program(CPPCHECK cppcheck)\nif(CPPCHECK)\n    add_custom_target(cppcheck\n        COMMAND ${CPPCHECK} --enable=all --inconclusive --suppressions-list=${CMAKE_CURRENT_SOURCE_DIR}/cppcheck.xml src\n        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}\n        COMMENT \"Running cppcheck\"\n        VERBATIM\n    )\nendif()\n"
				}
			};
			if matches!(self.tool, StaticAnalysisTool::ClangTidy) {
				insert_before_targets(&mut content, target);
			} else {
				content.push_str(target);
			}
		}
		write_atomic(cmake_path, &content).context("Failed to write CMakeLists.txt")
	}

	pub fn add_to_makefile(&self, makefile_path: &Path) -> Result<()> {
		let mut content = fs::read_to_string(makefile_path).context("Failed to read Makefile")?;
		let target = match self.tool {
			StaticAnalysisTool::ClangTidy => "lint-clang-tidy",
			StaticAnalysisTool::Cppcheck => "lint-cppcheck",
		};
		if !content.contains(&format!("{}:", target)) {
			let rule = match self.tool {
				StaticAnalysisTool::ClangTidy => {
					"\nlint: lint-clang-tidy\nlint-clang-tidy:\n\tfind src -type f \\( -iname '*.c' -o -iname '*.cpp' -o -iname '*.cc' -o -iname '*.cxx' -o -iname '*.c++' \\) -exec clang-tidy {} -- $(CPPFLAGS) $(CXXFLAGS) $(CFLAGS) +\n"
				}
				StaticAnalysisTool::Cppcheck => {
					"\nlint: lint-cppcheck\nlint-cppcheck:\n\tcppcheck --enable=all --inconclusive --suppressions-list=cppcheck.xml src\n"
				}
			};
			content.push_str(rule);
			if let Some(position) = content.find(".PHONY:") {
				let line_end = content[position..]
					.find('\n')
					.map(|offset| position + offset)
					.unwrap_or(content.len());
				content.insert_str(line_end, &format!(" {}", target));
			} else {
				content.push_str(&format!("\n.PHONY: {}\n", target));
			}
		}
		write_atomic(makefile_path, &content).context("Failed to write Makefile")
	}
}

fn insert_before_targets(content: &mut String, block: &str) {
	let target_position = ["add_executable(", "add_library("]
		.iter()
		.filter_map(|needle| content.find(needle))
		.min();
	let insertion = target_position.or_else(|| {
		content.find("project(").and_then(|position| {
			content[position..]
				.find('\n')
				.map(|offset| position + offset + 1)
		})
	});
	if let Some(position) = insertion {
		content.insert_str(position, block);
	} else {
		content.push_str(block);
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
}
