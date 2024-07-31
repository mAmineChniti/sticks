extern crate clap;

use clap::{App, Arg, SubCommand};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::io::{Read, Write};
use std::path::Path;

const UPDATE_SCRIPT_URL: &str = "https://rb.gy/ltig1b";

enum Language {
	C,
	Cpp,
}

fn add_dependency(dependency_name: &str) -> io::Result<()> {
	if !Path::new("Makefile").exists() {
		return Err(io::Error::new(
			io::ErrorKind::NotFound,
			"Makefile not found in the current directory. Cannot add a dependency.",
		));
	}

	// Read Makefile content
	let mut makefile_content = String::new();
	let mut makefile = File::open("Makefile")?;
	makefile.read_to_string(&mut makefile_content)?;

	// Check if "all: clean install-deps" is present
	if !makefile_content.contains("all: clean install-deps") {
		// Replace "all: clean" with "all: clean install-deps"
		makefile_content = makefile_content.replace("all: clean", "all: clean install-deps");
	}

	// Check if the dependency is already present in the install-deps rule
	if makefile_content.contains(&format!("sudo apt install -y {}", dependency_name)) {
		println!(
			"Dependency '{}' is already present in the install-deps rule.",
			dependency_name
		);
		return Ok(());
	}

	// Check if "install-deps:" is present
	if !makefile_content.contains("install-deps:") {
		// Add a new install-deps rule
		makefile_content.push_str(&format!(
			"\ninstall-deps:\n\tsudo apt install -y {}\n",
			dependency_name
		));
	} else {
		// Append the dependency to the existing install-deps rule
		makefile_content = makefile_content.replace(
			"sudo apt install -y",
			&format!("sudo apt install -y {}", dependency_name),
		);
	}

	// Write the updated content back to the Makefile
	let mut makefile = OpenOptions::new()
		.write(true)
		.truncate(true)
		.create(true)
		.open("Makefile")?;
	makefile.write_all(makefile_content.as_bytes())?;

	Ok(())
}

fn has_install_deps_rule(makefile_content: &str) -> bool {
	makefile_content.contains("install-deps:")
}

fn remove_dependency(dependency_names: &[&str]) -> io::Result<()> {
	if !Path::new("Makefile").exists() {
		return Err(io::Error::new(
			io::ErrorKind::NotFound,
			"Makefile not found in the current directory. Cannot remove a dependency.",
		));
	}

	let makefile_path = "Makefile";
	let mut makefile_content = String::new();

	// Read the existing Makefile content
	{
		let mut makefile = fs::File::open(makefile_path)?;
		makefile.read_to_string(&mut makefile_content)?;
	}

	let mut updated_makefile_content = String::new();
	let mut found_dependencies = false;

	// Create a new string to accumulate the modified content
	let mut temp_updated_makefile_content = String::new();

	// Remove the lines containing the dependencies
	for line in makefile_content.lines() {
		if !dependency_names.iter().any(|dep| line.contains(dep)) {
			temp_updated_makefile_content.push_str(line);
			temp_updated_makefile_content.push('\n');
		} else {
			found_dependencies = true;
		}
	}

	if found_dependencies {
		// Write the updated content back to the Makefile
		let mut makefile = fs::File::create(makefile_path)?;
		makefile.write_all(temp_updated_makefile_content.as_bytes())?;
		println!("Dependencies {:?} removed from Makefile.", dependency_names);

		// Check if the install-deps rule is present and there are no more dependencies
		if has_install_deps_rule(&temp_updated_makefile_content)
			&& !temp_updated_makefile_content.contains("sudo apt install -y")
		{
			// Remove the install-deps rule
			let lines = temp_updated_makefile_content.lines();
			let mut remove_install_deps_rule = false;
			// write code to replace all: clean install-deps with all: clean
			for line in lines {
				if remove_install_deps_rule {
					if line.is_empty() {
						remove_install_deps_rule = false;
					}
					continue;
				}
				if line.contains("install-deps:") {
					remove_install_deps_rule = true;
					continue;
				}
				updated_makefile_content.push_str(line);
				updated_makefile_content.push('\n');
			}
			if updated_makefile_content.contains("all: clean install-deps") {
				updated_makefile_content =
					updated_makefile_content.replace("all: clean install-deps", "all: clean");
			}
			// Write the updated content (without install-deps rule) back to the Makefile
			let mut makefile = fs::File::create(makefile_path)?;
			makefile.write_all(updated_makefile_content.as_bytes())?;
			println!("Removed install-deps rule from Makefile.");
		}
	} else {
		println!("Dependencies {:?} not found in Makefile.", dependency_names);
	}

	Ok(())
}

fn create_dir(project_name: &str) -> io::Result<()> {
	let path = env::current_dir()?.join(project_name);

	if path.exists() {
		return Err(io::Error::new(
			io::ErrorKind::AlreadyExists,
			format!("Directory '{}' already exists", project_name),
		));
	}

	fs::create_dir(&path)?;

	env::set_current_dir(&path)?;

	Ok(())
}

fn add_sources(source_names: &[&str]) -> io::Result<()> {
	if !Path::new("src").exists() {
		print_colored(
			"src directory not found. Cannot add sources and headers.",
			"31",
			1,
		);
		print_colored("Maybe try creating a new project or initializing a new project in the current directory","31",1);

		return Err(io::Error::new(io::ErrorKind::NotFound, ""));
	}

	let src_path = Path::new("src");

	// Determine the extension based on existing files in src/
	let extension = determine_extension(src_path)?;

	for &source_name in source_names {
		let source_file = format!("{}.{}", source_name, extension);
		let source_path = src_path.join(&source_file);

		// Check if the source file already exists
		if source_path.exists() {
			println!("Source file {} already exists. Skipping.", source_file);
		} else {
			// Create the source file
			fs::write(&source_path, format!("// Code for {}\n", source_name))?;

			// Create corresponding .h file
			let header_file = format!("{}.h", source_name);
			let header_path = src_path.join(&header_file);
			fs::write(
				&header_path,
				format!(
					"#ifndef {}_H\n#define {}_H\n#endif /* {}_H */",
					source_name.to_uppercase(),
					source_name.to_uppercase(),
					source_name.to_uppercase()
				),
			)?;

			println!("Added source: {}", source_file);
		}
	}

	Ok(())
}

fn determine_extension(src_path: &Path) -> io::Result<&'static str> {
	// Find the first source file in src/ to determine the extension
	let source_file = fs::read_dir(src_path)?
		.filter_map(|entry| {
			let entry = entry.ok()?;
			let path = entry.path();
			if path.is_file() {
				path.extension()
					.map(|ext| ext.to_string_lossy().to_string())
			} else {
				None
			}
		})
		.next();

	match source_file.as_deref() {
		Some("c") => Ok("c"),
		Some("cpp") => Ok("cpp"),
		_ => {
			eprintln!("No existing source files found in src/. Defaulting to .c extension.");
			Ok("c") // Default to .c if no existing source files are found
		}
	}
}

fn create_project(project_name: &str, language: Language) -> io::Result<()> {
	println!("Creating project {}...", project_name);
	fs::create_dir("src")?;
	let source_file = format!("src/main.{}", language_extension(&language));
	File::create(&source_file)?;

	let cc = match language {
		Language::C => "gcc",
		Language::Cpp => "g++",
	};

	let makefile_content = format!(
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
		cc,
		project_name,
		project_name,
		language_extension(&language),
		project_name,
		project_name
	);

	let hello_world_code = match language {
		Language::C => {
			r#"
            #include <stdio.h>

            int main() {
                printf("Hello, World!\n");
                return 0;
            }
            "#
		}
		Language::Cpp => {
			r#"
            #include <iostream>

            int main() {
                std::cout << "Hello, World!" << std::endl;
                return 0;
            }
            "#
		}
	};

	let mut source_file = File::create(&source_file)?;
	source_file.write_all(hello_world_code.as_bytes())?;

	let mut makefile = File::create("Makefile")?;
	makefile.write_all(makefile_content.as_bytes())?;

	Ok(())
}

fn new_project(project_name: &str, language: Language) -> io::Result<()> {
	create_dir(project_name)?;
	create_project(project_name, language)?;
	Ok(())
}

fn init_project(language: Language) -> io::Result<()> {
	let current_dir = env::current_dir()?;
	let current_dir_name = current_dir
		.file_name()
		.ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to get directory name"))?
		.to_str()
		.ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Failed to convert to string"))?;
	create_project(current_dir_name, language)?;
	Ok(())
}

fn update_project() {
	let update_command = format!("curl -fsSL {} | bash", UPDATE_SCRIPT_URL);
	let status = std::process::Command::new("sh")
		.arg("-c")
		.arg(&update_command)
		.status()
		.expect("Failed to execute update command");

	if status.success() {
		// println!("Update successful!");
	} else {
		eprintln!("Update failed with exit code: {}", status);
	}
}

fn language_extension(language: &Language) -> &str {
	match language {
		Language::C => "c",
		Language::Cpp => "cpp",
	}
}

fn print_colored(text: &str, color_code: &str, num_newlines: usize) {
	print!("\x1b[{}m{}\x1b[0m", color_code, text);
	for _ in 0..num_newlines {
		println!();
	}
}

fn main() {
	let version = env!("CARGO_PKG_VERSION");
	let matches = App::new("sticks")
		.version(version)
		.about("A tool for managing C and C++ projects")
		.subcommand(
			SubCommand::with_name("c")
				.about("Create a C project")
				.arg(Arg::with_name("project_name").required(true).multiple(true)),
		)
		.subcommand(
			SubCommand::with_name("cpp")
				.about("Create a C++ project")
				.arg(Arg::with_name("project_name").required(true).multiple(true)),
		)
		.subcommand(
			SubCommand::with_name("init")
				.about("Initialize a project")
				.arg(
					Arg::with_name("language")
						.required(true)
						.possible_values(&["c", "cpp"]),
				),
		)
		.subcommand(
			SubCommand::with_name("add")
				.about("Add a dependency rule to the Makefile")
				.arg(Arg::with_name("dependency_name").required(true)),
		)
		.subcommand(
			SubCommand::with_name("remove")
				.about("Remove a dependency from the Makefile")
				.arg(
					Arg::with_name("dependency_name")
						.required(true)
						.multiple(true),
				),
		)
		.subcommand(
			SubCommand::with_name("src")
				.about("Add more source files to your project")
				.arg(Arg::with_name("source_names").required(true).multiple(true)),
		)
		.subcommand(SubCommand::with_name("update").about("Update sticks to the latest version"))
		.subcommand(SubCommand::with_name("help").about("Prints help information"))
		.version_short("v")
		.get_matches();

	match matches.subcommand() {
		("c", Some(sub_m)) => {
			let main_name = sub_m.value_of("project_name").unwrap();

			new_project(main_name, Language::C).unwrap_or_else(|e| {
				eprintln!("Error: {}", e);
				std::process::exit(1);
			});
		}
		("cpp", Some(sub_m)) => {
			let main_name = sub_m.value_of("project_name").unwrap();
			new_project(main_name, Language::Cpp).unwrap_or_else(|e| {
				eprintln!("Error: {}", e);
				std::process::exit(1);
			});
		}
		("init", Some(sub_m)) => {
			let language = match sub_m.value_of("language").unwrap() {
				"c" => Language::C,
				"cpp" => Language::Cpp,
				_ => {
					eprintln!("Invalid language");
					std::process::exit(1);
				}
			};
			init_project(language).unwrap_or_else(|e| {
				eprintln!("Error: {}", e);
				std::process::exit(1);
			});
		}
		("add", Some(sub_m)) => {
			add_dependency(sub_m.value_of("dependency_name").unwrap()).unwrap_or_else(|e| {
				eprintln!("Error: {}", e);
				std::process::exit(1);
			});
		}
		("remove", Some(sub_m)) => {
			// Implement the removal logic when needed
			let dependencies: Vec<&str> = sub_m
				.values_of("dependency_name")
				.unwrap_or_default()
				.collect();
			remove_dependency(&dependencies).unwrap_or_else(|e| {
				eprintln!("Error: {}", e);
				std::process::exit(1);
			});
		}
		("src", Some(sub_m)) => {
			// Implement the removal logic when needed
			let sources: Vec<&str> = sub_m
				.values_of("source_names")
				.unwrap_or_default()
				.collect();
			add_sources(&sources).unwrap_or_else(|e| {
				eprintln!("Error: {}", e);
				std::process::exit(1);
			});
		}
		("update", Some(_)) => {
			update_project();
		}
		("help", Some(_)) | ("", None) => {
			// Display colored help message
			print_colored("sticks - A tool for managing C and C++ projects", "1;36", 2);
			print_colored("Available commands:", "1;34", 2);
			print_colored("sticks", "1;32", 0);
			print_colored(" c", "0", 0);
			print_colored(" <project_name>", "1;36", 1);
			print_colored("	Create a C project", "0", 2);
			print_colored("sticks", "1;32", 0);
			print_colored(" cpp", "0", 0);
			print_colored(" <project_name>", "1;36", 1);
			print_colored("	Create a C++ project", "0", 2);
			print_colored("sticks", "1;32", 0);
			print_colored(" init", "0", 0);
			print_colored(" <language>", "1;36", 1);
			print_colored("	Initialize a project", "0", 2);
			print_colored("sticks", "1;32", 0);
			print_colored(" add", "0", 0);
			print_colored(" <dependency_name>", "1;36", 1);
			print_colored("	Add a dependency rule to the Makefile", "0", 2);
			print_colored("sticks", "1;32", 0);
			print_colored(" remove", "0", 0);
			print_colored(" <dependency_name>", "1;36", 1);
			print_colored("	Remove a dependency from the Makefile", "0", 2);
			print_colored("sticks", "1;32", 0);
			print_colored(" src", "0", 0);
			print_colored(" <source_names>", "1;36", 1);
			print_colored("	Add source files and their headers", "0", 2);
			print_colored("sticks", "1;32", 0);
			print_colored(" update", "0", 1);
			print_colored("	Update sticks to the latest version", "0", 2);
		}
		_ => println!("Unknown command"),
	}
}
