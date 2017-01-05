use log::*;
use std::env;

mod window;

const DEFAULT_LOG_LEVEL: LogLevelFilter = LogLevelFilter::Debug;

lazy_static! {
    static ref LOG_LEVEL_FILTER: LogLevelFilter =
        env::var("Y_LOGLEVEL").map(|s| string_to_log_level(&s)).unwrap_or(DEFAULT_LOG_LEVEL);
}

fn string_to_log_level(string: &str) -> LogLevelFilter {
    match string {
        x if x == "TRACE" => LogLevelFilter::Trace,
        x if x == "DEBUG" => LogLevelFilter::Debug,
        x if x == "INFO" => LogLevelFilter::Info,
        x if x == "WARN" => LogLevelFilter::Warn,
        x if x == "ERROR" => LogLevelFilter::Error,
        _ => DEFAULT_LOG_LEVEL,
    }
}

struct Logger {
    _h: (),
}

impl Logger {
    fn new() -> Self {
        window::init();

        Logger { _h: () }
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= *LOG_LEVEL_FILTER
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            println!("[{}] [{}] {}", record.level(), record.target(), record.args());
            window::log(record);
        }
    }
}

pub fn init() -> Result<(), SetLoggerError> {
    set_logger(|max_log_level| {
        max_log_level.set(*LOG_LEVEL_FILTER);
        Box::new(Logger::new())
    })
}
