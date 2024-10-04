<p align="center">
<a href="https://crates.io/crates/sticks" rel="noopener noreferrer">
<img src="sticks.png" alt="sticks Logo" height="150" width="150"/>
</a>
</p>
<h1 align="center">sticks</h1>

Sticks is a Rust command-line tool for managing C and C++ projects. It simplifies the process of creating new projects and managing dependencies in your Makefile.

## Features

- Create new C and C++ projects with a single command.
- Generate a basic project structure with source files and a Makefile.
- Automatically set up "Hello, World!" code in the chosen language.
- Easily add and remove dependencies in your Makefile.

**Before proceeding with the quick install, please make sure you have Rust installed. If you don't have Rust installed, you can download and install it from the official [Rust website](https://www.rust-lang.org/tools/install).**

## Quick Install

```bash
curl -fsSL https://rebrand.ly/tyzot1g | bash
```

**OR**

```bash
cargo install sticks
```

## Quick Uninstall

```bash
sudo apt remove sticks -y
```

**OR**

```bash
cargo uninstall sticks
```

## Build from Source

To use `sticks`, you'll need to build the project:

1. Clone the repository:

    ```bash
    git clone https://github.com/mAmineChniti/sticks.git
   ```

2. Change the current directory to the project folder:

    ```bash
    cd sticks
    ```

3. Build the project using cargo:

    ```bash
    cargo build --release
    ```

4. Once the build is complete, you can find the binary in the target/release directory. You can add this directory to your system's PATH for convenient usage.

## Usage

### Creating a New Project

To create a new C project, use the following command:

```bash
sticks c <project_name>
```

To create a new C++ project, use the following command:

```bash
sticks cpp <project_name>
```

Replace <project_name> with the name of your project.

These commands will create a new project directory with the specified name, set up a source file, and create a basic Makefile. The source file will contain a "Hello, World!" program in C++ or C.

### Initializing a Project in the Current Directory

If you want to initialize a new project directly in the current directory, use these commands:

To initialize a new C project in the current directory, use:

```bash
sticks init c
```

To initialize a new C++ project in the current directory, use:

```bash
sticks init cpp
```

These commands will create a new project based on the current directory name, set up a source file, and create a basic Makefile. The source file will contain a "Hello, World!" program in C++ or C, depending on the chosen project type.

### Adding a Dependency

To add a dependency to your project's Makefile, use the following command:

```bash
sticks add <dependency_name>
```

Replace <dependency_name> with the name of the dependency you want to add. Sticks will automatically modify your Makefile to include the new dependency. If the install-deps rule doesn't exist in your Makefile, Sticks will create it for you.

### Adding source files

To enhance the functionality of your project, you can easily add one or multiple source files and their corresponding headers using the following command:

```bash
sticks src <source_names>
```

Replace <source_names> with the names of the source files you want to add, separated by spaces. Sticks will intelligently update your project structure, including the necessary modifications to your Makefile.

### List Subcommands

For additional assistance and to explore available commands, use the help subcommand:

```bash
sticks help
```

### Updating Sticks

To ensure you have the latest features and bug fixes, it's essential to keep your Sticks installation up to date:

```bash
sticks update
```

## To-Do List

- [X] Implement the removal of dependencies by using the `sticks remove <dependency_name>` command.
- [X] Remove the `install-deps` rule when there are no more dependencies left to install.
- [ ] Modularize the code, put the smaller functions into a commonly used mod, keep only the functions that correspond to subcommands in main.
- [X] Add the ability to add multiple dependencies using `sticks add`.
- [X] Set up a CI/CD pipeline for the project to automate testing, building, deployment and releases processes.

## Contributing

If you'd like to contribute to Sticks or report issues We welcome contributions and feedback from the community, all you have to is open an issue or fork this repo to contribute.

Fore more details visit [CONTRIBUTING](CONTRIBUTING.md)

## License

This project is licensed under the MIT License. See the [LICENSE file](https://github.com/mAmineChniti/sticks/blob/master/LICENSE) for details.

For any inquiries, feel free to email us at [emin.chniti@esprit.tn](mailto:emin.chniti@esprit.tn).
