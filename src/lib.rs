//! Asynchronous logger is a performant implementation of [log](https://docs.rs/log) facade. The implementation is
//! based on [async_logger](https://docs.rs/async_logger) crate, and allows non-blocking writes of 
//! references of log records in memory buffer. The messages in turn then processed in separate thread 
//! by writer (see more details in `async_logger` documentation).
//!
//! Default log record format includes date, time, timezone, log level, target, and log message
//! itself. Log record example:
//!
//! > [2020-03-15 11:47:32.339865887+0100 WARN thread]: log message.
//!
//! The log record format, and other parameters are customizable with `LoggerBuilder`.
//!
//! # Examples
//!
//! ```
//! use async_logger_log::Logger;
//! use log::{info, warn};
//!
//! let logger = Logger::new("/tmp", 256, 10*1024*1024).expect("Failed to create Logger instance");
//!
//! log::set_boxed_logger(Box::new(logger)).expect("Failed to set logger");
//! log::set_max_level(log::LevelFilter::Info);
//!
//! info!("{}", "test msg");
//! warn!("{}", "warning msg");
//!
//! log::logger().flush();
//! ```
//!
//! Custom writer and formatter:
//!
//! ```
//! use async_logger_log::Logger;
//! use async_logger::Writer;
//! use log::{debug, Record};
//!
//! // Custom formatting of `log::Record`
//! fn custom_formatter(record: &Record) -> String {
//!     format!("log record: {}\n", record.args())
//! }
//! 
//! struct StdoutWriter {}
//! 
//! // Writer simply prints log messages to stdout
//! impl Writer<Box<String>> for StdoutWriter {
//! 
//!     fn process_slice(&mut self, slice: &[Box<String>]) {
//!         for item in slice {
//!             println!("{}", **item);
//!         }
//!     }
//! 
//!     fn flush(&mut self) { }
//! }
//! 
//! let logger = Logger::builder()
//!     .buf_size(256)
//!     .formatter(custom_formatter)
//!     .writer(Box::new(StdoutWriter {}))
//!     .build()
//!     .unwrap();
//! 
//! log::set_boxed_logger(Box::new(logger)).expect("Failed to set logger");
//! log::set_max_level(log::LevelFilter::Trace);
//! 
//! debug!("{}", "Hello, Wrold!");
//! 
//! log::logger().flush();
//! ```
//!
//! ### Notes
//!
//! The formatting is done by the caller of logging macros. This operation produces `String` insance 
//! containing complete log message. Then the reference of that `String` instance is passed to
//! the underling non-blocking queue. The log message is then fetched, processed, and dropped by the writer thread. 
//! So, the cost for the client mostly consists of allocation and building of log message string. 
//!
//! The logger doesn't drop the log messages if the queue is full. In that case the operation
//! blocks until there is free slot in one of the queue buffers.
//!
//! Dependency on `time` crate is optional and can be excluded by adding in Cargo.toml:
//!
//! ``` toml
//! [dependencies.async_logger_log]
//! default-features = false
//! ```



extern crate async_logger;
extern crate log;

#[cfg(feature="time")]
extern crate time;


use log::{Log, Metadata, Record};
use async_logger::{AsyncLoggerNB, FileWriter, Error};
use std::sync::Arc;

#[cfg(feature="time")]
use time::OffsetDateTime;


const DEFAULT_BUF_SIZE: usize = 256;
const DEFAULT_LOG_FILE_SIZE: usize = 10*1024*1024;


/// Log trait implementation.
pub struct Logger {
    async_logger: Arc<AsyncLoggerNB<Box<String>>>,
    formatter: fn(&Record) -> String,
}

impl Logger {

    /// Creates a new logger instance and registers itself in the log facade.
    ///
    /// `buf_sz`: number of messages that internal buffer can hold.
    ///
    /// `file_size`: the size in bytes after which log file rotation occurs.
    pub fn new(log_dir: &str, buf_sz: usize, file_size: usize) -> Result<Logger, Error> {

        let writer = FileWriter::new(log_dir, file_size)?;

        let async_logger = Arc::new(AsyncLoggerNB::new(Box::new(writer), buf_sz)?);

        let formatter = Logger::format_msg;

        Ok(Logger {
            async_logger,
            formatter,
        })
    }

    /// Return `LoggerBuilder`
    pub fn builder() -> LoggerBuilder {
        LoggerBuilder {
            buf_sz: None,
            writer: None,
            formatter: None,
        }
    }

    fn format_msg(record: &Record) -> String {

        let time;
        #[cfg(feature="time")] 
        {
            time = OffsetDateTime::now_local().format("%Y-%m-%d %H:%M:%S.%N%z");
        }
        #[cfg(not(feature="time"))] 
        {
            time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or(std::time::Duration::new(0,0))
                .as_secs();
        }

        format!("[{} {} {}]: {}\n", time, record.level(), record.target(), record.args())
    }
}

impl Log for Logger {

    fn enabled(&self, metadata: &Metadata) -> bool {

        return metadata.level() <= log::max_level();
    }

    fn log(&self, record: &Record) {

        let msg = (self.formatter)(record);

        let _ = self.async_logger.write_value(Box::new(msg));
    }

    fn flush(&self) {

        AsyncLoggerNB::flush(&self.async_logger);
    }
}

/// Builder of `Logger` instance. It can be used to provide custom record formatter, and writer
/// implementation.
pub struct LoggerBuilder {
    buf_sz: Option<usize>,
    writer: Option<Box<dyn async_logger::Writer<Box<String>>>>,
    formatter: Option<fn(&Record) -> String>,
}

impl LoggerBuilder {

    /// Set the size of the pair of underlying buffers. The default is 256 messages each.
    pub fn buf_size(mut self, size: usize) -> Self {
        self.buf_sz = Some(size);
        self
    }

    /// Set custom formatter of log records.
    pub fn formatter(mut self, formatter: fn(&Record) -> String) -> Self {
        self.formatter = Some(formatter);
        self
    }

    /// Set custom writer implementation. The default is `FileWriter`.
    pub fn writer(mut self, writer: Box<dyn async_logger::Writer<Box<String>>>) -> Self {
        self.writer = Some(writer);
        self
    }

    /// Build the `Logger` instance.
    pub fn build(self) -> Result<Logger,Error> {

        let buf_sz = match self.buf_sz {
            Some(buf_sz) => buf_sz,
            None => DEFAULT_BUF_SIZE,
        };
        
        let writer = match self.writer {
            Some(writer) => writer,
            None => Box::new(FileWriter::new(".", DEFAULT_LOG_FILE_SIZE)?),
        };

        let formatter = match self.formatter {
            Some(formatter) => formatter,
            None => Logger::format_msg,
        };

        let async_logger = Arc::new(AsyncLoggerNB::new(writer, buf_sz)?);

        Ok(Logger {
            async_logger,
            formatter,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use async_logger::ErrorKind;
    use std::path::Path;


    const LOG_DIR: &str = "/tmp/AsyncLoggerNBTest_000239400377";
    const NONEXISTING_LOG_DIR: &str = "/tmp/AsyncLoggerNBTest_85003857407";
    const LOG_FILE_SIZE: usize = 4096;

    #[test]
    fn test_error() {

        // via new

        if Path::new(LOG_DIR).exists() {
            std::fs::remove_dir_all(LOG_DIR).expect("Failed to delete test dir on cleanup");
        }

        std::fs::create_dir(LOG_DIR).expect("Failed to create test dir");

        match Logger::new(LOG_DIR, 0, LOG_FILE_SIZE) {
            Err(e) if e.kind() == ErrorKind::IncorrectBufferSize => {},
            _ => panic!("Expected error, got Ok!"),
        }

        std::fs::remove_dir_all(LOG_DIR).expect("Failed to delete test dir on cleanup");
        std::fs::create_dir(LOG_DIR).expect("Failed to create test dir");

        match Logger::new(LOG_DIR, std::usize::MAX, LOG_FILE_SIZE) {
            Err(e) if e.kind() == ErrorKind::IncorrectBufferSize => {},
            _ => panic!("Expected error, got Ok!"),
        }

        std::fs::remove_dir_all(LOG_DIR).expect("Failed to delete test dir on cleanup");

        match Logger::new(NONEXISTING_LOG_DIR, 100, LOG_FILE_SIZE) {
            Err(e) if e.kind() == ErrorKind::IoError => {
            },
            _ => panic!("Expected error, got Ok!"),
        }

        // via builder 

        std::fs::create_dir(LOG_DIR).expect("Failed to create test dir");

        let writer = FileWriter::new(LOG_DIR, LOG_FILE_SIZE).expect("Failed to create file writer");

        match Logger::builder()
            .buf_size(0)
            .writer(Box::new(writer))
            .build() 
        {
            Err(e) if e.kind() == ErrorKind::IncorrectBufferSize => {},
            _ => panic!("Expected error, got Ok!"),
        }

        std::fs::remove_dir_all(LOG_DIR).expect("Failed to delete test dir on cleanup");
        std::fs::create_dir(LOG_DIR).expect("Failed to create test dir");

        let writer = FileWriter::new(LOG_DIR, LOG_FILE_SIZE).expect("Failed to create file writer");

        match Logger::builder()
            .buf_size(std::usize::MAX)
            .writer(Box::new(writer))
            .build() 
        {
            Err(e) if e.kind() == ErrorKind::IncorrectBufferSize => {},
            _ => panic!("Expected error, got Ok!"),
        }

        std::fs::remove_dir_all(LOG_DIR).expect("Failed to delete test dir on cleanup");
    }
}
