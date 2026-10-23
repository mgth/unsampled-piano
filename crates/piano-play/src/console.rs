//! The terminal on Windows. The player is a GUI program there, so the
//! Explorer opens no console window; started from a terminal, it writes to
//! that terminal, and without one a fatal error goes to a message box.
//! Elsewhere the standard streams are used as they are.

/// Whether there is a terminal (or redirected output) to write to.
#[cfg(windows)]
pub fn attach() -> bool {
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE};
    // SAFETY: plain Win32 calls without pointers.
    unsafe {
        let err = GetStdHandle(STD_ERROR_HANDLE);
        // Redirected to a file or a pipe: already usable.
        if !err.is_null() && err != INVALID_HANDLE_VALUE {
            return true;
        }
        AttachConsole(ATTACH_PARENT_PROCESS) != 0
    }
}

#[cfg(not(windows))]
pub fn attach() -> bool {
    true
}

/// Reports an error that stops the program.
pub fn fatal(msg: &str) {
    eprintln!("error: {msg}");
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::{GetConsoleWindow, GetStdHandle, STD_ERROR_HANDLE};
        use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
        let wide = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
        // SAFETY: null-terminated UTF-16 strings that outlive the call.
        unsafe {
            if GetConsoleWindow().is_null() && GetStdHandle(STD_ERROR_HANDLE).is_null() {
                MessageBoxW(std::ptr::null_mut(), wide(msg).as_ptr(), wide("Unsampled Piano").as_ptr(), MB_OK | MB_ICONERROR);
            }
        }
    }
}
