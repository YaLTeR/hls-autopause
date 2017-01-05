use kernel32;
use log::*;
use std::{mem, ptr, thread};
use std::sync::{Arc, Condvar, Mutex};
use user32;
use utils;
use winapi::*;

const CFM_COLOR: DWORD = 0x40000000;
const CFM_FACE: DWORD = 0x20000000;
const EM_EXSETSEL: UINT = WM_USER + 55;
const EM_SETCHARFORMAT: UINT = WM_USER + 68;
const EM_REPLACESEL: UINT = 0xC2;
const ES_MULTILINE: DWORD = 0x4;
const ES_AUTOVSCROLL: DWORD = 0x40;
const ES_READONLY: DWORD = 0x800;
const LF_FACESIZE: usize = 32;
const SCF_DEFAULT: DWORD = 0x0;
const SCF_SELECTION: DWORD = 0x1;

static mut HWND_EDIT: HWND = 0 as HWND;

lazy_static! {
	static ref CLASS_NAME: Vec<u16> = utils::utf16("Debug Console");
	static ref WINDOW_TITLE: Vec<u16> = utils::utf16("Debug Console");
	static ref FONT_NAME: Vec<u16> = {
		let mut v = utils::utf16("Consolas");
		v.resize(LF_FACESIZE, 0);
		v
	};
	static ref CV: Arc<(Mutex<bool>, Condvar)> = Arc::new((Mutex::new(false), Condvar::new()));
}

#[repr(C)]
struct CHARRANGE {
	cpMin: LONG,
	cpMax: LONG,
}

#[repr(C)]
struct CHARFORMAT {
	cbSize: UINT,
	dwMask: DWORD,
	dwEffects: DWORD,
	yHeight: LONG,
	yOffset: LONG,
	crTextColor: COLORREF,
	bCharSet: BYTE,
	bPitchAndFamily: BYTE,
	szFaceName: [WCHAR; LF_FACESIZE],
}

unsafe extern "system" fn WndProc(hwnd: HWND, message: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
	match message {
		x if x == WM_SIZE => {
			let w = LOWORD(lparam as u32);
			let h = HIWORD(lparam as u32);
			user32::SetWindowPos(HWND_EDIT, ptr::null_mut(), 0, 0, w as i32, h as i32, 0);
			0
		},

		x if x == WM_DESTROY => {
			user32::PostQuitMessage(0);
			0
		},

		_ => user32::DefWindowProcW(hwnd, message, wparam, lparam)
	}
}

fn load_richedit() -> Result<(), DWORD> {
	unsafe {
		if kernel32::LoadLibraryW(utils::utf16("Msftedit.dll").as_ptr()) != ptr::null_mut() {
			Ok(())
		} else {
			Err(kernel32::GetLastError())
		}
	}
}

fn register_class(instance: HINSTANCE) -> Result<(), DWORD> {
	let wc = WNDCLASSW {
		style: 0,
		lpfnWndProc: Some(WndProc),
		cbClsExtra: 0,
		cbWndExtra: 0,
		hInstance: instance,
		hIcon: ptr::null_mut(),
		hCursor: ptr::null_mut(),
		hbrBackground: (COLOR_WINDOW + 1) as HBRUSH,
		lpszMenuName: ptr::null(),
		lpszClassName: CLASS_NAME.as_ptr(),
	};

	unsafe {
		if user32::RegisterClassW(&wc) != 0 {
			Ok(())
		} else {
			Err(kernel32::GetLastError())
		}
	}
}

fn create_window(instance: HINSTANCE) -> Result<HWND, DWORD> {
	unsafe {
		let hwnd = user32::CreateWindowExW(0,
		                                   CLASS_NAME.as_ptr(),
		                                   WINDOW_TITLE.as_ptr(),
		                                   WS_OVERLAPPEDWINDOW,
		                                   CW_USEDEFAULT, 0, CW_USEDEFAULT, 0,
		                                   ptr::null_mut(),
		                                   ptr::null_mut(),
		                                   instance,
		                                   ptr::null_mut());

		if hwnd != ptr::null_mut() {
			Ok(hwnd)
		} else {
			Err(kernel32::GetLastError())
		}
	}
}

fn create_richedit(instance: HINSTANCE, window: HWND) -> Result<HWND, DWORD> {
	unsafe {
		let hwnd_edit = user32::CreateWindowExW(0,
												utils::utf16("RICHEDIT50W").as_ptr(),
												ptr::null(),
												ES_MULTILINE | ES_AUTOVSCROLL | ES_READONLY | WS_VISIBLE | WS_CHILD | WS_VSCROLL | WS_HSCROLL,
												0, 0, 0, 0,
												window,
												ptr::null_mut(),
												instance,
												ptr::null_mut());

		if hwnd_edit != ptr::null_mut() {
			Ok(hwnd_edit)
		} else {
			Err(kernel32::GetLastError())
		}
	}
}

fn set_font(edit: HWND) {
	unsafe {
		let mut cf = mem::zeroed::<CHARFORMAT>();
		cf.cbSize = mem::size_of::<CHARFORMAT>() as UINT;
		cf.dwMask = CFM_FACE;
		ptr::copy(FONT_NAME.as_ptr(), cf.szFaceName.as_mut_ptr(), LF_FACESIZE);

		user32::SendMessageW(edit, EM_SETCHARFORMAT, SCF_DEFAULT, &cf as *const _ as LPARAM);
	}
}

fn initialize_window() -> Result<(), String> {
	try!(load_richedit().map_err(|e| format!("Error loading richedit: {}", e)));

	let instance = unsafe { kernel32::GetModuleHandleW(ptr::null()) };
	try!(register_class(instance).map_err(|e| format!("Error registering class: {}", e)));
	let window = try!(create_window(instance).map_err(|e| format!("Error creating window: {}", e)));
	let edit = try!(create_richedit(instance, window).map_err(|e| format!("Error creating richedit: {}", e)));
	set_font(edit);

	unsafe {
		HWND_EDIT = edit;

		user32::ShowWindow(window, SW_SHOWNORMAL);
		user32::ShowWindow(window, SW_SHOWMINNOACTIVE);
		user32::UpdateWindow(window);
	}

	Ok(())
}

fn message_thread() {
	let result = initialize_window();

	// Notify the main thread that we're ready.
	{
		let &(ref lock, ref cvar) = &**CV;
		let mut started = lock.lock().unwrap();
		*started = true;
		cvar.notify_one();
	}

	if let Err(err) = result {
		utils::msgbox(&err);
		return;
	}

	// Message loop.
	unsafe {
		let mut msg = mem::uninitialized();

		loop {
			match user32::GetMessageW(&mut msg, ptr::null_mut(), 0, 0) {
				-1 => {
					utils::msgbox(&format!("Error in the message loop: {}", kernel32::GetLastError()));
					break;
				},

				0 => break, // The window was closed.

				_ => {
					user32::TranslateMessage(&msg);
					user32::DispatchMessageW(&msg);
				}
			}
		}

		// The window was closed, clear the edit HWND.
		HWND_EDIT = ptr::null_mut();
	}
}

pub fn log(record: &LogRecord) {
	let edit = unsafe { HWND_EDIT };
	if edit == ptr::null_mut() {
		return;
	}

	// Set the selection to past-the-end.
	{
		let cr = CHARRANGE {
			cpMin: -1,
			cpMax: -1,
		};

		unsafe {
			user32::SendMessageW(HWND_EDIT, EM_EXSETSEL, 0, &cr as *const _ as LPARAM);
		}
	}

	// Set the appropriate text color.
	{
		let cf = CHARFORMAT {
			cbSize: mem::size_of::<CHARFORMAT>() as UINT,
			dwMask: CFM_COLOR,
			dwEffects: 0,
			yHeight: 0,
			yOffset: 0,
			crTextColor: match record.level() {
				LogLevel::Error => RGB(255, 0, 0),
				LogLevel::Warn => RGB(200, 0, 0),
				LogLevel::Info => unsafe { user32::GetSysColor(COLOR_WINDOWTEXT) },
				LogLevel::Debug => RGB(100, 100, 0),
				LogLevel::Trace => unsafe { user32::GetSysColor(COLOR_GRAYTEXT) },
			},
			bCharSet: 0,
			bPitchAndFamily: 0,
			szFaceName: [0; LF_FACESIZE]
		};

		unsafe {
			user32::SendMessageW(HWND_EDIT, EM_SETCHARFORMAT, SCF_SELECTION, &cf as *const _ as LPARAM);
		}
	}

	// Put in the text.
	{
		let text = format!("[{}] {}\n", record.target(), record.args());

		unsafe {
			user32::SendMessageW(HWND_EDIT, EM_REPLACESEL, 0, utils::utf16(&text).as_ptr() as LPARAM);
		}
	}

	// Scroll the rich edit to the bottom.
	unsafe {
		user32::SendMessageW(HWND_EDIT, WM_VSCROLL, SB_BOTTOM as WPARAM, 0);
	}
}

pub fn init() {
	thread::spawn(message_thread);

	// Wait till the window is created.
	let &(ref lock, ref cvar) = &**CV;
	let mut started = lock.lock().unwrap();
	while !*started {
		started = cvar.wait(started).unwrap();
	}
}
