#![windows_subsystem = "windows"]

mod cli;

use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder,
};
use windows::Win32::Foundation::{
    GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HWND, LPARAM, WPARAM,
};
use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
use windows::Win32::System::Threading::{
    CreateMutexW, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABM_GETSTATE, ABM_SETSTATE, ABS_AUTOHIDE, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, FindWindowExW, GetMessageW,
    GetWindowThreadProcessId, IsWindowVisible, MessageBoxW, PostQuitMessage, RegisterClassW,
    ShowWindow, TranslateMessage, HWND_MESSAGE, MB_ICONWARNING, MB_OK, MSG, SW_HIDE, SW_SHOW,
    WNDCLASSW, WS_OVERLAPPEDWINDOW,
};
use winit::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};

// Constants
const TASKBAR_MONITOR_INTERVAL_MS: u64 = 500; // Reduced from 100ms to 500ms to save CPU cycles
const TASKBAR_CACHE_REFRESH_MS: u64 = 5000; // Refresh taskbar cache every 5 seconds

// IPC Message Types
#[derive(Debug, Clone)]
enum IPCMessage {
    Show,
    Hide,
    Quit,
}

// Cached taskbar handles with timestamp
struct TaskbarCache {
    handles: Vec<HWND>,
    last_updated: std::time::Instant,
}

impl TaskbarCache {
    fn new() -> Self {
        Self {
            handles: Vec::new(),
            last_updated: std::time::Instant::now() - std::time::Duration::from_secs(10),
        }
    }

    fn should_refresh(&self) -> bool {
        self.last_updated.elapsed().as_millis() > TASKBAR_CACHE_REFRESH_MS as u128
    }

    fn update(&mut self, handles: Vec<HWND>) {
        self.handles = handles;
        self.last_updated = std::time::Instant::now();
    }

    fn get(&mut self) -> &[HWND] {
        if self.should_refresh() {
            self.update(find_all_explorer_taskbars_uncached());
        }
        &self.handles
    }
}

// Cache for commonly used UTF-16 strings
struct Utf16StringCache {
    shell_tray_wnd: Vec<u16>,
    shell_secondary_tray_wnd: Vec<u16>,
    explorer_exe: String,
}

impl Utf16StringCache {
    fn new() -> Self {
        Self {
            shell_tray_wnd: "Shell_TrayWnd\0".encode_utf16().collect(),
            shell_secondary_tray_wnd: "Shell_SecondaryTrayWnd\0".encode_utf16().collect(),
            explorer_exe: "explorer.exe".to_string(),
        }
    }
}

// Global event proxy storage for IPC communication
static GLOBAL_EVENT_PROXY: Mutex<Option<EventLoopProxy<IPCMessage>>> = Mutex::new(None);

/// Attach to parent console for CLI mode and ensure it's ready
fn attach_console() -> bool {
    unsafe {
        use windows::Win32::System::Console::{
            AttachConsole, GetConsoleMode, GetStdHandle, ATTACH_PARENT_PROCESS, CONSOLE_MODE,
            STD_OUTPUT_HANDLE,
        };

        // Try to attach to parent console
        if AttachConsole(ATTACH_PARENT_PROCESS).is_err() {
            return false;
        }

        // Verify console is ready by checking if we can get stdout handle
        let stdout = GetStdHandle(STD_OUTPUT_HANDLE);
        if let Ok(handle) = stdout {
            if !handle.is_invalid() {
                // Try to get console mode to ensure console is fully initialized
                let mut mode = CONSOLE_MODE(0);
                GetConsoleMode(handle, &mut mode).is_ok()
            } else {
                false
            }
        } else {
            false
        }
    }
}

/// Get the process name for a given window handle
fn get_process_name(hwnd: HWND) -> Option<String> {
    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));

        let h_process =
            OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).ok()?;

        let mut buffer: [u16; 512] = [0; 512];
        let len = GetModuleBaseNameW(h_process, None, &mut buffer);

        let _ = windows::Win32::Foundation::CloseHandle(h_process);

        if len > 0 {
            Some(String::from_utf16_lossy(&buffer[..len as usize]))
        } else {
            None
        }
    }
}

/// Load the application icon from embedded resources
fn load_icon() -> tray_icon::Icon {
    const ICON_DATA: &[u8] = include_bytes!("../assets/icon.ico");
    load_icon_file(ICON_DATA).expect("Failed to load icon")
}

/// Load an icon from raw ICO file data
fn load_icon_file(data: &[u8]) -> Result<tray_icon::Icon, Box<dyn std::error::Error>> {
    let icon_dir = ico::IconDir::read(std::io::Cursor::new(data))?;

    let entry = icon_dir
        .entries()
        .iter()
        .max_by_key(|e| e.width() as u32 * e.height() as u32)
        .ok_or("No icon entries found")?;

    let image = entry.decode()?;
    let rgba = image.rgba_data().to_vec();
    let width = image.width();
    let height = image.height();

    Ok(tray_icon::Icon::from_rgba(rgba, width, height)?)
}

/// Find all explorer.exe taskbars (primary and secondary monitors) - uncached version
fn find_all_explorer_taskbars_uncached() -> Vec<HWND> {
    thread_local! {
        static STRING_CACHE: Utf16StringCache = Utf16StringCache::new();
    }

    unsafe {
        let mut taskbars = Vec::new();

        STRING_CACHE.with(|cache| {
            let class_names = [
                (&cache.shell_tray_wnd, true),
                (&cache.shell_secondary_tray_wnd, false),
            ];

            for (class_wide, is_primary) in &class_names {
                let mut hwnd = HWND(std::ptr::null_mut());

                loop {
                    match FindWindowExW(
                        HWND(std::ptr::null_mut()),
                        hwnd,
                        windows::core::PCWSTR(class_wide.as_ptr()),
                        windows::core::PCWSTR::null(),
                    ) {
                        Ok(found_hwnd) => {
                            hwnd = found_hwnd;
                            if hwnd.0.is_null() {
                                break;
                            }

                            if let Some(process_name) = get_process_name(hwnd) {
                                if process_name.eq_ignore_ascii_case(&cache.explorer_exe) {
                                    taskbars.push(hwnd);
                                    if *is_primary {
                                        break; // Only one primary taskbar exists
                                    }
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        });

        taskbars
    }
}

/// Find all explorer.exe taskbars (primary and secondary monitors)
/// Note: This is the uncached version for cases where fresh handles are required
fn find_all_explorer_taskbars() -> Vec<HWND> {
    find_all_explorer_taskbars_uncached()
}

/// Check if any taskbar is currently visible (with caching)
fn is_taskbar_visible_cached(cache: &mut TaskbarCache) -> bool {
    unsafe {
        cache
            .get()
            .iter()
            .any(|&hwnd| IsWindowVisible(hwnd).as_bool())
    }
}

/// Check if any taskbar is currently visible (without caching)
fn is_taskbar_visible() -> bool {
    unsafe {
        find_all_explorer_taskbars()
            .into_iter()
            .any(|hwnd| IsWindowVisible(hwnd).as_bool())
    }
}

/// Show or hide all taskbars (with optional caching)
fn set_taskbar_state_cached(show: bool, cache: Option<&mut TaskbarCache>) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let taskbars = if let Some(c) = cache {
            c.get().to_vec()
        } else {
            find_all_explorer_taskbars()
        };
        let show_cmd = if show { SW_SHOW } else { SW_HIDE };

        for hwnd in taskbars {
            let _ = ShowWindow(hwnd, show_cmd);
        }

        Ok(())
    }
}

/// Show or hide all taskbars
fn set_taskbar_state(show: bool) -> Result<(), Box<dyn std::error::Error>> {
    set_taskbar_state_cached(show, None)
}

/// Read the current taskbar AppBar state
fn read_taskbar_appbar_state() -> u32 {
    unsafe {
        let mut appbar_data: APPBARDATA = mem::zeroed();
        appbar_data.cbSize = mem::size_of::<APPBARDATA>() as u32;
        SHAppBarMessage(ABM_GETSTATE, &mut appbar_data) as u32
    }
}

/// Write a new taskbar AppBar state
fn write_taskbar_appbar_state(state: u32) {
    unsafe {
        let mut appbar_data: APPBARDATA = mem::zeroed();
        appbar_data.cbSize = mem::size_of::<APPBARDATA>() as u32;
        appbar_data.lParam = LPARAM(state as isize);
        let _ = SHAppBarMessage(ABM_SETSTATE, &mut appbar_data);
    }
}

/// Manages taskbar AppBar state with automatic restoration on drop
struct TaskbarStateManager {
    original_state: u32,
    enforced_state: u32,
}

impl TaskbarStateManager {
    /// Create a new manager and enforce auto-hide state
    fn new() -> Self {
        let original_state = read_taskbar_appbar_state();
        let enforced_state = original_state | ABS_AUTOHIDE;

        if enforced_state != original_state {
            write_taskbar_appbar_state(enforced_state);
        }

        Self {
            original_state,
            enforced_state,
        }
    }

    /// Enforce the auto-hide state
    fn enforce(&self) {
        write_taskbar_appbar_state(self.enforced_state);
    }

    /// Restore the original taskbar state
    fn restore(&self) {
        write_taskbar_appbar_state(self.original_state);
    }
}

impl Drop for TaskbarStateManager {
    fn drop(&mut self) {
        self.restore();
    }
}

/// Get cached strings for single instance check
fn get_singleton_strings() -> (&'static [u16], &'static [u16], &'static [u16]) {
    use std::sync::OnceLock;
    static MUTEX_NAME: OnceLock<Vec<u16>> = OnceLock::new();
    static TITLE: OnceLock<Vec<u16>> = OnceLock::new();
    static MESSAGE: OnceLock<Vec<u16>> = OnceLock::new();

    (
        MUTEX_NAME.get_or_init(|| "Global\\TaskbarHideApp_SingleInstance\0".encode_utf16().collect()),
        TITLE.get_or_init(|| "Taskbar Hide\0".encode_utf16().collect()),
        MESSAGE.get_or_init(|| "Application is already running!\0".encode_utf16().collect()),
    )
}

/// Check if another instance is already running
fn check_single_instance() -> Option<HANDLE> {
    unsafe {
        let (mutex_name, title, message) = get_singleton_strings();

        let mutex_handle =
            CreateMutexW(None, true, windows::core::PCWSTR(mutex_name.as_ptr())).ok()?;

        if GetLastError() == ERROR_ALREADY_EXISTS {
            MessageBoxW(
                HWND(std::ptr::null_mut()),
                windows::core::PCWSTR(message.as_ptr()),
                windows::core::PCWSTR(title.as_ptr()),
                MB_OK | MB_ICONWARNING,
            );

            return None;
        }

        Some(mutex_handle)
    }
}

/// Get cached IPC window class name as UTF-16 (for main.rs usage)
fn get_ipc_window_class_utf16_main() -> &'static [u16] {
    use std::sync::OnceLock;
    static IPC_CLASS_UTF16: OnceLock<Vec<u16>> = OnceLock::new();
    IPC_CLASS_UTF16.get_or_init(|| format!("{}\0", cli::get_ipc_window_class()).encode_utf16().collect())
}

/// Create a hidden IPC window for CLI communication
fn create_ipc_window(event_loop_proxy: EventLoopProxy<IPCMessage>) {
    std::thread::spawn(move || unsafe {
        // Use cached UTF-16 string to avoid conversion overhead
        let class_name = get_ipc_window_class_utf16_main();

        let wc = WNDCLASSW {
            lpfnWndProc: Some(ipc_window_proc),
            lpszClassName: windows::core::PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };

        RegisterClassW(&wc);

        if let Ok(mut proxy) = GLOBAL_EVENT_PROXY.lock() {
            proxy.replace(event_loop_proxy);
        }

        let hwnd = CreateWindowExW(
            Default::default(),
            windows::core::PCWSTR(class_name.as_ptr()),
            windows::core::PCWSTR::null(),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            None,
            None,
        );

        if hwnd.is_err() {
            eprintln!("Failed to create IPC window");
            return;
        }

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}

/// Window procedure for IPC message handling
unsafe extern "system" fn ipc_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    let (msg_show, msg_hide, msg_quit) = cli::get_message_ids();

    let ipc_message = if msg == msg_show {
        Some(IPCMessage::Show)
    } else if msg == msg_hide {
        Some(IPCMessage::Hide)
    } else if msg == msg_quit {
        PostQuitMessage(0);
        Some(IPCMessage::Quit)
    } else {
        None
    };

    if let Some(ipc_msg) = ipc_message {
        if let Ok(guard) = GLOBAL_EVENT_PROXY.lock() {
            if let Some(proxy) = guard.as_ref() {
                let _ = proxy.send_event(ipc_msg);
            }
        }
        return windows::Win32::Foundation::LRESULT(0);
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // CLI mode
    if !args.is_empty() {
        let _ = attach_console();
        return cli::handle_cli_command(&args);
    }

    // GUI mode - ensure single instance
    let _mutex = check_single_instance().ok_or("Another instance is already running")?;

    let event_loop = EventLoopBuilder::<IPCMessage>::with_user_event().build()?;
    let event_loop_proxy = event_loop.create_proxy();

    // Build tray menu
    let tray_menu = Menu::new();
    let show_item = MenuItem::new("Show Taskbar", true, None);
    let hide_item = MenuItem::new("Hide Taskbar", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    tray_menu.append(&show_item)?;
    tray_menu.append(&hide_item)?;
    tray_menu.append(&quit_item)?;

    // Create tray icon
    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Taskbar Hide")
        .with_icon(load_icon())
        .build()?;

    // Initialize taskbar state manager and enforce auto-hide
    let taskbar_manager = Arc::new(TaskbarStateManager::new());
    taskbar_manager.enforce();
    set_taskbar_state(false)?;

    // Setup IPC for CLI communication
    create_ipc_window(event_loop_proxy);

    let menu_channel = MenuEvent::receiver();
    let should_hide = Arc::new(AtomicBool::new(true));
    let should_hide_clone = Arc::clone(&should_hide);
    let taskbar_manager_for_loop = Arc::clone(&taskbar_manager);

    // Monitor thread: continuously hide taskbar when it becomes visible
    // Uses caching to reduce expensive window lookups
    std::thread::spawn(move || {
        let mut cache = TaskbarCache::new();
        loop {
            if should_hide_clone.load(Ordering::SeqCst) && is_taskbar_visible_cached(&mut cache) {
                let _ = set_taskbar_state_cached(false, Some(&mut cache));
            }
            std::thread::sleep(std::time::Duration::from_millis(
                TASKBAR_MONITOR_INTERVAL_MS,
            ));
        }
    });

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        // Handle IPC messages from CLI
        if let winit::event::Event::UserEvent(ipc_msg) = event {
            match ipc_msg {
                IPCMessage::Show => {
                    should_hide.store(false, Ordering::SeqCst);
                    taskbar_manager_for_loop.restore();
                    let _ = set_taskbar_state(true);
                }
                IPCMessage::Hide => {
                    should_hide.store(true, Ordering::SeqCst);
                    taskbar_manager_for_loop.enforce();
                    let _ = set_taskbar_state(false);
                }
                IPCMessage::Quit => {
                    should_hide.store(false, Ordering::SeqCst);
                    taskbar_manager_for_loop.restore();
                    let _ = set_taskbar_state(true);
                    elwt.exit();
                }
            }
        }

        // Handle tray menu events
        if let Ok(menu_event) = menu_channel.try_recv() {
            let event_id = menu_event.id;

            if event_id == show_item.id() {
                should_hide.store(false, Ordering::SeqCst);
                taskbar_manager_for_loop.restore();
                let _ = set_taskbar_state(true);
            } else if event_id == hide_item.id() {
                should_hide.store(true, Ordering::SeqCst);
                taskbar_manager_for_loop.enforce();
                let _ = set_taskbar_state(false);
            } else if event_id == quit_item.id() {
                should_hide.store(false, Ordering::SeqCst);
                taskbar_manager_for_loop.restore();
                let _ = set_taskbar_state(true);
                elwt.exit();
            }
        }
    })?;

    Ok(())
}
