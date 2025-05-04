use std::os::raw::c_int;

extern "C" {
    fn _newt_sys__colorset_custom(i: c_int) -> c_int;
}

#[inline]
pub fn NEWT_COLORSET_CUSTOM(i: i32) -> i32 {
    unsafe { _newt_sys__colorset_custom(i) }
}
