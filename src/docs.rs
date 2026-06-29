use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::str::FromStr;

/// Represents documentation tool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocTool {
	Doxygen,
	Sphinx,
}

impl FromStr for DocTool {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self> {
		match s.to_lowercase().as_str() {
			"doxygen" => Ok(DocTool::Doxygen),
			"sphinx" => Ok(DocTool::Sphinx),
			_ => bail!(
				"Invalid documentation tool: {}. Valid options: doxygen, sphinx",
				s
			),
		}
	}
}

impl DocTool {
	pub fn as_str(&self) -> &'static str {
		match self {
			DocTool::Doxygen => "Doxygen",
			DocTool::Sphinx => "Sphinx",
		}
	}
}

/// Documentation generator
pub struct DocGenerator {
	pub tool: DocTool,
}

impl DocGenerator {
	pub fn new(tool: DocTool) -> Self {
		DocGenerator { tool }
	}

	/// Generate Doxyfile configuration
	fn generate_doxyfile(&self, project_name: &str) -> String {
		format!(
			r#"# Doxyfile for {}

PROJECT_NAME = "{}"
PROJECT_NUMBER = 1.0
OUTPUT_DIRECTORY = docs
INPUT = src
RECURSIVE = YES
EXTRACT_ALL = YES
GENERATE_HTML = YES
GENERATE_LATEX = NO
HAVE_DOT = YES
CALL_GRAPH = YES
CALLER_GRAPH = YES
"#,
			project_name, project_name
		)
	}

	/// Generate Sphinx configuration
	fn generate_sphinx_conf(&self, project_name: &str) -> String {
		format!(
			r#"# Sphinx configuration for {}

project = '{}'
copyright = '2025'
author = 'Author'

extensions = []

html_theme = 'alabaster'
"#,
			project_name, project_name
		)
	}

	/// Generate documentation configuration file
	pub fn generate_config(&self, project_name: &str) -> String {
		match self.tool {
			DocTool::Doxygen => self.generate_doxyfile(project_name),
			DocTool::Sphinx => self.generate_sphinx_conf(project_name),
		}
	}

	/// Write documentation configuration to file
	pub fn write_config(&self, project_name: &str) -> Result<()> {
		let content = self.generate_config(project_name);

		match self.tool {
			DocTool::Doxygen => {
				fs::write("Doxyfile", content).context("Failed to write Doxyfile")?;
			}
			DocTool::Sphinx => {
				fs::create_dir_all("docs").context("Failed to create docs directory")?;
				fs::write("docs/conf.py", content).context("Failed to write Sphinx conf")?;
				fs::write(
					"docs/index.rst",
					format!(
						r#"Welcome to {}'s documentation!
==================================

.. toctree::
   :maxdepth: 2
   :caption: Contents:

API Documentation
================

.. doxygenindex::
   :project: {}
"#,
						project_name, project_name
					),
				)
				.context("Failed to write Sphinx index")?;
			}
		}

		Ok(())
	}

	/// Add documentation target to CMakeLists.txt
	pub fn add_to_cmake(&self, cmake_path: &Path, _project_name: &str) -> Result<()> {
		let mut content =
			fs::read_to_string(cmake_path).context("Failed to read CMakeLists.txt")?;

		let doc_target = match self.tool {
			DocTool::Doxygen => r#"
find_package(Doxygen)
if(DOXYGEN_FOUND)
    add_custom_target(docs
        COMMAND doxygen Doxyfile
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
        COMMENT "Generating API documentation with Doxygen"
        VERBATIM
    )
endif()
"#
			.to_string(),
			DocTool::Sphinx => r#"
find_package(Sphinx)
if(SPHINX_FOUND)
    add_custom_target(docs
        COMMAND sphinx-build -b html docs docs/_build/html
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
        COMMENT "Generating documentation with Sphinx"
        VERBATIM
    )
endif()
"#
			.to_string(),
		};

		if !content.contains("add_custom_target(docs") {
			content.push_str(&doc_target);
		}

		fs::write(cmake_path, content).context("Failed to write CMakeLists.txt")?;
		Ok(())
	}

	/// Add documentation target to Makefile
	pub fn add_to_makefile(&self, makefile_path: &Path) -> Result<()> {
		let mut content = fs::read_to_string(makefile_path).context("Failed to read Makefile")?;

		let doc_target = match self.tool {
			DocTool::Doxygen => {
				r#"
docs:
	doxygen Doxyfile
"#
			}
			DocTool::Sphinx => {
				r#"
docs:
	sphinx-build -b html docs docs/_build/html
"#
			}
		};

		if !content.contains("docs:") {
			content.push_str(doc_target);
		}

		fs::write(makefile_path, content).context("Failed to write Makefile")?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

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
}
