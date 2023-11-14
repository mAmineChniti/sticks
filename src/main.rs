extern crate clap;

use clap::{App, Arg, SubCommand};
use std::fs::{self, File};
use std::io::{Write, Read};
use std::env;
use std::io;

fn _remove_dependency(_dependency_name: &str) {
    // TODO: implement the removal of dependencies and the install_deps rule if 0 deps are left 
}
const UPDATE_SCRIPT_URL: &str = "https://rb.gy/ltig1b";

enum Language {
    C,
    Cpp,
}

fn create_dir(project_name: &str) -> io::Result<()> {
    fs::create_dir(project_name)?;
    env::set_current_dir(project_name)?;
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

    if let Some(index) = makefile_content.find("sudo apt install -y") {
        let install_deps_line = &makefile_content[index..];
        if let Some(eol_index) = install_deps_line.find('\n') {
            let (before, after) = install_deps_line.split_at(eol_index);
            let modified_line = format!("{} {}{}", before, dependency_name, after);
            makefile_content = makefile_content.replace(install_deps_line, &modified_line);
        }
    } else {
        makefile_content.push_str(&format!(
            r#"
# Add a rule to install dependencies
install-deps:
    sudo apt install -y {}
"#,
            dependency_name
        ));
    }

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

fn print_colored(text: &str, color_code: &str) {
    print!("\x1b[{}m{}\x1b[0m", color_code, text);
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
                .arg(Arg::with_name("dependency_name").required(true)),
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
            println!("Removing dependency: {}", sub_m.value_of("dependency_name").unwrap());
        }
        ("update", Some(_)) => {
            update_project();
        }
        ("help", Some(_)) | ("", None) => {
            // Display colored help message
            // TODO: Simplify this code
            print_colored("sticks - A tool for managing C and C++ projects", "1;36");
            println!();
            println!();
            print_colored("Available commands:", "1;34");
            println!();
            println!();
            print_colored("sticks", "1;32");
            print_colored(" c", "0");
            print_colored(" <project_name>", "1;36");
            println!();
            print_colored("    Create a C project", "0");
            println!();
            println!();
            print_colored("sticks", "1;32");
            print_colored(" cpp", "0");
            print_colored(" <project_name>", "1;36");
            println!();
            print_colored("    Create a C++ project", "0");
            println!();
            println!();
            print_colored("sticks", "1;32");
            print_colored(" init", "0");
            print_colored(" <language>", "1;36");
            println!();
            print_colored("    Initialize a project", "0");
            println!();
            println!();
            print_colored("sticks", "1;32");
            print_colored(" add", "0");
            print_colored(" <dependency_name>", "1;36");
            println!();
            print_colored("    Add a dependency rule to the Makefile", "0");
            println!();
            println!();
            print_colored("sticks", "1;32");
            print_colored(" remove", "0");
            print_colored(" <dependency_name>", "1;36");
            println!();
            print_colored("    Remove a dependency from the Makefile", "0");
            println!();
            println!();
            print_colored("sticks", "1;32");
            print_colored(" update", "0");
            println!();
            print_colored("    Update sticks to the latest version", "0");
            println!();
            println!();
        }
        _ => println!("Unknown command"),
    }
}
