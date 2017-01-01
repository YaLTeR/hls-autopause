pub type Pattern = [(u8, bool)];

pub fn compare(data: *const u8, pattern: &Pattern) -> bool {
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
