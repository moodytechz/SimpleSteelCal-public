/// Show an error to the user even when there is no console window.
///
/// On Windows with `#![windows_subsystem = "windows"]` stderr is not visible,
/// so we write to a log file next to the executable AND show a native message
/// box via the Win32 `MessageBoxW` API. On other platforms we fall back to
/// stderr + log file.
pub fn show_startup_error(message: &str) {
    if let Ok(exe) = std::env::current_exe() {
        let log_path = exe.with_extension("error.log");
        let _ = std::fs::write(&log_path, message);
    }

    #[cfg(windows)]
    {
        show_message_box_win32("SteelCal - Startup Error", message);
    }

    #[cfg(not(windows))]
    {
        eprintln!("SteelCal startup error: {message}");
    }
}

#[cfg(windows)]
fn show_message_box_win32(title: &str, message: &str) {
    use std::ffi::OsStr;
    use std::iter;
    use std::os::windows::ffi::OsStrExt;

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(iter::once(0)).collect()
    }

    let title_w = to_wide(title);
    let msg_w = to_wide(message);

    const MB_OK: u32 = 0x0000_0000;
    const MB_ICONERROR: u32 = 0x0000_0010;

    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            msg_w.as_ptr(),
            title_w.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(windows)]
unsafe extern "system" {
    fn MessageBoxW(hwnd: *mut u8, text: *const u16, caption: *const u16, utype: u32) -> i32;
}
