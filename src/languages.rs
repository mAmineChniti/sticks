use anyhow::Context;
use std::fs;
use std::path::Path;
use std::str::FromStr;

pub trait LanguageConsts {
	fn cc(&self) -> &'static str;
	fn extension(&self) -> &'static str;
	fn generate_helloworld_content(&self) -> String;

	fn generate_makefile_content(&self, project_name: &str) -> String {
		let project_name = crate::project_build_name(project_name);
		let compiler_variable = makefile_compiler_variable(self.extension());
		let flags_variable = makefile_flags_variable(self.extension());
		let standard_flag = makefile_standard_flag(self.extension());
		let source_block = makefile_source_block(self.extension());

		format!(
			"# Compiler and flags\n\
			{} = {}\n\
			{} = {} -Wall -Wextra -Werror -O2 -g\n\
			CPPFLAGS = -I$(INCLUDE_DIR)\n\
			LDFLAGS =\n\
			LDLIBS =\n\
			\n\
			# Directories\n\
			SRC_DIR = src\n\
			INCLUDE_DIR = include\n\
			BUILD_DIR = build\n\
			BIN_DIR = bin\n\
			\n\
			# Source files\n{}\n\
			# Target executable\n\
			TARGET = $(BIN_DIR)/{}\n\
			\n\
			# Default target\n\
			all: $(TARGET)\n\
			\n\
			# Build target\n\
			$(TARGET): $(OBJS)\n\
				\t@mkdir -p $(BUILD_DIR) $(BIN_DIR)\n\
				\t$({}) $(CPPFLAGS) $({}) -o $@ $^ $(LDFLAGS) $(LDLIBS)\n\
				\t@echo \"Build complete: $(TARGET)\"\n\
			\n\
			# Clean build artifacts\n\
			clean:\n\
				\t@rm -rf $(BUILD_DIR) $(BIN_DIR)\n\
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
			compiler_variable,
			self.cc(),
			flags_variable,
			standard_flag,
			source_block,
			project_name,
			compiler_variable,
			flags_variable,
		)
	}
}

fn makefile_compiler_variable(extension: &str) -> &'static str {
	if is_cpp_extension(extension) {
		"CXX"
	} else {
		"CC"
	}
}

fn makefile_flags_variable(extension: &str) -> &'static str {
	if is_cpp_extension(extension) {
		"CXXFLAGS"
	} else {
		"CFLAGS"
	}
}

fn makefile_standard_flag(extension: &str) -> &'static str {
	if is_cpp_extension(extension) {
		"-std=c++17"
	} else {
		"-std=c11"
	}
}

fn makefile_source_block(extension: &str) -> String {
	let source_pattern = if is_cpp_extension(extension) {
		r#"find $(SRC_DIR) -type f \( -name '*.cpp' -o -name '*.cc' -o -name '*.cxx' -o -name '*.c++' -o -name '*.C' -o -name '*.CPP' -o -name '*.CC' -o -name '*.CXX' \) -print | sort"#
	} else {
		"find $(SRC_DIR) -type f -name '*.c' -print | sort"
	};
	format!(
		"SRCS = $(shell {source_pattern})\n\
		OBJS = $(patsubst $(SRC_DIR)/%,$(BUILD_DIR)/%.o,$(SRCS))\n\
		\n\
		$(BUILD_DIR)/%.o: $(SRC_DIR)/%\n\
			\t@mkdir -p $(dir $@)\n\
			\t$({compiler}) $(CPPFLAGS) $({flags}) -c $< -o $@\n",
		compiler = makefile_compiler_variable(extension),
		flags = makefile_flags_variable(extension),
	)
}

fn is_cpp_extension(extension: &str) -> bool {
	matches!(
		extension.to_ascii_lowercase().as_str(),
		"cpp" | "cc" | "cxx" | "c++"
	) || extension == "C"
}

pub(crate) fn source_extension(src_path: &Path) -> anyhow::Result<Option<&'static str>> {
	let mut has_c = false;
	let mut has_cpp = false;
	collect_source_extensions(src_path, &mut has_c, &mut has_cpp)?;

	if has_c && has_cpp {
		anyhow::bail!(
			"Mixed C and C++ source files are not supported; use separate projects or targets"
		);
	} else if has_cpp {
		Ok(Some("cpp"))
	} else if has_c {
		Ok(Some("c"))
	} else {
		Ok(None)
	}
}

fn collect_source_extensions(
	src_path: &Path,
	has_c: &mut bool,
	has_cpp: &mut bool,
) -> anyhow::Result<()> {
	let mut entries = fs::read_dir(src_path)
		.with_context(|| format!("Failed to read source directory {}", src_path.display()))?
		.collect::<std::result::Result<Vec<_>, _>>()
		.with_context(|| format!("Failed to read source directory {}", src_path.display()))?;
	entries.sort_by_key(|entry| entry.file_name());

	for entry in entries {
		let file_type = entry
			.file_type()
			.context("Failed to inspect directory entry")?;
		if file_type.is_dir() {
			collect_source_extensions(&entry.path(), has_c, has_cpp)?;
		} else if file_type.is_file()
			&& let Some(extension) = entry
				.path()
				.extension()
				.and_then(|extension| extension.to_str())
		{
			if is_cpp_extension(extension) {
				*has_cpp = true;
			} else if extension.eq_ignore_ascii_case("c") {
				*has_c = true;
			}
		}
	}

	Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
		Self::from_project_structure_with_prompt(true)
	}

	pub fn from_project_structure_with_prompt(
		interactive: bool,
	) -> Result<Language, anyhow::Error> {
		if Path::new("src").is_dir()
			&& let Some(extension) = source_extension(Path::new("src"))?
		{
			return Ok(if extension == "cpp" {
				Language::Cpp
			} else {
				Language::C
			});
		}

		if !interactive {
			return Ok(Language::C);
		}

		println!("⚠️  No source files found to detect language.");
		println!("   Please select the target language:");
		println!("   [1] C");
		println!("   [2] C++");
		print!("   Choice (1-2): ");

		use std::io::{self, Write};
		io::stdout()
			.flush()
			.context("Failed to flush language prompt")?;

		let mut input = String::new();
		match io::stdin().read_line(&mut input) {
			Ok(_) => match input.trim() {
				"1" | "c" | "C" => {
					println!("✓ Selected C language");
					Ok(Language::C)
				}
				"2" | "cpp" | "C++" | "c++" => {
					println!("✓ Selected C++ language");
					Ok(Language::Cpp)
				}
				_ => {
					println!("Invalid choice. Defaulting to C.");
					Ok(Language::C)
				}
			},
			Err(_) => {
				println!("Failed to read input. Defaulting to C.");
				Ok(Language::C)
			}
		}
	}
}
