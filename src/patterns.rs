use moduleinfo::ModuleInfo;
use winapi::*;

pub type Pattern = [(u8, bool)];

fn compare(data: *const u8, pattern: &Pattern) -> bool {
	for i in 0..pattern.len() {
		let (b, m) = pattern[i];

		unsafe {
			if m && *data.offset(i as isize) != b {
				return false;
			}
		}
	}

	true
}

pub fn find(module: &ModuleInfo, pattern: &Pattern) -> Option<LPVOID> {
	if module.size < pattern.len() {
		return None;
	}

	let start = module.base as *const u8;
	let end = module.size - pattern.len();

	for i in 0..end {
		let ptr = unsafe { start.offset(i as isize) };

		if compare(ptr, pattern) {
			return Some(ptr as LPVOID);
		}
	}

	None
}

pub static Cbuf_AddText: [(u8, bool); 32] = [(0x8B, true), (0x54, true), (0x24, true), (0x04, true), (0x83, true), (0xC9, true), (0xFF, true), (0x57, true), (0x33, true), (0xC0, true), (0x8B, true), (0xFA, true), (0xF2, true), (0xAE, true), (0x8B, true), (0x3D, true), (0x00, false), (0x00, false), (0x00, false), (0x00, false), (0xA1, true), (0x00, false), (0x00, false), (0x00, false), (0x00, false), (0xF7, true), (0xD1, true), (0x49, true), (0x03, true), (0xCF, true), (0x3B, true), (0xC8, true)];
pub static Host_Spawn_f: [(u8, bool); 32] = [(0xA1, true), (0x00, false), (0x00, false), (0x00, false), (0x00, false), (0x53, true), (0xBB, true), (0x01, true), (0x00, true), (0x00, true), (0x00, true), (0x3B, true), (0xC3, true), (0x56, true), (0x75, true), (0x11, true), (0x68, true), (0x00, false), (0x00, false), (0x00, false), (0x00, false), (0xFF, true), (0x15, true), (0x00, false), (0x00, false), (0x00, false), (0x00, false), (0x83, true), (0xC4, true), (0x04, true), (0x5E, true), (0x5B, true)];
pub static Host_UnPause_f: [(u8, bool); 26] = [(0xA0, true), (0x00, false), (0x00, false), (0x00, false), (0x00, false), (0x84, true), (0xC0, true), (0x74, true), (0x59, true), (0x8B, true), (0x0D, true), (0x00, false), (0x00, false), (0x00, false), (0x00, false), (0xB8, true), (0x01, true), (0x00, true), (0x00, true), (0x00, true), (0x3B, true), (0xC8, true), (0x75, true), (0x0A, true), (0x50, true), (0xE8, true)];
pub static CHL1GameMovement__CheckJumpButton: [(u8, bool); 32] = [(0x83, true), (0xEC, true), (0x14, true), (0x53, true), (0x56, true), (0x8B, true), (0xF1, true), (0x57, true), (0x8B, true), (0x7E, true), (0x08, true), (0x85, true), (0xFF, true), (0x74, true), (0x12, true), (0x8B, true), (0x07, true), (0x8B, true), (0xCF, true), (0xFF, true), (0x90, true), (0x60, true), (0x01, true), (0x00, true), (0x00, true), (0x84, true), (0xC0, true), (0x74, true), (0x04, true), (0x8B, true), (0xCF, true), (0xEB, true)];
