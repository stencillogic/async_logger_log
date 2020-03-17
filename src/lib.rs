//! Asyncronous logger implementation of [log](https://docs.rs/log) facade. The implementation is
//! based on [async_logger](https://docs.rs/async_logger) crate, and allows non-blocking writes of 
//! log records in memory buffer, which in turn then processed in separate thread by writer (see
//! more details in `async_logger` documentation).
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
//! let logger = Logger::new("/tmp", 65536).expect("Failed to create Logger instance");
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
//! struct WriterTest {}
//! 
//! // Writer simply prints log messages to stdout
//! impl Writer for WriterTest {
//! 
//!     fn process_slice(&mut self, slice: &[u8]) {
//!         println!("Got log message: {}", String::from_utf8_lossy(slice));
//!     }
//! 
//!     fn flush(&mut self) { }
//! }
//! 
//! let logger = Logger::builder()
//!     .buf_size(100)
//!     .formatter(custom_formatter)
//!     .writer(Box::new(WriterTest {}))
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


extern crate async_logger;
extern crate log;
extern crate time;


use log::{Log, Metadata, Record};
use async_logger::{AsyncLoggerNB, FileWriter, Error};
use std::sync::Arc;
use time::OffsetDateTime;


const DEFAULT_BUF_SIZE: usize = 65536;


/// Log trait implementation.
pub struct Logger {
    async_logger: Arc<AsyncLoggerNB>,
    formatter: fn(&Record) -> String,
}

impl Logger {
    
    /// Create a new logger instance and register in log facade.
    pub fn new(log_dir: &str, buf_sz: usize) -> Result<Logger, Error> {

        let writer = FileWriter::new(log_dir)?;

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

        let time = OffsetDateTime::now_local().format("%Y-%m-%d %H:%M:%S.%N%z");

        format!("[{} {} {}]: {}\n", time, record.level(), record.target(), record.args())
    }
}

impl Log for Logger {

    fn enabled(&self, metadata: &Metadata) -> bool {

        return metadata.level() <= log::max_level();
    }

    fn log(&self, record: &Record) {

        let msg = (self.formatter)(record);

        let _ = self.async_logger.write_slice(msg.as_bytes());
    }

    fn flush(&self) {

        AsyncLoggerNB::flush(&self.async_logger);
    }
}


/// Builder of `Logger` instance. It can be used to provide custom record formatter, and writer
/// implementation.
pub struct LoggerBuilder {
    buf_sz: Option<usize>,
    writer: Option<Box<dyn async_logger::Writer>>,
    formatter: Option<fn(&Record) -> String>,
}

impl LoggerBuilder {

    /// Set the size of the pair of underlying buffers. The default is 65536 bytes each.
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
    pub fn writer(mut self, writer: Box<dyn async_logger::Writer>) -> Self {
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
            None => Box::new(FileWriter::new(".")?),
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


    #[test]
    fn test_error() {

        // via new

        if Path::new(LOG_DIR).exists() {
            std::fs::remove_dir_all(LOG_DIR).expect("Failed to delete test dir on cleanup");
        }

        std::fs::create_dir(LOG_DIR).expect("Failed to create test dir");

        match Logger::new(LOG_DIR, 0) {
            Err(e) if e.kind() == ErrorKind::IncorrectBufferSize => {},
            _ => panic!("Expected error, got Ok!"),
        }

        std::fs::remove_dir_all(LOG_DIR).expect("Failed to delete test dir on cleanup");
        std::fs::create_dir(LOG_DIR).expect("Failed to create test dir");

        match Logger::new(LOG_DIR, std::usize::MAX) {
            Err(e) if e.kind() == ErrorKind::IncorrectBufferSize => {},
            _ => panic!("Expected error, got Ok!"),
        }

        std::fs::remove_dir_all(LOG_DIR).expect("Failed to delete test dir on cleanup");

        match Logger::new(NONEXISTING_LOG_DIR, 100) {
            Err(e) if e.kind() == ErrorKind::IoError => {
            },
            _ => panic!("Expected error, got Ok!"),
        }

        // via builder 

        std::fs::create_dir(LOG_DIR).expect("Failed to create test dir");

        let writer = FileWriter::new(LOG_DIR).expect("Failed to create file writer");

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

        let writer = FileWriter::new(LOG_DIR).expect("Failed to create file writer");

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
