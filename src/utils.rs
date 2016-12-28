use std::ptr;
use user32;
use utils;
use winapi::*;

pub fn utf16(string: &str) -> Vec<u16> {
	string.encode_utf16().chain(Some(0)).collect()
}

pub fn msgbox(message: &str) {
	unsafe {
		user32::MessageBoxW(ptr::null_mut(), utils::utf16(&message).as_ptr(), utils::utf16("HL:S OOE Autopause").as_ptr(), MB_ICONERROR);
	}
}
