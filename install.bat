@echo off
setlocal EnableExtensions EnableDelayedExpansion

REM Exit immediately if a command exits with a non-zero status
REM Batch doesn't have a direct equivalent of 'set -e', so we'll handle errors manually.

REM Function: Display a progress bar
:progress_bar
set "duration=%1"
set "steps=100"
set /A "increment=duration * 10"  REM Convert to milliseconds for timeout
for /L %%i in (0,1,%steps%) do (
    set /A "completed=%%i"
    set "bar=["
    for /L %%j in (1,1,%%i) do set "bar=!bar!=!"
    for /L %%j in (%%i,1,%steps%) do set "bar=!bar! "
    set "bar=!bar!]"
    <nul set /p ="!bar! %%i/%steps%]`r"
    timeout /nobreak /t 0.1 >nul
)
echo.
goto :eof

REM Function: Compare version numbers
:version_gt
set "ver1=%1"
set "ver2=%2"

for /f "tokens=1-3 delims=." %%a in ("%ver1%") do (
    set "v1_1=%%a"
    set "v1_2=%%b"
    set "v1_3=%%c"
)

for /f "tokens=1-3 delims=." %%a in ("%ver2%") do (
    set "v2_1=%%a"
    set "v2_2=%%b"
    set "v2_3=%%c"
)

if not defined v2_2 set "v2_2=0"
if not defined v2_3 set "v2_3=0"

if %v1_1% GTR %v2_1% (
    exit /b 1
) else if %v1_1% LSS %v2_1% (
    exit /b 0
) else (
    if %v1_2% GTR %v2_2% (
        exit /b 1
    ) else if %v1_2% LSS %v2_2% (
        exit /b 0
    ) else (
        if %v1_3% GTR %v2_3% (
            exit /b 1
        ) else (
            exit /b 0
        )
    )
)
goto :eof

REM Get the version from the output of sticks -v
for /f "tokens=2" %%a in ('sticks -v 2^>nul') do set "local_version=%%a"

if defined local_version (
    REM Fetch the version from Cargo.toml in the repository
    for /f "tokens=2 delims= " %%a in ('curl -s https://raw.githubusercontent.com/mAmineChniti/sticks/master/Cargo.toml ^| findstr /R "^version"') do set "cargo_toml_version=%%a"
    REM Remove quotes if present
    set "cargo_toml_version=%cargo_toml_version:"=%"

    call :version_gt "%cargo_toml_version%" "%local_version%"
    set "is_newer=%ERRORLEVEL%"
    if %is_newer%==0 (
        echo %local_version%
        echo Latest version of sticks is already installed.
        exit /b 0
    ) else (
        echo %cargo_toml_version% > %local_version%
        echo Newer version of sticks has been found.
    )
)

REM Display progress bar for Rust installation check
echo Checking if Rust is installed...
call :progress_bar 3

REM Check if Rust is installed
where rustc >nul 2>&1
if errorlevel 1 (
    echo Rust is not installed. Please install Rust from https://www.rust-lang.org/ before running this script.
    exit /b 1
)

REM Get the host value from the output of `rustc -vV`
for /f "tokens=2 delims=:" %%a in ('rustc -vV ^| findstr /C:"host:"') do set "host=%%a"
REM Trim spaces
set "host=%host:~1%"

REM Batch doesn't have dpkg. Assuming Windows environment, skip build-essential.
REM Alternatively, check for necessary build tools if applicable.

REM Display progress bar for temporary directory creation
echo Creating a temporary directory...
call :progress_bar 3

REM Create a temporary directory and change into it
set "temp_dir=%TEMP%\sticks_build_%RANDOM%"
mkdir "%temp_dir%" || (
    echo Error: Unable to create temporary directory.
    exit /b 1
)
pushd "%temp_dir%"

REM Display progress bar for repository cloning
echo Cloning the Git repository...
call :progress_bar 5

REM Clone the Git repository
git clone https://github.com/mAmineChniti/sticks.git >nul 2>&1 || (
    echo Error: Unable to clone the Git repository.
    popd
    rd /s /q "%temp_dir%"
    exit /b 1
)

REM Change into the cloned directory
cd sticks

REM Build the project with Cargo in release mode
echo Building the project with Cargo...
cargo build --release >nul 2>&1 || (
    echo Error: Cargo build failed.
    popd
    rd /s /q "%temp_dir%"
    exit /b 1
)
call :progress_bar 10

REM Install cargo-deb if not already installed
where cargo-deb >nul 2>&1
if errorlevel 1 (
    echo Installing cargo-deb...
    cargo install cargo-deb >nul 2>&1 || (
        echo Error: Unable to install cargo-deb.
        popd
        rd /s /q "%temp_dir%"
        exit /b 1
    )
    call :progress_bar 10
)

REM Detect the OS name
set "os_name=windows"

REM Build a package using cargo-deb for the detected OS
echo Building a package for %os_name%/%host% using cargo-deb...
cargo deb --target "%host%" >nul 2>&1 || (
    echo Error: Cargo deb failed.
    popd
    rd /s /q "%temp_dir%"
    exit /b 1
)
call :progress_bar 10

REM Install the generated package using the appropriate package manager
REM Windows doesn't use .deb packages. Skipping installation step.
echo Skipping package installation. Please install the package manually if needed.

REM Clean up by removing the temporary directory
popd
rd /s /q "%temp_dir%"

REM Optionally, print a message to confirm the installation
sticks -v
echo is now installed.

endlocal
exit /b 0
