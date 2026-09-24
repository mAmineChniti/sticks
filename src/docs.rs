use crate::file_handler::{ensure_path_available, write_atomic};
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocTool {
	Doxygen,
	Sphinx,
}

impl FromStr for DocTool {
	type Err = anyhow::Error;

	fn from_str(value: &str) -> Result<Self> {
		match value.to_lowercase().as_str() {
			"doxygen" => Ok(Self::Doxygen),
			"sphinx" => Ok(Self::Sphinx),
			_ => bail!(
				"Invalid documentation tool: {}. Valid options: doxygen, sphinx",
				value
			),
		}
	}
}

impl DocTool {
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Doxygen => "Doxygen",
			Self::Sphinx => "Sphinx",
		}
	}
}

pub struct DocGenerator {
	pub tool: DocTool,
}

impl DocGenerator {
	pub fn new(tool: DocTool) -> Self {
		Self { tool }
	}

	pub fn generate_config(&self, project_name: &str) -> String {
		let project_name = crate::project_build_name(project_name);
		match self.tool {
			DocTool::Doxygen => format!(
				r#"# Doxyfile for {}

PROJECT_NAME = "{}"
PROJECT_NUMBER = 1.0
OUTPUT_DIRECTORY = docs
INPUT = src include
RECURSIVE = YES
EXTRACT_ALL = YES
GENERATE_HTML = YES
GENERATE_XML = YES
XML_OUTPUT = xml
GENERATE_LATEX = NO
HAVE_DOT = NO
CALL_GRAPH = NO
CALLER_GRAPH = NO
"#,
				project_name, project_name
			),
			DocTool::Sphinx => format!(
				r#"# Sphinx configuration for {}

project = '{}'
copyright = '2025'
author = 'Author'

extensions = ['breathe']

html_theme = 'alabaster'
breathe_projects = {{"{}": "xml"}}
breathe_default_project = '{}'
"#,
				project_name, project_name, project_name, project_name
			),
		}
	}

	pub fn write_config(&self, project_name: &str) -> Result<()> {
		let project_name = crate::project_build_name(project_name);
		match self.tool {
			DocTool::Doxygen => {
				if Path::new("docs/conf.py").exists()
					|| Path::new("docs/index.rst").exists()
					|| Path::new("docs/requirements.txt").exists()
				{
					bail!(
						"Cannot add Doxygen: Sphinx documentation is already configured in docs/"
					);
				}
				ensure_path_available(Path::new("Doxyfile"))?;
				write_atomic(Path::new("Doxyfile"), &self.generate_config(&project_name))
					.context("Failed to write Doxyfile")?;
			}
			DocTool::Sphinx => {
				let conf_path = Path::new("docs/conf.py");
				let index_path = Path::new("docs/index.rst");
				let doxyfile_path = Path::new("Doxyfile");
				let requirements_path = Path::new("docs/requirements.txt");
				let conf_exists = conf_path.exists();
				let index_exists = index_path.exists();
				if doxyfile_path.exists() && !conf_exists && !index_exists {
					bail!(
						"Cannot add Sphinx: Doxygen documentation is already configured by Doxyfile"
					);
				}
				if conf_exists != index_exists {
					bail!(
						"Sphinx configuration is incomplete: both docs/conf.py and docs/index.rst are required"
					);
				}
				if !conf_exists {
					ensure_path_available(conf_path)?;
					ensure_path_available(index_path)?;
				}
				fs::create_dir_all("docs").context("Failed to create docs directory")?;
				if !conf_exists {
					let title = format!("Welcome to {}'s documentation!", project_name);
					let underline = "=".repeat(title.chars().count());
					let index_content = format!(
						"{title}\n{underline}\n\n.. toctree::\n   :maxdepth: 2\n   :caption: Contents:\n\nAPI Documentation\n================\n\n.. doxygenindex::\n   :project: {}\n",
						project_name
					);
					write_atomic(conf_path, &self.generate_config(&project_name))
						.context("Failed to write Sphinx conf")?;
					write_atomic(index_path, &index_content)
						.context("Failed to write Sphinx index")?;
				}
				if !doxyfile_path.exists() {
					write_atomic(
						doxyfile_path,
						&DocGenerator::new(DocTool::Doxygen).generate_config(&project_name),
					)
					.context("Failed to write Doxyfile")?;
				}
				if !requirements_path.exists() {
					write_atomic(requirements_path, "breathe\n")
						.context("Failed to write Sphinx requirements")?;
				}
			}
		}
		Ok(())
	}

	pub fn add_to_cmake(&self, cmake_path: &Path, _project_name: &str) -> Result<()> {
		let mut content =
			fs::read_to_string(cmake_path).context("Failed to read CMakeLists.txt")?;
		if cmake_target_exists(&content, "docs") {
			ensure_cmake_docs_compatible(&content, self.tool)?;
		} else {
			let target = match self.tool {
				DocTool::Doxygen => {
					r#"
find_program(DOXYGEN doxygen)
if(DOXYGEN)
    add_custom_target(docs
        COMMAND ${DOXYGEN} Doxyfile
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
        COMMENT "Generating API documentation with Doxygen"
        VERBATIM
    )
endif()
"#
				}
				DocTool::Sphinx => {
					r#"
find_program(DOXYGEN doxygen)
find_program(SPHINX_BUILD sphinx-build)
if(DOXYGEN AND SPHINX_BUILD)
    add_custom_target(docs
        COMMAND ${DOXYGEN} Doxyfile
        COMMAND ${SPHINX_BUILD} -b html docs docs/_build/html
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
        COMMENT "Generating documentation with Sphinx"
        VERBATIM
    )
endif()
"#
				}
			};
			content.push_str(target);
		}
		write_atomic(cmake_path, &content).context("Failed to write CMakeLists.txt")
	}

	pub fn add_to_makefile(&self, makefile_path: &Path) -> Result<()> {
		let mut content = fs::read_to_string(makefile_path).context("Failed to read Makefile")?;
		if make_target_exists(&content, "docs") {
			ensure_make_docs_compatible(&content, self.tool)?;
		} else {
			let target = match self.tool {
				DocTool::Doxygen => "\ndocs:\n\tdoxygen Doxyfile\n",
				DocTool::Sphinx => {
					"\ndocs:\n\tdoxygen Doxyfile\n\tsphinx-build -b html docs docs/_build/html\n"
				}
			};
			content.push_str(target);
			if let Some(position) = content.find(".PHONY:") {
				let line_end = content[position..]
					.find('\n')
					.map(|offset| position + offset)
					.unwrap_or(content.len());
				content.insert_str(line_end, " docs");
			} else {
				content.push_str("\n.PHONY: docs\n");
			}
		}
		write_atomic(makefile_path, &content).context("Failed to write Makefile")
	}
}

fn cmake_target_exists(content: &str, target: &str) -> bool {
	content.lines().any(|line| {
		let trimmed = line.trim_start();
		trimmed.starts_with("add_custom_target(")
			&& trimmed
				.split_once('(')
				.and_then(|(_, arguments)| arguments.split_whitespace().next())
				.is_some_and(|name| name == target)
	})
}

fn make_target_exists(content: &str, target: &str) -> bool {
	let prefix = format!("{}:", target);
	content
		.lines()
		.any(|line| line.trim_start().starts_with(&prefix))
}

fn configured_doc_tool(content: &str) -> Option<DocTool> {
	let content = content.to_ascii_lowercase();
	if content.contains("sphinx-build") || content.contains("sphinx_build") {
		Some(DocTool::Sphinx)
	} else if content.contains("doxygen") {
		Some(DocTool::Doxygen)
	} else {
		None
	}
}

fn ensure_cmake_docs_compatible(content: &str, requested: DocTool) -> Result<()> {
	match configured_doc_tool(content) {
		Some(existing) if existing == requested => Ok(()),
		Some(existing) => bail!(
			"Cannot add {}: {} is already configured in the CMake docs target",
			requested.as_str(),
			existing.as_str()
		),
		None => bail!(
			"Cannot add {}: an existing docs target has no recognized documentation tool",
			requested.as_str()
		),
	}
}

fn ensure_make_docs_compatible(content: &str, requested: DocTool) -> Result<()> {
	match configured_doc_tool(content) {
		Some(existing) if existing == requested => Ok(()),
		Some(existing) => bail!(
			"Cannot add {}: {} is already configured in the Makefile docs target",
			requested.as_str(),
			existing.as_str()
		),
		None => bail!(
			"Cannot add {}: an existing docs target has no recognized documentation tool",
			requested.as_str()
		),
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
}
