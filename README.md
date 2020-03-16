# async_logger_log

![Rust](https://github.com/stencillogic/async_logger_log/workflows/Rust/badge.svg)

Asyncronous logger implementation of [log](https://docs.rs/log) facade. The implementation is
based on [async_logger](https://docs.rs/async_logger) crate, and allows non-blocking writes of 
log records in memory buffer, which in turn then processed in separate thread by writer (see
more details in `async_logger` documentation).

Default log record format includes date, time, timezone, log level, target, and log message
itself. Log record example:

> [2020-03-15 11:47:32.339865887+0100 WARN thread]: log message.

The log record format, and other parameters are customizable with `LoggerBuilder`.

# Examples

``` rust
use async_logger_log::Logger;
use log::{info, warn};

let logger = Logger::new("/tmp", 65536).expect("Failed to create Logger instance");

log::set_boxed_logger(Box::new(logger)).expect("Failed to set logger");
log::set_max_level(log::LevelFilter::Info);

info!("{}", "test msg");
warn!("{}", "warning msg");

log::logger().flush();
```

Custom writer and formatter:

``` rust
use async_logger_log::Logger;
use async_logger::Writer;
use log::{debug, Record};

// Custom formatting of `log::Record`
fn custom_formatter(record: &Record) -> String {
    format!("log record: {}\n", record.args())
}

struct WriterTest {}

// Writer simply prints log messages to stdout
impl Writer for WriterTest {

    fn process_slice(&mut self, slice: &[u8]) {
        println!("Got log message: {}", String::from_utf8_lossy(slice));
    }

    fn flush(&mut self) { }
}

let logger = Logger::builder()
    .buf_size(100)
    .formatter(custom_formatter)
    .writer(Box::new(WriterTest {}))
    .build()
    .unwrap();

log::set_boxed_logger(Box::new(logger)).expect("Failed to set logger");
log::set_max_level(log::LevelFilter::Trace);

debug!("{}", "Hello, Wrold!");

log::logger().flush();
```
