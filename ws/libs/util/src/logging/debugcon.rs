use {
    crate::{
        drivers::DebugCon,
        logging::fmt_and_write_msg,
    },
    core::fmt::Write,
    log::{
        Metadata,
        Record,
    },
};

pub struct DebugconLogger;

impl log::Log for DebugconLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        fmt_and_write_msg(&mut DebugCon, record).unwrap();
        DebugCon.write_char('\n').unwrap();
    }

    fn flush(&self) {}
}
