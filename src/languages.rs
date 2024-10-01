use std::str::FromStr;

pub trait LanguageConsts {
	fn cc(&self) -> &'static str;
	fn extension(&self) -> &'static str;
	fn generate_helloworld_content(&self) -> String;

	fn generate_makefile_content(&self, project_name: &str) -> String {
		format!(
			"CC = {}\n\
             CFLAGS = -Wall -Wextra -g\n\
             \n\
             all: clean {}\n\
             \n\
             {}: src/*.{}\n\
             \t$(CC) $(CFLAGS) -o {} $^\n\
             \n\
             clean:\n\
             \trm -f {}\n",
			self.cc(),
			project_name,
			project_name,
			self.extension(),
			project_name,
			project_name
		)
	}
}

pub enum Language {
	C,
	Cpp,
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
	type Err = String;

	fn from_str(input: &str) -> Result<Language, Self::Err> {
		match input.to_lowercase().as_str() {
			"c" => Ok(Language::C),
			"cpp" => Ok(Language::Cpp),
			_ => Err(format!("Unsupported language: {}", input)),
		}
	}
}
