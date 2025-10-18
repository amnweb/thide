# Taskbar Hide

A lightweight Windows 10/11 application to hide/show the taskbar with system tray and CLI control.

## Features

- 🎯 **Hide/Show Windows 10/11 Taskbar** - Complete control over taskbar visibility
- 🖱️ **System Tray Icon** - Easy access from system tray with menu
- ⌨️ **CLI Support** - Command-line interface for automation
- 🔒 **Single Instance** - Prevents multiple instances from running
- 🎨 **YASB Compatible** - Works with YASB and other custom status bars
- ⚡ **Lightweight** - Only ~500 KB, minimal resource usage
- 🚀 **No Dependencies** - Self-contained executable

## Download

Get the latest release from the [Releases page](../../releases).

## Installation

1. Download `thide.exe` from the latest release
2. Place it anywhere on your system (e.g., `C:\Tools\`)
3. Run the executable - no installation required!

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
# Start THide in GUI mode
.\thide.exe start

# Show the taskbar (if app is running)
.\thide.exe show

# Hide the taskbar (if app is running)
.\thide.exe hide

# Stop the app and restore taskbar
.\thide.exe stop

# Enable autostart on Windows login
.\thide.exe enable-autostart

# Disable autostart
.\thide.exe disable-autostart

# Show help
.\thide.exe help
```

**Notes:**
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

### Build Steps

```powershell
# Clone the repository
git clone https://github.com/amnweb/thide.git
cd thide
cargo install --path . --locked
```

## Compatibility

- ✅ Windows 11 (Primary target)
- ✅ Windows 10 (Should work)
- ✅ [YASB](https://github.com/amnweb/yasb) (Yet Another Status Bar)
- ✅ Other custom status bars using `Shell_TrayWnd` class name

## Troubleshooting

**Taskbar won't hide:**
- Ensure you're running the latest version
- Check if another taskbar tool is interfering
- Try running as administrator (usually not needed)

**App won't start / "Already running" message:**
- Check system tray - the app might already be running
- Kill any existing `thide.exe` processes in Task Manager

**YASB/Custom status bar disappears:**
- This should NOT happen - the app filters by process name
- Please report as a bug with details
