use crate::UEFI_BOOT_SERVICES_EXITED;
use log::{LevelFilter, Log, Metadata, Record};
use std::fmt::Write;
use std::sync::atomic::Ordering;
use util::logging::{DebugconLogger, LoggerFacade, LoggerFacadeInner, fmt_and_write_msg};

static LOGGER: LoggerFacade = LoggerFacade::new();

/// Inits the logger.
pub fn init() {
    let mut logger = LoggerFacadeInner::new();
    logger.set_debugcon(DebugconLogger);
    logger.set_stdout_logger(Box::new(StdOutLogger));
    LOGGER.init(logger, LevelFilter::Trace);
}

/// Removes any logging functionality using UEFI boot services.
pub fn exit_boot_services() {}

struct StdOutLogger;

impl Log for StdOutLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if UEFI_BOOT_SERVICES_EXITED.load(Ordering::SeqCst) {
            return;
        }

        uefi::system::with_stdout(|out| {
            fmt_and_write_msg(out, record)
                .expect("should not failed to format and write log message");
            out.write_char('\r').unwrap();
            out.write_char('\n').unwrap();
        })
    }

    fn flush(&self) {}
}
