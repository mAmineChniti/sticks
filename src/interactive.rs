use crate::{Language, BuildSystem};
use anyhow::Result;
use std::io::{self, Read, Write};

pub fn run_interactive() -> Result<()> {
	println!("\n🎯 Welcome to sticks - Interactive Mode\n");

	// Step 1: Get project name
	print!("📝 Enter project name: ");
	io::stdout().flush()?;
	let mut project_name = String::new();
	io::stdin().read_line(&mut project_name)?;
	let project_name = project_name.trim().to_string();

	if project_name.is_empty() {
		anyhow::bail!("Project name cannot be empty");
	}

	// Step 2: Select language
	println!();
	let language = select_language_interactive()?;

	// Step 3: Select build system
	println!();
	let build_system = select_build_system_interactive()?;

	// Step 4: Create project
	println!("\n🔨 Creating {} project: {}", language, project_name);
	crate::create_project_with_system(&project_name, language, build_system)?;
	println!("✅ Project created successfully!\n");

	Ok(())
}

/// Interactive language selector with arrow keys and enter
pub fn select_language_interactive() -> Result<Language> {
	println!("Choose language:");
	let options = vec!["C", "C++"];
	let selected = interactive_select(&options)?;

	Ok(match selected {
		0 => Language::C,
		1 => Language::Cpp,
		_ => unreachable!(),
	})
}

/// Interactive build system selector with arrow keys and enter
pub fn select_build_system_interactive() -> Result<BuildSystem> {
	println!("Choose build system:");
	let options = vec!["Makefile", "CMake"];
	let selected = interactive_select(&options)?;

	Ok(match selected {
		0 => BuildSystem::Makefile,
		1 => BuildSystem::CMake,
		_ => unreachable!(),
	})
}

/// Generic interactive selector - shows options and allows arrow keys / enter selection
fn interactive_select(options: &[&str]) -> Result<usize> {
	let mut selected = 0;
	let num_options = options.len();

	// Enable raw mode
	let _guard = RawModeGuard::new()?;

	// Initial display
	display_options(options, selected)?;

	loop {
		// Read input
		let input = read_key()?;

		match input.as_str() {
			"UP" => {
				if selected > 0 {
					selected -= 1;
					// Move cursor up and redraw
					move_cursor_up(num_options)?;
					display_options(options, selected)?;
				}
			}
			"DOWN" => {
				if selected < num_options - 1 {
					selected += 1;
					// Move cursor up and redraw
					move_cursor_up(num_options)?;
					display_options(options, selected)?;
				}
			}
			"ENTER" => {
				// Move cursor to end of list
				print!("\n");
				io::stdout().flush()?;
				break;
			}
			_ => {}
		}
	}

	Ok(selected)
}

fn display_options(options: &[&str], selected: usize) -> Result<()> {
	for (i, option) in options.iter().enumerate() {
		if i == selected {
			println!("❯ {}", option);
		} else {
			println!("  {}", option);
		}
	}
	io::stdout().flush()?;
	Ok(())
}

fn move_cursor_up(lines: usize) -> Result<()> {
	for _ in 0..lines {
		print!("\x1B[A\x1B[2K");
	}
	io::stdout().flush()?;
	Ok(())
}

/// RAII guard to manage raw mode
struct RawModeGuard {
	original_termios: libc::termios,
}

impl RawModeGuard {
	fn new() -> Result<Self> {
		use std::os::unix::io::AsRawFd;

		let stdin_fd = io::stdin().as_raw_fd();
		let original_termios = unsafe {
			let mut t = std::mem::zeroed::<libc::termios>();
			libc::tcgetattr(stdin_fd, &mut t);
			t
		};

		let mut new_termios = original_termios;
		new_termios.c_lflag &= !(libc::ICANON | libc::ECHO);
		new_termios.c_cc[libc::VMIN] = 1;
		new_termios.c_cc[libc::VTIME] = 0;

		unsafe {
			libc::tcsetattr(stdin_fd, libc::TCSADRAIN, &new_termios);
		}

		Ok(RawModeGuard { original_termios })
	}
}

impl Drop for RawModeGuard {
	fn drop(&mut self) {
		use std::os::unix::io::AsRawFd;

		let stdin_fd = io::stdin().as_raw_fd();
		unsafe {
			libc::tcsetattr(stdin_fd, libc::TCSADRAIN, &self.original_termios);
		}
	}
}

/// Read a single key press (handles arrow keys)
fn read_key() -> Result<String> {
	let mut buf = [0; 3];

	// Set non-blocking read with timeout
	let n = io::stdin().read(&mut buf)?;

	if n == 0 {
		return Ok(String::new());
	}

	// Check for escape sequence (arrow keys)
	if n >= 3 && buf[0] == 27 && buf[1] == 91 {
		match buf[2] {
			b'A' => return Ok("UP".to_string()),
			b'B' => return Ok("DOWN".to_string()),
			_ => {}
		}
	}

	// Check for Enter key
	if buf[0] == b'\n' || buf[0] == b'\r' {
		return Ok("ENTER".to_string());
	}

	Ok(String::new())
}

// Backward compatibility functions for main.rs
pub fn select_language() -> Language {
	match select_language_interactive() {
		Ok(lang) => lang,
		Err(_) => Language::C,
	}
}

pub fn select_build_system() -> Result<BuildSystem> {
	select_build_system_interactive()
}
