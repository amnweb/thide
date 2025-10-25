# Taskbar Hide

A lightweight Windows 10/11 application to hide/show the taskbar with system tray and CLI control.

## Features

- 🎯 **Hide/Show Windows 10/11 Taskbar** - Complete control over taskbar visibility
- 🖱️ **System Tray Icon** - Easy access from system tray with menu
- ⌨️ **CLI Support** - Command-line interface for automation
- 🔒 **Single Instance** - Prevents multiple instances from running
- 🎨 **YASB Compatible** - Works with YASB and other custom status bars
- ⚡ **Lightweight** - ~600 KB, minimal resource usage
- 🚀 **No Dependencies** - Self-contained executable with static CRT linking
- 💻 **Multi-Architecture** - Available for x64 and ARM64 Windows

## Download

Get the latest release from the [Releases page](../../releases).

### Available Formats

- **MSI Installer** (Recommended) - Installs to Program Files and adds to PATH automatically
- **Portable ZIP** - Standalone package with executable, LICENSE, and README - no installation needed

### Architectures

- **x64** - For Intel/AMD 64-bit systems (most common)
- **ARM64** - For ARM-based Windows devices (e.g., Surface Pro X, Snapdragon PCs)

## Installation

### Option 1: MSI Installer (Recommended)

1. Download `thide-x64.msi` or `thide-arm64.msi` from the [Releases page](../../releases)
2. Double-click the MSI file and follow the installation wizard
3. The application will be installed to `C:\Program Files\thide\bin\`
4. **PATH is configured automatically** - you can run `thide` from any command prompt/PowerShell window
5. Start menu shortcut is created automatically

**Uninstall:** Use "Add or Remove Programs" in Windows Settings

### Option 2: Portable ZIP

1. Download `thide-x64-portable.zip` or `thide-arm64-portable.zip` from the [Releases page](../../releases)
2. Extract the ZIP file to any location on your system (e.g., `C:\Tools\thide\`)
3. Run `thide.exe` from the extracted folder - no installation required!
4. The ZIP includes:
   - `thide.exe` - The application
   - `LICENSE.txt` - License information
   - `README-PORTABLE.txt` - Quick start guide
5. (Optional) Add the folder to your PATH to use CLI commands globally

## Usage

### GUI Mode

Double-click `thide.exe` to run in system tray mode:

- The app will hide the taskbar and run in the background
- Look for the icon in your system tray
- Right-click the tray icon to access the menu:
  - **Show Taskbar** - Make taskbar visible
  - **Hide Taskbar** - Hide the taskbar
  - **Quit** - Exit and restore taskbar

### CLI Mode

Control the running app from the command line:

```powershell
# If installed via MSI, you can run from anywhere:
thide start

# If using portable exe, run from the directory or add to PATH:
.\thide.exe start

# Show the taskbar (if app is running)
thide show

# Hide the taskbar (if app is running)
thide hide

# Stop the app and restore taskbar
thide stop

# Enable autostart on Windows login
thide enable-autostart

# Disable autostart
thide disable-autostart

# Show help
thide help
```

**Notes:**
- **MSI users**: The `thide` command works from any location (added to PATH automatically)
- **Portable users**: Run `.\thide.exe` from the directory, or add the folder to your PATH manually
- The `start` command launches THide in GUI mode if it's not already running
- Control commands (show/hide/stop) require the GUI app to be running
- Autostart commands use Windows registry

### Autostart

Use the built-in CLI command to add THide to Windows startup:

```powershell
# Enable autostart (adds registry entry)
.\thide.exe enable-autostart

# Disable autostart (removes registry entry)
.\thide.exe disable-autostart
```

This adds an entry to `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`.

## Building from Source

### Prerequisites

- [Rust](https://www.rust-lang.org/) (stable toolchain)
- Windows 10/11
- (Optional) [WiX Toolset](https://wixtoolset.org/) for building MSI installers

### Build Steps

```powershell
# Clone the repository
git clone https://github.com/amnweb/thide.git
cd thide
cargo build --release
```

### Cross-compile for ARM64 (on x64 machine)

```powershell
# Add ARM64 target
rustup target add aarch64-pc-windows-msvc

# Build for ARM64
cargo build --release --target aarch64-pc-windows-msvc

# The executable will be at: target\aarch64-pc-windows-msvc\release\thide.exe
```

## Compatibility

- ✅ Windows 11 (Primary target)
- ✅ Windows 10 (Should work)
- ✅ Windows on ARM64 (Native ARM64 builds available)
- ✅ [YASB](https://github.com/amnweb/yasb) (Yet Another Status Bar)
- ✅ Other custom status bars using `Shell_TrayWnd` class name

## Troubleshooting

### Taskbar won't hide

- Ensure you're running the latest version
- Check if another taskbar tool is interfering
- Try running as administrator (usually not needed)

### App won't start / "Already running" message

- Check system tray - the app might already be running
- Kill any existing `thide.exe` processes in Task Manager

### YASB/Custom status bar disappears

- This should NOT happen - the app filters by process name
- Please report as a bug with details
