use log::{LevelFilter, Log, Metadata, Record};
use std::fmt::Write;
use util::logging::fmt_and_write_msg;

/// Inits the logger.
pub fn init() {
    log::set_max_level(LevelFilter::Trace);
    log::set_logger(&StdErrLogger).expect("should only be initialized once");
}

struct StdErrLogger;

impl Log for StdErrLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        uefi::system::with_stdout(|out| {
            fmt_and_write_msg(out, record)
                .expect("should not failed to format and write log message");
            out.write_char('\r').unwrap();
            out.write_char('\n').unwrap();
        })
    }

    fn flush(&self) {}
}
