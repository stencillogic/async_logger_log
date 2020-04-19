# async_logger_log

![Rust](https://github.com/stencillogic/async_logger_log/workflows/Rust/badge.svg)

Asynchronous logger is a performant implementation of [log](https://docs.rs/log) facade. The implementation is
based on [async_logger](https://docs.rs/async_logger) crate, and allows non-blocking writes of 
log records in memory buffer, which in turn then processed in separate thread by writer (see
more details in `async_logger` documentation).

Default log record format includes date, time, timezone, log level, target, and log message
itself. Log record example:

> [2020-03-15 11:47:32.339865887+0100 WARN thread]: log message.

The log record format, and other parameters are customizable with `LoggerBuilder`.

## Examples

``` rust
use async_logger_log::Logger;
use log::{info, warn};

let logger = Logger::new("/tmp", 256, 10*1024*1024).expect("Failed to create Logger instance");

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

struct StdoutWriter {}

// Writer simply prints log messages to stdout
impl Writer<Box<String>> for StdoutWriter {

    fn process_slice(&mut self, slice: &[Box<String>]) {
        for item in slice {
            println!("{}", **item);
        }
    }

    fn flush(&mut self) { }
}

let logger = Logger::builder()
    .buf_size(256)
    .formatter(custom_formatter)
    .writer(Box::new(StdoutWriter {}))
    .build()
    .unwrap();

log::set_boxed_logger(Box::new(logger)).expect("Failed to set logger");
log::set_max_level(log::LevelFilter::Trace);

debug!("{}", "Hello, Wrold!");

log::logger().flush();
```

# Notes

1. Dependency on `time` crate is optional and can be excluded by adding in Cargo.toml:

```
[dependencies.async_logger_log]
default-features = false
```

2. This crate was tested on Linux x86_64. Rust version 1.42.
