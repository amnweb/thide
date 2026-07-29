use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, LPARAM, WPARAM};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW,
    HKEY_CURRENT_USER, KEY_SET_VALUE, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, WM_APP};

// Custom message IDs for IPC
const WM_THIDE_SHOW: u32 = WM_APP + 1;
const WM_THIDE_HIDE: u32 = WM_APP + 2;
const WM_THIDE_QUIT: u32 = WM_APP + 3;
const WM_THIDE_TOGGLE: u32 = WM_APP + 4;

const IPC_WINDOW_CLASS: &str = "THideIPCWindow";

pub fn handle_cli_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        print_usage();
        return Ok(());
    }

    match args[0].to_lowercase().as_str() {
        "start" => start_gui(),
        "show" => send_command(WM_THIDE_SHOW, "Showing taskbar..."),
        "hide" => send_command(WM_THIDE_HIDE, "Hiding taskbar..."),
        "toggle" => send_command(WM_THIDE_TOGGLE, "Toggling taskbar state..."),
        "stop" | "quit" => send_command(WM_THIDE_QUIT, "Stopping THide..."),
        "enable-autostart" => enable_autostart(),
        "disable-autostart" => disable_autostart(),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        _ => {
            eprintln!("Unknown command: {}", args[0]);
            print_usage();
            std::process::exit(1);
        }
    }
}

pub fn get_ipc_window_class() -> &'static str {
    IPC_WINDOW_CLASS
}

pub const fn get_message_ids() -> (u32, u32, u32, u32) {
    (WM_THIDE_SHOW, WM_THIDE_HIDE, WM_THIDE_TOGGLE, WM_THIDE_QUIT)
}

/// Send an IPC command to the running THide instance
fn send_command(message: u32, success_msg: &str) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let class_name: Vec<u16> = format!("{}\0", IPC_WINDOW_CLASS).encode_utf16().collect();

        match FindWindowW(
            windows::core::PCWSTR(class_name.as_ptr()),
            windows::core::PCWSTR::null(),
        ) {
            Ok(hwnd) if !hwnd.0.is_null() => {
                let _ = PostMessageW(hwnd, message, WPARAM(0), LPARAM(0));
                println!("{}", success_msg);
                Ok(())
            }
            _ => {
                eprintln!("Error: THide is not running!");
                std::process::exit(1);
            }
        }
    }
}

/// Check if THide is currently running
fn is_thide_running() -> bool {
    unsafe {
        let class_name: Vec<u16> = format!("{}\0", IPC_WINDOW_CLASS).encode_utf16().collect();

        matches!(
            FindWindowW(
                windows::core::PCWSTR(class_name.as_ptr()),
                windows::core::PCWSTR::null(),
            ),
            Ok(hwnd) if !hwnd.0.is_null()
        )
    }
}

/// Start THide in GUI mode
fn start_gui() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;

    if is_thide_running() {
        println!("THide is already running.");
        return Ok(());
    }

    let exe_path = std::env::current_exe()?;
    Command::new(exe_path).spawn()?;

    println!("Starting THide...");
    Ok(())
}

/// Enable THide to start automatically on Windows login
fn enable_autostart() -> Result<(), Box<dyn std::error::Error>> {
    let exe_path = std::env::current_exe()?;
    let exe_path_str = exe_path.to_string_lossy();

    let key_path: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0"
        .encode_utf16()
        .collect();
    let value_name: Vec<u16> = "THide\0".encode_utf16().collect();
    let mut value_data: Vec<u16> = exe_path_str.encode_utf16().collect();
    value_data.push(0);
    let value_bytes = unsafe {
        std::slice::from_raw_parts(
            value_data.as_ptr() as *const u8,
            value_data.len() * std::mem::size_of::<u16>(),
        )
    };

    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        let err = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR(key_path.as_ptr()),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        );
        if err != ERROR_SUCCESS {
            eprintln!("Failed to open registry key (code {})", err.0);
            std::process::exit(1);
        }
        let set_err = RegSetValueExW(
            hkey,
            windows::core::PCWSTR(value_name.as_ptr()),
            0,
            REG_SZ,
            Some(value_bytes),
        );
        let _ = RegCloseKey(hkey);
        if set_err != ERROR_SUCCESS {
            eprintln!("Failed to set registry value (code {})", set_err.0);
            std::process::exit(1);
        }
    }

    println!("✓ Autostart enabled successfully!");
    println!("  THide will start automatically when you log in.");
    Ok(())
}

/// Disable THide autostart on Windows login
fn disable_autostart() -> Result<(), Box<dyn std::error::Error>> {
    let key_path: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0"
        .encode_utf16()
        .collect();
    let value_name: Vec<u16> = "THide\0".encode_utf16().collect();

    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        let open_err = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR(key_path.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        );
        if open_err != ERROR_SUCCESS {
            // Key doesn't exist — autostart was never enabled
            println!("Autostart was not enabled.");
            return Ok(());
        }
        let del_err = RegDeleteValueW(hkey, windows::core::PCWSTR(value_name.as_ptr()));
        let _ = RegCloseKey(hkey);
        if del_err == ERROR_SUCCESS {
            println!("✓ Autostart disabled successfully!");
        } else if del_err == ERROR_FILE_NOT_FOUND {
            println!("Autostart was not enabled.");
        } else {
            eprintln!("Failed to delete registry value (code {})", del_err.0);
            std::process::exit(1);
        }
    }
    Ok(())
}

/// Display CLI usage information
fn print_usage() {
    println!("THide - Taskbar Hide Utility");
    println!();
    println!("USAGE:");
    println!("    thide [COMMAND]");
    println!();
    println!("COMMANDS:");
    println!("    start              Start THide in GUI mode");
    println!("    show               Show the taskbar (if THide is running)");
    println!("    hide               Hide the taskbar (if THide is running)");
    println!(
        "    toggle             Hide the taskbar if visible, show if hidden (if THide is running)"
    );
    println!("    stop               Stop THide and restore taskbar");
    println!("    enable-autostart   Enable autostart on login");
    println!("    disable-autostart  Disable autostart on login");
    println!("    help               Show this help message");
}
