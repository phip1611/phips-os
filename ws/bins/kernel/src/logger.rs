use {
    log::LevelFilter,
    util::logging::{
        DebugconLogger,
        LoggerFacade,
        LoggerFacadeInner,
    },
};

static LOGGER: LoggerFacade = LoggerFacade::new();

/// Inits early loggers that doesn't need allocation.
pub fn early_init() {
    let mut logger = LoggerFacadeInner::new();
    logger.set_debugcon(DebugconLogger);
    LOGGER.init(logger, LevelFilter::Trace);
    log::debug!("initialized early loggers");
}

/*/// Inits additional loggers and logging that need memory allocations.
pub fn init() {
    todo!();
    //LOGGER.update(|logger| logger.set_stdout_logger(Box::new(StdOutLogger)));
    log::debug!("initialized additional loggers");
}*/
