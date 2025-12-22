pub mod clang;
pub mod editor;
pub mod git;
pub mod hooks;
pub mod readme;
pub mod vscode;

pub use clang::generate_clang_format_config;
pub use editor::generate_editorconfig;
pub use git::{generate_gitattributes, generate_gitignore};
pub use hooks::generate_precommit_hook;
pub use readme::generate_readme;
pub use vscode::{
	generate_vscode_launch_config, generate_vscode_settings, generate_vscode_tasks_config,
};
