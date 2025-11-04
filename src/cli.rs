use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, WM_APP};

// Custom message IDs for IPC
const WM_THIDE_SHOW: u32 = WM_APP + 1;
const WM_THIDE_HIDE: u32 = WM_APP + 2;
const WM_THIDE_QUIT: u32 = WM_APP + 3;

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

pub const fn get_message_ids() -> (u32, u32, u32) {
    (WM_THIDE_SHOW, WM_THIDE_HIDE, WM_THIDE_QUIT)
}

/// Get cached IPC window class name as UTF-16
pub fn get_ipc_window_class_utf16() -> &'static [u16] {
    use std::sync::OnceLock;
    static IPC_CLASS_UTF16: OnceLock<Vec<u16>> = OnceLock::new();
    IPC_CLASS_UTF16.get_or_init(|| format!("{}\0", IPC_WINDOW_CLASS).encode_utf16().collect())
}

/// Send an IPC command to the running THide instance
fn send_command(message: u32, success_msg: &str) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let class_name = get_ipc_window_class_utf16();

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
        let class_name = get_ipc_window_class_utf16();

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
    use std::process::Command;

    let exe_path = std::env::current_exe()?;
    let exe_path_str = exe_path.to_string_lossy();

    let output = Command::new("reg")
        .args([
            "add",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            "/v",
            "THide",
            "/t",
            "REG_SZ",
            "/d",
            &exe_path_str,
            "/f",
        ])
        .output()?;

    if output.status.success() {
        println!("✓ Autostart enabled successfully!");
        println!("  THide will start automatically when you log in.");
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        eprintln!("Failed to enable autostart: {}", error);
        std::process::exit(1);
    }
}

/// Disable THide autostart on Windows login
fn disable_autostart() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;

    let output = Command::new("reg")
        .args([
            "delete",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            "/v",
            "THide",
            "/f",
        ])
        .output()?;

    if output.status.success() {
        println!("✓ Autostart disabled successfully!");
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        if error.contains("unable to find") || error.contains("does not exist") {
            println!("Autostart was not enabled.");
            Ok(())
        } else {
            eprintln!("Failed to disable autostart: {}", error);
            std::process::exit(1)
        }
    }
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
    println!("    stop               Stop THide and restore taskbar");
    println!("    enable-autostart   Enable autostart on login");
    println!("    disable-autostart  Disable autostart on login");
    println!("    help               Show this help message");
}
