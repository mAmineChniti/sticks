extern crate clap;

use clap::{App, Arg, SubCommand};
use std::fs::{self, File};
use std::io::{Write, Read};
use std::env;
use std::io;

const UPDATE_SCRIPT_URL: &str = "https://rb.gy/ltig1b";

enum Language {
    C,
    Cpp,
}

fn has_install_deps_rule(makefile_content: &str) -> bool {
    makefile_content.contains("install-deps:")
}

fn remove_dependency(dependency_names: &[&str]) -> io::Result<()> {
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
            let mut lines = temp_updated_makefile_content.lines();
            let mut remove_install_deps_rule = false;

            while let Some(line) = lines.next() {
                if remove_install_deps_rule {
                    if line.trim().is_empty() {
                        remove_install_deps_rule = false;
                    }
                } else {
                    if line.contains("install-deps:") {
                        remove_install_deps_rule = true;
                    } else {
                        updated_makefile_content.push_str(line);
                        updated_makefile_content.push('\n');
                    }
                }
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

fn create_project(project_name: &str, language: Language) -> io::Result<()> {
    fs::create_dir("src")?;

    let source_file = format!("src/{}.{}", project_name, language_extension(&language));
    File::create(&source_file)?;

    let cc = match language {
        Language::C => "gcc",
        Language::Cpp => "g++",
    };

    let makefile_content = format!(
        "CC = {}\n\
        CFLAGS = -Wall -g\n\
        \n\
        all: clean {}\n\
        \n\
        {}: {}\n\
        \t$(CC) $(CFLAGS) -o {} $<\n\
        \n\
        clean:\n\
        \trm -f {}\n",
        cc, project_name, project_name, source_file, project_name, project_name
    );

    let hello_world_code = match language {
        Language::C => r#"
#include <stdio.h>

int main() {
    printf("Hello, World!\n");
    return 0;
}
"#,
        Language::Cpp => r#"
#include <iostream>

int main() {
    std::cout << "Hello, World!" << std::endl;
    return 0;
}
"#,
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
    let current_dir_name = current_dir.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::Other, "Failed to get directory name")
    })?.to_str().ok_or_else(|| {
        io::Error::new(io::ErrorKind::Other, "Failed to convert to string")
    })?;
    create_project(current_dir_name, language)?;
    Ok(())
}

fn add_dependency(dependency_name: &str) -> io::Result<()> {
    let mut makefile_content = String::new();
    let mut makefile = fs::OpenOptions::new().read(true).open("Makefile")?;
    makefile.read_to_string(&mut makefile_content)?;

    // Check if the dependency is already present in the install-deps rule
    if makefile_content.contains(&format!("sudo apt install -y {}", dependency_name)) {
        println!("Dependency '{}' is already present in the install-deps rule.", dependency_name);
        return Ok(());
    }

    // Check if the install-deps rule is present
    if let Some(_index) = makefile_content.find("install-deps:") {
        let mut lines = makefile_content.lines();
        let mut new_content = String::new();
        let mut dependency_already_present = false;

        // Append the dependency to the existing install-deps rule
        while let Some(line) = lines.next() {
            if line.trim().starts_with("sudo apt install -y") && line.contains(dependency_name) {
                dependency_already_present = true;
                break;
            }

            new_content.push_str(line);
            new_content.push('\n');

            if line.trim() == "install-deps:" {
                new_content.push_str(&format!("    sudo apt install -y {}\n", dependency_name));
            }
        }

        if dependency_already_present {
            println!("Dependency '{}' is already present in the install-deps rule.", dependency_name);
            return Ok(());
        }

        makefile_content = new_content;
    } else {
        // Add a new install-deps rule
        makefile_content.push_str(&format!(
            r#"
install-deps:
    sudo apt install -y {}
"#,
            dependency_name
        ));
    }

    // Write the updated content back to the Makefile
    let mut makefile = fs::OpenOptions::new().write(true).truncate(true).open("Makefile")?;
    makefile.write_all(makefile_content.as_bytes())?;

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
    let matches = App::new("sticks")
        .version(env!("CARGO_PKG_VERSION"))
        .about("A tool for managing C and C++ projects")
        .subcommand(
            SubCommand::with_name("c")
                .about("Create a C project")
                .arg(Arg::with_name("project_name").required(true)),
        )
        .subcommand(
            SubCommand::with_name("cpp")
                .about("Create a C++ project")
                .arg(Arg::with_name("project_name").required(true)),
        )
        .subcommand(
            SubCommand::with_name("init")
                .about("Initialize a project")
                .arg(Arg::with_name("language").required(true).possible_values(&["c", "cpp"])),
        )
        .subcommand(
            SubCommand::with_name("add")
                .about("Add a dependency rule to the Makefile")
                .arg(Arg::with_name("dependency_name").required(true)),
        )
        .subcommand(
            SubCommand::with_name("remove")
                .about("Remove a dependency from the Makefile")
                .arg(Arg::with_name("dependency_name").required(true).multiple(true)),
        )
        .subcommand(
            SubCommand::with_name("update")
                .about("Update sticks to the latest version"),
        )
        .subcommand(
            SubCommand::with_name("help")
            .about("Prints help information"),
        )
        .version_short("v")
        .get_matches();

    match matches.subcommand() {
        ("c", Some(sub_m)) => {
            new_project(sub_m.value_of("project_name").unwrap(), Language::C).unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });
        }
        ("cpp", Some(sub_m)) => {
            new_project(sub_m.value_of("project_name").unwrap(), Language::Cpp).unwrap_or_else(|e| {
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
            let dependencies: Vec<&str> = sub_m.values_of("dependency_name").unwrap_or_default().collect();
		    remove_dependency(&dependencies).unwrap_or_else(|e| {
		        eprintln!("Error: {}", e);
		        std::process::exit(1);
		    });
        }
        ("update", Some(_)) => {
            update_project();
        }
        ("help", Some(_)) | ("", None) => {
            // Display colored help message
            print_colored("sticks - A tool for managing C and C++ projects", "1;36",2);
            print_colored("Available commands:", "1;34",2);
            print_colored("sticks", "1;32",0);
            print_colored(" c", "0",0);
            print_colored(" <project_name>", "1;36",1);
            print_colored("    Create a C project", "0",2);
            print_colored("sticks", "1;32",0);
            print_colored(" cpp", "0",0);
            print_colored(" <project_name>", "1;36",1);
            print_colored("    Create a C++ project", "0",2);
            print_colored("sticks", "1;32",0);
            print_colored(" init", "0",0);
            print_colored(" <language>", "1;36",1);
            print_colored("    Initialize a project", "0",2);
            print_colored("sticks", "1;32",0);
            print_colored(" add", "0",0);
            print_colored(" <dependency_name>", "1;36",1);
            print_colored("    Add a dependency rule to the Makefile", "0",2);
            print_colored("sticks", "1;32",0);
            print_colored(" remove", "0",0);
            print_colored(" <dependency_name>", "1;36",1);
            print_colored("    Remove a dependency from the Makefile", "0",2);
            print_colored("sticks", "1;32",0);
            print_colored(" update", "0",1);
            print_colored("    Update sticks to the latest version", "0",2);
        }
        _ => println!("Unknown command"),
    }
}
