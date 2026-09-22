use core::ffi::{VaList, c_char, c_int};

pub type VprintfFn = unsafe extern "C" fn(*const c_char, VaList<'_>) -> c_int;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vprintf(_fmt: *const c_char, _args: VaList<'_>) -> c_int {
    todo!("Quest 2: implement format parsing")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printf(fmt: *const c_char, args: ...) -> c_int {
    unsafe { vprintf(fmt, args) }
}
