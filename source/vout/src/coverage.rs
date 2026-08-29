#[cfg(unix)]
pub fn flush() {
    use std::ffi::{c_char, c_void};

    type WriteFile = unsafe extern "C" fn() -> i32;

    unsafe extern "C" {
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }

    let symbol = c"__llvm_profile_write_file";
    let func = unsafe { dlsym(std::ptr::null_mut(), symbol.as_ptr()) };

    if !func.is_null() {
        let write_file: WriteFile = unsafe { std::mem::transmute(func) };
        let _ = unsafe { write_file() };
    }
}

#[cfg(not(unix))]
pub fn flush() {}
