//! The terminal, or its absence. On Windows the player is a GUI program, so
//! the Explorer opens no console window; started from a terminal, it writes
//! to that terminal. On macOS the application bundle opened from the Finder
//! has no terminal either. Without one, the window opens and a fatal error
//! goes to a dialog. Elsewhere the standard streams are used as they are.

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

/// Whether there is a terminal: the Finder starts the bundle without
/// arguments, its streams on /dev/null.
#[cfg(target_os = "macos")]
pub fn attach() -> bool {
    use std::io::IsTerminal;
    std::env::args_os().len() > 1 || std::io::stdin().is_terminal() || std::io::stderr().is_terminal()
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn attach() -> bool {
    true
}

/// Reports an error that stops the program, in a dialog without terminal.
pub fn fatal(msg: &str, terminal: bool) {
    eprintln!("error: {msg}");
    if !terminal {
        dialog(msg);
    }
}

#[cfg(windows)]
fn dialog(msg: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
    let wide = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
    // SAFETY: null-terminated UTF-16 strings that outlive the call.
    unsafe {
        MessageBoxW(std::ptr::null_mut(), wide(msg).as_ptr(), wide("Unsampled Piano").as_ptr(), MB_OK | MB_ICONERROR);
    }
}

#[cfg(target_os = "macos")]
fn dialog(msg: &str) {
    // The message as an argument, not inside the script: no quoting.
    let script = ["on run argv", "display alert \"Unsampled Piano\" message (item 1 of argv) as critical", "end run"];
    let mut cmd = std::process::Command::new("osascript");
    for line in script {
        cmd.args(["-e", line]);
    }
    let _ = cmd.arg(msg).status();
}

#[cfg(not(any(windows, target_os = "macos")))]
fn dialog(_msg: &str) {}
