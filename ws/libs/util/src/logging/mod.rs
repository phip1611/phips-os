mod debugcon;

pub use debugcon::*;

use alloc::boxed::Box;
use core::cell::OnceCell;
use core::fmt;
use log::{LevelFilter, Log, Metadata, Record};
use spin::Mutex as SpinMutex;

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

/// Logging facade for [`log`] over various optional logger backends.
///
/// This is for applications without runtime, i.e., OS loader, kernel, etc.
/// To start logging, [`LoggerFacade::init`] must be called once.
pub struct LoggerFacade(SpinMutex<OnceCell<LoggerFacadeInner>>);

impl LoggerFacade {
    /// Creates a new default object.
    pub const fn new() -> LoggerFacade {
        Self(SpinMutex::new(OnceCell::new()))
    }

    /// Inits the logger.
    ///
    /// This operation must only be called once.
    pub fn init<'a: 'static>(&'a self, inner: LoggerFacadeInner, max_level: LevelFilter) {
        self.0.lock().get_or_init(|| inner);
        log::set_logger(self).expect("should init logger only once");
        log::set_max_level(max_level);
    }

    /// Updates the object from the given closure.
    pub fn update(&self, update_fn: impl Fn(&mut LoggerFacadeInner)) {
        let mut guard = self.0.lock();
        let inner = guard.get_mut().expect("should have initialized logger");
        update_fn(inner);
    }
}

impl Default for LoggerFacade {
    fn default() -> Self {
        Self::new()
    }
}

impl Log for LoggerFacade {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.0
            .lock()
            .get()
            .map(|f| f.enabled(metadata))
            .unwrap_or(false)
    }

    fn log(&self, record: &Record) {
        let _ = self.0.lock().get().map(|f| f.log(record));
    }

    fn flush(&self) {
        let _ = self.0.lock().get().map(|f| f.flush());
    }
}

pub struct LoggerFacadeInner {
    debugcon: Option<DebugconLogger>,
    stdout_logger: Option<Box<dyn Log>>,
}

impl LoggerFacadeInner {
    pub fn new() -> Self {
        Self {
            debugcon: None,
            stdout_logger: None,
        }
    }

    pub fn set_debugcon(&mut self, debugcon: DebugconLogger) {
        self.debugcon = Some(debugcon);
    }

    pub fn set_stdout_logger(&mut self, stdout_logger: Box<dyn Log>) {
        self.stdout_logger = Some(stdout_logger);
    }

    fn loggers(&self) -> [Option<&dyn Log>; 2] {
        [
            self.stdout_logger.as_deref(),
            self.debugcon.as_ref().map(|d| d as &dyn Log),
        ]
    }
}

impl Default for LoggerFacadeInner {
    fn default() -> Self {
        Self::new()
    }
}

impl Log for LoggerFacadeInner {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        for logger in self.loggers().into_iter().flatten() {
            if logger.enabled(record.metadata()) {
                logger.log(record);
            }
        }
    }

    fn flush(&self) {
        for logger in self.loggers().into_iter().flatten() {
            logger.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::logging::test_support::StdErrLogger;
    use crate::logging::{LoggerFacade, LoggerFacadeInner};
    use alloc::boxed::Box;
    use log::LevelFilter;

    static TEST_LOGGER: LoggerFacade = LoggerFacade::new();

    #[test]
    fn set_facade_as_logger() {
        let mut logger_facade = LoggerFacadeInner::new();
        logger_facade.set_stdout_logger(Box::new(StdErrLogger));
        TEST_LOGGER.init(logger_facade, LevelFilter::Trace);
        log::info!("hello from logger");
    }
}

#[cfg(test)]
pub mod test_support {
    use super::*;

    pub struct StdErrLogger;

    impl fmt::Write for StdErrLogger {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            std::eprint!("{s}");
            Ok(())
        }
    }

    impl Log for StdErrLogger {
        fn enabled(&self, _metadata: &Metadata) -> bool {
            true
        }

        fn log(&self, record: &Record) {
            fmt_and_write_msg(&mut StdErrLogger, record)
                .expect("should not failed to format and write log message")
        }

        fn flush(&self) {}
    }
}
