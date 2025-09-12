use core::fmt;
use log::Record;

/// Actually formats a [`log`] message properly and writes it to the
/// corresponding destination specified by `writer`.
///
/// This does not add a terminating newline.
pub fn fmt_and_write_msg(writer: &mut dyn fmt::Write, record: &Record) -> core::fmt::Result {
    write!(
        writer,
        "[{:>5} {}@{:03}]: {}",
        record.level(),
        record.file().unwrap_or("<unknown>"),
        record.line().unwrap_or(0),
        record.args()
    )
}
