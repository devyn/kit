/*******************************************************************************
 *
 * kit/kernel/log.rs
 *
 * vim:ft=rust:ts=4:sw=4:et:tw=80
 *
 * Copyright (C) 2015-2021, Devyn Cairns
 * Redistribution of this file is permitted under the terms of the simplified
 * BSD license. See LICENSE for more information.
 *
 ******************************************************************************/

use core::fmt;
use core::str::FromStr;

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::sync::Arc;

use log::{Metadata, Record, LevelFilter};

use crate::cmdline::Cmdline;
use crate::sync::Rcu;

static mut LOG: Option<Log> = None;

/// Initialize the logging system.
///
/// Command line options:
///
/// * `earlylog=com1`: Initially output to serial port 1.
/// * `earlylog=console`: Initially output to [terminal::console].
/// * `loglevel=...`: Options for log filtering. See [Log::set_log_levels]
pub fn initialize(cmdline: &Cmdline) {
    unsafe {
        assert!(LOG.is_none());
    }

    let log = Log::new();

    // Configure from command line.
    for (key, value) in cmdline.iter() {
        match key {
            "earlylog" => {
                log.add_logger(match value {
                    "com1" => Arc::new(destinations::Serial::Com1),
                    "console" => Arc::new(destinations::Console),
                    _ => continue
                });
            },
            "loglevel" => {
                let _ = log.set_log_levels(value);
            },
            _ => ()
        }
    }

    unsafe {
        LOG = Some(log);
        log::set_logger(LOG.as_ref().unwrap()).unwrap();
        log::set_max_level(LevelFilter::Trace);
    }
}

pub fn global() -> &'static Log {
    unsafe { LOG.as_ref().unwrap() }
}

pub struct Log {
    config: Rcu<Config>,
}

impl log::Log for Log {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.config.read().enabled(metadata)
    }

    fn log(&self, record: &Record) {
        let config = self.config.read();

        if config.enabled(record.metadata()) {
            for logger in config.loggers.iter() {
                logger.log(record);
            }
        }
    }

    fn flush(&self) {
        for logger in self.config.read().loggers.iter() {
            logger.flush();
        }
    }
}

impl Log {
    pub fn new() -> Log {
        Log { config: Config::default().into() }
    }

    /// Set the default log level, if no other filters match.
    pub fn set_default_level(&self, level: LevelFilter) {
        self.config.update_with(|config| Some(Config {
            default_level: level,
            ..(**config).clone()
        }.into()));
    }

    /// Accepts log level filters in a comma-separated list, of which the
    /// elements can be either a plain log level (to use as the default level),
    /// or a pair in the form `target_prefix=level`.
    ///
    /// For example:
    ///
    /// * `info,kernel::memory=debug` - use info level for most things, but log
    ///   at the Debug level for [kernel::memory]
    /// * `kernel::process=trace` - turn off logging for everything except
    ///   [kernel::process], at the Trace level
    pub fn set_log_levels(&self, filter_string: &str) -> Result<(), ()> {
        let mut default_level = LevelFilter::Off;

        let mut levels: Vec<(Box<str>, LevelFilter)> = vec![];

        for pref in filter_string.split(",") {
            if let Some((target, level)) = pref.split_once("=") {
                let level = LevelFilter::from_str(level).map_err(|_| ())?;
                levels.push((target.into(), level));
            } else {
                let level = LevelFilter::from_str(pref).map_err(|_| ())?;
                default_level = level;
            }
        }

        levels.sort_by_key(|(prefix, _)| core::cmp::Reverse(prefix.len()));

        let levels: Arc<[_]> = levels.into();

        self.config.update_with(|config| Some(Config {
            default_level,
            log_levels: levels.clone(),
            loggers: config.loggers.clone(),
        }.into()));

        Ok(())
    }

    /// Add a logger to the list of loggers.
    ///
    /// The logger will be forwarded any messages that are not excluded by this
    /// logger's filter.
    pub fn add_logger(&self, logger: Arc<dyn log::Log>) {
        self.config.update_with(|config| {
            let mut loggers = Vec::with_capacity(config.loggers.len() + 1);

            loggers.extend(config.loggers.iter().cloned());
            loggers.push(logger.clone());

            Some(Config {
                loggers: loggers.into(),
                ..(**config).clone()
            }.into())
        });
    }
}

#[derive(Clone)]
struct Config {
    default_level: LevelFilter,
    log_levels: Arc<[(Box<str>, LevelFilter)]>,
    loggers: Arc<[Arc<dyn log::Log>]>,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Config")
            .field("default_level", &self.default_level)
            .field("log_levels", &self.log_levels)
            .field("loggers", &format_args!("Arc([... {} element(s) ...])",
                self.loggers.len()))
            .finish()
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            default_level: LevelFilter::Trace,
            log_levels: Arc::new([]),
            loggers: Arc::new([]),
        }
    }
}

impl Config {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let target = metadata.target();
        let max_level = self.log_levels.iter()
            .find(|(prefix, _)| {
                target.starts_with(&prefix[..])
            })
            .map(|(_, filter)| *filter)
            .unwrap_or(self.default_level);

        metadata.level() <= max_level
    }
}

pub mod destinations {
    use core::fmt::Write;
    use log::{Metadata, Record, Level};

    pub enum Serial {
        Com1,
    }

    impl log::Log for Serial {
        fn enabled(&self, _: &Metadata) -> bool { true }
        fn flush(&self) { }

        fn log(&self, record: &Record) {
            let _ = self.log_internal(record);
        }
    }

    impl Serial {
        fn log_internal(&self, record: &Record) -> core::fmt::Result {
            let mut stream = match *self {
                Serial::Com1 => crate::serial::com1(),
            };

            write!(stream, "{}: ", record.level())?;

            if let Some(filename) = record.file() {
                write!(stream, "{}:{}: ", filename, record.line().unwrap_or(0))
            } else {
                write!(stream, "{}: ", record.target())
            }?;

            writeln!(stream, "{}", record.args())
        }
    }

    pub struct Console;

    impl log::Log for Console {
        fn enabled(&self, _: &Metadata) -> bool { true }
        fn flush(&self) { }

        fn log(&self, record: &Record) {
            let _ = self.log_internal(record);
        }
    }

    impl Console {
        fn log_internal(&self, record: &Record) -> core::fmt::Result {
            use crate::terminal::*;

            let console = console();

            let (fg, bg) = console.get_color();

            console.set_color(match record.level() {
                Level::Error => Color::LightRed,
                Level::Warn => Color::LightBrown,
                Level::Info => Color::LightGreen,
                Level::Debug => Color::LightCyan,
                Level::Trace => Color::LightGrey,
            }, Color::Black)?;

            write!(console, "{}: ", record.level())?;

            console.set_color(Color::LightGrey, Color::Black)?;

            if let Some(filename) = record.file() {
                write!(console, "{}:{}: ", filename, record.line().unwrap_or(0))
            } else {
                write!(console, "{}: ", record.target())
            }?;

            console.set_color(Color::White, Color::Black)?;

            writeln!(console, "{}", record.args())?;

            console.set_color(fg, bg)?;

            Ok(())
        }
    }
}
