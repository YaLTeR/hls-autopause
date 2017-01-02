use log::*;

struct Logger;

impl Log for Logger {
	fn enabled(&self, _metadata: &LogMetadata) -> bool {
		true
	}

	fn log(&self, record: &LogRecord) {
		if self.enabled(record.metadata()) {
			println!("[{}] [{}] {}", record.level(), record.target(), record.args());
		}
	}
}

pub fn init() -> Result<(), SetLoggerError> {
	set_logger(|max_log_level| {
		max_log_level.set(LogLevelFilter::Trace);
		Box::new(Logger)
	})
}
