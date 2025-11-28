use crate::languages::Language;

pub fn generate_gitignore(language: Language) -> String {
	match language {
		Language::C | Language::Cpp => "# Build artifacts\n\
			build/\n\
			cmake-build-*/\n\
			*.o\n\
			*.a\n\
			*.so\n\
			*.exe\n\
			*.dll\n\
			*.dylib\n\
			\n\
			# CMake\n\
			CMakeFiles/\n\
			CMakeCache.txt\n\
			cmake_install.cmake\n\
			Makefile\n\
			\n\
			# IDE\n\
			.vscode/\n\
			.idea/\n\
			*.swp\n\
			*.swo\n\
			*~\n\
			.DS_Store\n\
			\n\
			# Dependencies\n\
			conan.lock\n\
			vcpkg_installed/\n\
			\n\
			# Generated files\n\
			*.d\n\
			*.su\n\
			\n\
			# Binaries\n\
			bin/\n\
			dist/\n\
			\n\
			# Testing\n\
			test_*\n\
			my_project/\n"
			.to_string(),
	}
}

pub fn generate_editorconfig() -> String {
	"root = true\n\
	\n\
	[*]\n\
	indent_style = tab\n\
	indent_size = 4\n\
	end_of_line = lf\n\
	charset = utf-8\n\
	trim_trailing_whitespace = true\n\
	insert_final_newline = true\n\
	\n\
	[*.md]\n\
	trim_trailing_whitespace = false\n"
		.to_string()
}

pub fn generate_clang_format_config(language: Language) -> String {
	match language {
		Language::C => "---\n\
			Language: C\n\
			Standard: C11\n\
			IndentWidth: 4\n\
			UseTab: ForContinuationAndIndentation\n\
			TabWidth: 4\n\
			ColumnLimit: 100\n\
			AllowShortFunctionsOnASingleLine: Empty\n\
			AllowShortIfStatementsOnASingleLine: Never\n\
			BreakBeforeBraces: Linux\n\
			SpaceAfterCStyleCast: true\n"
			.to_string(),
		Language::Cpp => "---\n\
			Language: Cpp\n\
			Standard: C++17\n\
			IndentWidth: 4\n\
			UseTab: ForContinuationAndIndentation\n\
			TabWidth: 4\n\
			ColumnLimit: 100\n\
			AllowShortFunctionsOnASingleLine: Empty\n\
			AllowShortIfStatementsOnASingleLine: Never\n\
			BreakBeforeBraces: Linux\n\
			SpaceAfterCStyleCast: true\n\
			Cpp11BracedListStyle: true\n"
			.to_string(),
	}
}

pub fn generate_vscode_settings(language: Language) -> String {
	let extension = match language {
		Language::C => "c",
		Language::Cpp => "cpp",
	};

	format!(
		"{{\n\
		\t\"[{}]\": {{\n\
		\t\t\"editor.defaultFormatter\": \"ms-vscode.cpptools\",\n\
		\t\t\"editor.formatOnSave\": true,\n\
		\t\t\"editor.rulers\": [100],\n\
		\t\t\"editor.tabSize\": 4,\n\
		\t\t\"editor.insertSpaces\": false\n\
		\t}}\n\
		}}\n",
		extension
	)
}

pub fn generate_vscode_launch_config(project_name: &str) -> String {
	format!(
		"{{\n\
		\t\"version\": \"0.2.0\",\n\
		\t\"configurations\": [\n\
		\t\t{{\n\
		\t\t\t\"name\": \"C/C++ Debug\",\n\
		\t\t\t\"type\": \"cppdbg\",\n\
		\t\t\t\"request\": \"launch\",\n\
		\t\t\t\"program\": \"${{workspaceFolder}}/build/{}\",\n\
		\t\t\t\"args\": [],\n\
		\t\t\t\"stopAtEntry\": false,\n\
		\t\t\t\"cwd\": \"${{workspaceFolder}}\",\n\
		\t\t\t\"environment\": [],\n\
		\t\t\t\"externalConsole\": false,\n\
		\t\t\t\"MIMode\": \"gdb\",\n\
		\t\t\t\"preLaunchTask\": \"build\",\n\
		\t\t\t\"setupCommands\": [\n\
		\t\t\t\t{{\n\
		\t\t\t\t\t\"description\": \"Enable pretty-printing for gdb\",\n\
		\t\t\t\t\t\"text\": \"-enable-pretty-printing\",\n\
		\t\t\t\t\t\"ignoreFailures\": true\n\
		\t\t\t\t}}\n\
		\t\t\t]\n\
		\t\t}}\n\
		\t]\n\
		}}\n",
		project_name
	)
}

pub fn generate_vscode_tasks_config() -> String {
	"{\n\
	\t\"version\": \"2.0.0\",\n\
	\t\"tasks\": [\n\
	\t\t{\n\
	\t\t\t\"label\": \"build\",\n\
	\t\t\t\"type\": \"shell\",\n\
	\t\t\t\"command\": \"mkdir -p build && cd build && cmake -DCMAKE_BUILD_TYPE=Debug .. && cmake --build .\",\n\
	\t\t\t\"problemMatcher\": [\"$gcc\"],\n\
	\t\t\t\"group\": {\n\
	\t\t\t\t\"kind\": \"build\",\n\
	\t\t\t\t\"isDefault\": true\n\
	\t\t\t}\n\
	\t\t},\n\
	\t\t{\n\
	\t\t\t\"label\": \"rebuild\",\n\
	\t\t\t\"type\": \"shell\",\n\
	\t\t\t\"command\": \"rm -rf build && mkdir -p build && cd build && cmake .. && cmake --build .\",\n\
	\t\t\t\"problemMatcher\": [\"$gcc\"]\n\
	\t\t}\n\
	\t]\n\
	}\n"
		.to_string()
}

pub fn generate_readme(project_name: &str, language: Language) -> String {
	let lang_name = match language {
		Language::C => "C",
		Language::Cpp => "C++",
	};

	format!(
		"# {}\n\n\
		A {} project created with [sticks](https://github.com/mAmineChniti/sticks).\n\n\
		## Building\n\n\
		### Using CMake (Recommended)\n\
		```bash\n\
		mkdir build\n\
		cd build\n\
		cmake ..\n\
		cmake --build .\n\
		```\n\n\
		### Using Makefile\n\
		```bash\n\
		make\n\
		make run\n\
		```\n\n\
		### Using Make with Debug\n\
		```bash\n\
		make debug\n\
		```\n\n\
		## Project Structure\n\n\
		```\n\
		{}/\n\
		├── src/              # Source files\n\
		├── include/          # Header files (if applicable)\n\
		├── build/            # Build artifacts (generated)\n\
		├── CMakeLists.txt    # CMake configuration\n\
		├── Makefile          # Makefile configuration\n\
		└── README.md         # This file\n\
		```\n\n\
		## Adding Dependencies\n\n\
		### Using sticks\n\
		```bash\n\
		sticks add libcurl openssl\n\
		```\n\n\
		### Adding Source Files\n\
		```bash\n\
		sticks src utils network\n\
		```\n\n\
		## License\n\n\
		This project is licensed under the MIT License.\n",
		project_name, lang_name, project_name
	)
}

pub fn generate_precommit_hook() -> &'static str {
	"#!/bin/bash\n\
	\n\
	set -e\n\
	\n\
	echo \"Running pre-commit checks...\"\n\
	\n\
	# Format check\n\
	echo \"  Checking code formatting...\"\n\
	find src -name '*.c' -o -name '*.cpp' -o -name '*.h' | xargs clang-format --dry-run -i\n\
	\n\
	echo \"  All checks passed!\"\n"
}

pub fn generate_gitattributes() -> &'static str {
	"* text=auto\n\
	*.c text eol=lf\n\
	*.cpp text eol=lf\n\
	*.h text eol=lf\n\
	*.cmake text eol=lf\n\
	Makefile text eol=lf\n\
	CMakeLists.txt text eol=lf\n\
	*.md text eol=lf\n"
}
