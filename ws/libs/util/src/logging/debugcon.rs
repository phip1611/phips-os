use crate::drivers::DebugCon;
use crate::logging::fmt_and_write_msg;
use core::fmt::Write;
use log::{Metadata, Record};

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
