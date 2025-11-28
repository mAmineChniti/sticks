//! Centralized constants for the sticks project
//! This module contains all magic strings and configuration values

/// Makefile configuration
pub mod makefile {
	pub const FILENAME: &str = "Makefile";
	pub const INSTALL_DEPS_PREFIX: &str = "sudo apt install -y";
	pub const DEFAULT_TARGET: &str = "all: clean";
}

/// GitHub release and versioning
pub mod github {
	pub const REPO_OWNER: &str = "mAmineChniti";
	pub const REPO_NAME: &str = "sticks";
	pub const RELEASE_DOWNLOAD_URL: &str =
		"https://github.com/mAmineChniti/sticks/releases/latest/download";
	pub const RELEASE_API_URL: &str =
		"https://api.github.com/repos/mAmineChniti/sticks/releases/latest";
}

/// Project structure
pub mod project {
	pub const SRC_DIR: &str = "src";
	pub const INCLUDE_DIR: &str = "include";
	pub const BUILD_DIR: &str = "build";
	pub const VSCODE_DIR: &str = ".vscode";
}

/// File extensions
pub mod extensions {
	pub const GITIGNORE: &str = ".gitignore";
	pub const EDITORCONFIG: &str = ".editorconfig";
	pub const CLANG_FORMAT: &str = ".clang-format";
	pub const GIT_ATTRIBUTES: &str = ".gitattributes";
	pub const README: &str = "README.md";
	pub const VSCODE_SETTINGS: &str = "settings.json";
	pub const VSCODE_LAUNCH: &str = "launch.json";
	pub const VSCODE_TASKS: &str = "tasks.json";
}

/// Installation paths
pub mod install_paths {
	pub const USR_LOCAL_BIN: &str = "/usr/local/bin/sticks";
	pub const USR_BIN: &str = "/usr/bin/sticks";
	pub const CARGO_BIN_SUFFIX: &str = ".cargo/bin/sticks";
}
