use anyhow::Context;
use std::fs;
use std::path::Path;
use std::str::FromStr;

pub trait LanguageConsts {
	fn cc(&self) -> &'static str;
	fn extension(&self) -> &'static str;
	fn generate_helloworld_content(&self) -> String;

	fn generate_makefile_content(&self, project_name: &str) -> String {
		format!(
			"# Compiler and flags\n\
			CC = {}\n\
			CFLAGS = -Wall -Wextra -Werror -O2 -g\n\
			LDFLAGS =\n\
			\n\
			# Directories\n\
			SRC_DIR = src\n\
			BUILD_DIR = build\n\
			\n\
			# Source files\n\
			SRCS = $(wildcard $(SRC_DIR)/*.{})\n\
			OBJS = $(SRCS:$(SRC_DIR)/%.{}=$(BUILD_DIR)/%.o)\n\
			\n\
			# Target executable\n\
			TARGET = {}\n\
			\n\
			# Default target\n\
			all: $(TARGET)\n\
			\n\
			# Build target\n\
			$(TARGET): $(OBJS)\n\
			\t@mkdir -p $(BUILD_DIR)\n\
			\t$(CC) $(CFLAGS) -o $@ $^ $(LDFLAGS)\n\
			\t@echo \"Build complete: $(TARGET)\"\n\
			\n\
			# Compile source files\n\
			$(BUILD_DIR)/%.o: $(SRC_DIR)/%.{}\n\
			\t@mkdir -p $(BUILD_DIR)\n\
			\t$(CC) $(CFLAGS) -c $< -o $@\n\
			\n\
			# Clean build artifacts\n\
			clean:\n\
			\t@rm -rf $(BUILD_DIR) $(TARGET)\n\
			\t@echo \"Cleaned build artifacts\"\n\
			\n\
			# Run the program\n\
			run: $(TARGET)\n\
			\t./$(TARGET)\n\
			\n\
			# Rebuild\n\
			rebuild: clean all\n\
			\n\
			.PHONY: all clean run rebuild\n",
			self.cc(),
			self.extension(),
			self.extension(),
			project_name,
			self.extension()
		)
	}
}

#[derive(Debug, Clone, Copy)]
pub enum Language {
	C,
	Cpp,
}

impl std::fmt::Display for Language {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Language::C => write!(f, "C"),
			Language::Cpp => write!(f, "C++"),
		}
	}
}

impl LanguageConsts for Language {
	fn cc(&self) -> &'static str {
		match self {
			Language::C => "gcc",
			Language::Cpp => "g++",
		}
	}

	fn extension(&self) -> &'static str {
		match self {
			Language::C => "c",
			Language::Cpp => "cpp",
		}
	}

	fn generate_helloworld_content(&self) -> String {
		match self {
			Language::C => String::from(
				"#include <stdio.h>\n\n\
                 int main() {\n\
                 \tprintf(\"Hello, World!\\n\");\n\
                 \treturn 0;\n\
                 }\n",
			),
			Language::Cpp => String::from(
				"#include <iostream>\n\n\
                 int main() {\n\
                 \tstd::cout << \"Hello, World!\" << std::endl;\n\
                 \treturn 0;\n\
                 }\n",
			),
		}
	}
}

impl FromStr for Language {
	type Err = anyhow::Error;

	fn from_str(input: &str) -> Result<Language, Self::Err> {
		match input.to_lowercase().as_str() {
			"c" => Ok(Language::C),
			"cpp" => Ok(Language::Cpp),
			_ => anyhow::bail!("Unsupported language: {}. Use 'c' or 'cpp'", input),
		}
	}
}

impl Language {
	pub fn from_project_structure() -> Result<Language, anyhow::Error> {
		if Path::new("src").exists() {
			let entries = fs::read_dir("src").context("Failed to read src directory")?;

			for entry in entries {
				let entry = entry.context("Failed to read directory entry")?;
				let path = entry.path();

				if let Some(ext) = path.extension() {
					match ext.to_str() {
						Some("cpp") | Some("cc") | Some("cxx") => return Ok(Language::Cpp),
						Some("c") => return Ok(Language::C),
						_ => continue,
					}
				}
			}
		}

		Ok(Language::C)
	}
}
