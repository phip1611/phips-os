use loader_lib::KernelFile;
use log::{LevelFilter, Log, Metadata, Record};
use std::io::stdout;
use std::{fmt, fs, io};
use util::logging::fmt_and_write_msg;

struct IoToFmt<W: io::Write>(W);

impl<W: io::Write> fmt::Write for IoToFmt<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_all(s.as_bytes()).map_err(|e| {
            eprintln!("Failed to write to stdout: {e}");
            fmt::Error
        })?;
        self.0.flush().map_err(|e| {
            eprintln!("Failed to flush stdout: {e}");
            fmt::Error
        })
    }
}

struct Logger;

impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let stdout = stdout();
        let mut stdout = IoToFmt(stdout);
        fmt_and_write_msg(&mut stdout, record).unwrap();
    }

    fn flush(&self) {}
}

/// Performs the kernel ELF checks on the file that was provided as first
/// argument.
fn main() {
    log::set_logger(&Logger).unwrap();
    log::set_max_level(LevelFilter::Trace);

    let elf_path = std::env::args().nth(1).unwrap();
    let elf_bytes = fs::read(elf_path).unwrap();

    // This either returns success or panics.
    let kernel = KernelFile::from_bytes(&elf_bytes).unwrap();

    for (pr_hdr, data) in kernel.segments() {
        println!(
            "SEGMENT: type={}, flags={:#x} payload_len={}",
            pr_hdr.p_type,
            pr_hdr.p_flags,
            data.len()
        );
    }
}
