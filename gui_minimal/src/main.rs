fn to_wide(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn main() {
    use winapi::um::winuser::{MessageBoxW, MB_OK};
    use winapi::shared::windef::HWND;

    let text = to_wide("Parsec - Minimal GUI: running successfully");
    let title = to_wide("Parsec GUI (minimal)");

    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK);
    }
}
