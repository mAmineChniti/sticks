use crate::{BuildSystem, Language};
use anyhow::{Context, Result};
use std::io::{self, Read, Write};
#[cfg(unix)]
use std::os::unix::io::AsRawFd;

pub fn run_interactive() -> Result<()> {
	println!("\n🎯 Welcome to sticks - Interactive Mode\n");

	print!("📝 Enter project name: ");
	io::stdout()
		.flush()
		.context("Failed to flush project name prompt")?;
	let mut project_name = String::new();
	let bytes_read = io::stdin()
		.read_line(&mut project_name)
		.context("Failed to read project name")?;
	if bytes_read == 0 {
		anyhow::bail!("No project name was provided");
	}
	let project_name = project_name.trim().to_string();
	validate_project_name(&project_name)?;

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

fn validate_project_name(name: &str) -> Result<()> {
	crate::validate_project_name(name)
}

fn interactive_select(options: &[&str]) -> Result<usize> {
	if options.is_empty() {
		anyhow::bail!("No interactive options were provided");
	}

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
			"INTERRUPT" => {
				anyhow::bail!("Interactive selection cancelled");
			}
			_ => {}
		}
	}

	Ok(selected)
}

fn display_options(options: &[&str], selected: usize) -> Result<()> {
	for (index, option) in options.iter().enumerate() {
		if index == selected {
			println!("❯ {}", option);
		} else {
			println!("  {}", option);
		}
	}
	io::stdout()
		.flush()
		.context("Failed to flush interactive options")
}

fn move_cursor_up(lines: usize) -> Result<()> {
	for _ in 0..lines {
		print!("\x1B[A\x1B[2K");
	}
	io::stdout()
		.flush()
		.context("Failed to update interactive display")
}

#[cfg(unix)]
struct RawModeGuard {
	original_termios: libc::termios,
}

#[cfg(unix)]
impl RawModeGuard {
	fn new() -> Result<Self> {
		let stdin_fd = io::stdin().as_raw_fd();
		let mut original_termios = std::mem::MaybeUninit::<libc::termios>::uninit();
		let result = unsafe { libc::tcgetattr(stdin_fd, original_termios.as_mut_ptr()) };
		if result != 0 {
			return Err(io::Error::last_os_error()).context("Failed to read terminal settings");
		}
		let original_termios = unsafe { original_termios.assume_init() };
		let mut new_termios = original_termios;
		new_termios.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);
		new_termios.c_cc[libc::VMIN] = 1;
		new_termios.c_cc[libc::VTIME] = 0;

		let result = unsafe { libc::tcsetattr(stdin_fd, libc::TCSADRAIN, &new_termios) };
		if result != 0 {
			return Err(io::Error::last_os_error()).context("Failed to enable raw terminal mode");
		}

		Ok(Self { original_termios })
	}
}

#[cfg(unix)]
impl Drop for RawModeGuard {
	fn drop(&mut self) {
		let stdin_fd = io::stdin().as_raw_fd();
		let _ = unsafe { libc::tcsetattr(stdin_fd, libc::TCSADRAIN, &self.original_termios) };
	}
}

#[cfg(not(unix))]
struct RawModeGuard;

#[cfg(not(unix))]
impl RawModeGuard {
	fn new() -> Result<Self> {
		anyhow::bail!("Interactive selection is not supported on this platform")
	}
}

fn read_key() -> Result<String> {
	let mut first = [0; 1];
	let bytes_read = io::stdin()
		.read(&mut first)
		.context("Failed to read interactive input")?;
	if bytes_read == 0 {
		anyhow::bail!("Interactive input ended before a selection was made");
	}

	match first[0] {
		b'\n' | b'\r' => return Ok("ENTER".to_string()),
		3 => return Ok("INTERRUPT".to_string()),
		27 => {
			let Some(second) = read_escape_byte()? else {
				return Ok(String::new());
			};
			if second != b'[' {
				return Ok(String::new());
			}
			let Some(third) = read_escape_byte()? else {
				return Ok(String::new());
			};
			return Ok(match third {
				b'A' => "UP".to_string(),
				b'B' => "DOWN".to_string(),
				_ => String::new(),
			});
		}
		_ => {}
	}

	Ok(String::new())
}

#[cfg(unix)]
fn read_escape_byte() -> Result<Option<u8>> {
	let stdin_fd = io::stdin().as_raw_fd();
	let mut descriptor = libc::pollfd {
		fd: stdin_fd,
		events: libc::POLLIN,
		revents: 0,
	};
	let result = unsafe { libc::poll(&mut descriptor, 1, 100) };
	if result == 0 {
		return Ok(None);
	}
	if result < 0 {
		return Err(io::Error::last_os_error()).context("Failed to wait for interactive input");
	}
	let mut byte = [0; 1];
	let bytes_read = io::stdin()
		.read(&mut byte)
		.context("Failed to read interactive escape sequence")?;
	Ok((bytes_read == 1).then_some(byte[0]))
}

#[cfg(not(unix))]
fn read_escape_byte() -> Result<Option<u8>> {
	let mut byte = [0; 1];
	let bytes_read = io::stdin()
		.read(&mut byte)
		.context("Failed to read interactive escape sequence")?;
	Ok((bytes_read == 1).then_some(byte[0]))
}

pub fn select_language() -> Language {
	select_language_result().unwrap_or(Language::C)
}

pub fn select_language_result() -> Result<Language> {
	select_language_interactive()
}

pub fn select_build_system() -> Result<BuildSystem> {
	select_build_system_interactive()
}
