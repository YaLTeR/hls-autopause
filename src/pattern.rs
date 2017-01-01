#[derive(Clone, Copy)]
pub struct Pattern(pub &'static [(u8, bool)]);

impl Pattern {
	pub fn len(self) -> usize {
		self.0.len()
	}

	pub fn compare(self, data: *const u8) -> bool {
		for i in 0..self.0.len() {
			let (b, m) = self.0[i];

			unsafe {
				if m && *data.offset(i as isize) != b {
					return false;
				}
			}
		}

		true
	}
}
