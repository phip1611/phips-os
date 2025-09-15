use crate::UEFI_BOOT_SERVICES_EXITED;
use alloc::boxed::Box;
use core::fmt::Write;
use core::sync::atomic::Ordering;
use log::{LevelFilter, Log, Metadata, Record};
use util::logging::{DebugconLogger, LoggerFacade, LoggerFacadeInner, fmt_and_write_msg};

static LOGGER: LoggerFacade = LoggerFacade::new();

/// Inits early loggers that doesn't need allocation.
pub fn early_init() {
    let mut logger = LoggerFacadeInner::new();
    logger.set_debugcon(DebugconLogger);
    LOGGER.init(logger, LevelFilter::Trace);
    log::debug!("initialized early loggers");
}

/// Inits additional loggers and logging that need memory allocations.
pub fn init() {
    LOGGER.update(|logger| logger.set_stdout_logger(Box::new(StdOutLogger)));
    log::debug!("initialized additional loggers");
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
