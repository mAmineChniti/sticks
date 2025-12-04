use crate::{BuildSystem, Language};
use anyhow::Result;
use std::io::{self, Read, Write};

pub fn run_interactive() -> Result<()> {
	println!("\n🎯 Welcome to sticks - Interactive Mode\n");

	print!("📝 Enter project name: ");
	io::stdout().flush()?;
	let mut project_name = String::new();
	io::stdin().read_line(&mut project_name)?;
	let project_name = project_name.trim().to_string();

	if project_name.is_empty() {
		anyhow::bail!("Project name cannot be empty");
	}

	let language = select_language_interactive()?;
	let build_system = select_build_system_interactive()?;

	println!("\n🔨 Creating {} project: {}", language, project_name);
	crate::new_project_with_system(&project_name, language, build_system)?;
	println!("✅ Project created successfully!\n");

	Ok(())
}

pub fn select_language_interactive() -> Result<Language> {
	let options = vec!["C", "C++"];
	let selected = interactive_select(&options)?;

	Ok(match selected {
		0 => Language::C,
		1 => Language::Cpp,
		_ => unreachable!(),
	})
}

pub fn select_build_system_interactive() -> Result<BuildSystem> {
	let options = vec!["Makefile", "CMake"];
	let selected = interactive_select(&options)?;

	Ok(match selected {
		0 => BuildSystem::Makefile,
		1 => BuildSystem::CMake,
		_ => unreachable!(),
	})
}


fn interactive_select(options: &[&str]) -> Result<usize> {
	let mut selected = 0;
	let num_options = options.len();

	let _guard = RawModeGuard::new()?;

	display_options(options, selected)?;

	loop {
		let input = read_key()?;

		match input.as_str() {
			"UP" => {
				if selected > 0 {
					selected -= 1;
					move_cursor_up(num_options)?;
					display_options(options, selected)?;
				}
			}
			"DOWN" => {
				if selected < num_options - 1 {
					selected += 1;
					move_cursor_up(num_options)?;
					display_options(options, selected)?;
				}
			}
			"ENTER" => {
				move_cursor_up(num_options)?;
				println!("❯ {}", options[selected]);
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

fn read_key() -> Result<String> {
	let mut buf = [0; 3];

	let n = io::stdin().read(&mut buf)?;

	if n == 0 {
		return Ok(String::new());
	}

	if n >= 3 && buf[0] == 27 && buf[1] == 91 {
		match buf[2] {
			b'A' => return Ok("UP".to_string()),
			b'B' => return Ok("DOWN".to_string()),
			_ => {}
		}
	}

	if buf[0] == b'\n' || buf[0] == b'\r' {
		return Ok("ENTER".to_string());
	}

	Ok(String::new())
}

pub fn select_language() -> Language {
	match select_language_interactive() {
		Ok(lang) => lang,
		Err(_) => Language::C,
	}
}

pub fn select_build_system() -> Result<BuildSystem> {
	select_build_system_interactive()
}
