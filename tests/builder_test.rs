// testing builder with custom formatter and writer

extern crate async_logger;

use async_logger_log::*;
use async_logger::Writer;
use log::*;


const SAMPLE_LOG_MSG: &str = "Sample log msg";


fn custom_formatter(record: &Record) -> String {
    format!("\t{}\t", record.args())
}

struct WriterTest {}

impl Writer<Box<String>> for WriterTest {

    fn process_slice(&mut self, slice: &[Box<String>]) {

        for item in slice {
            assert_eq!(
                format!("\t{}\t", SAMPLE_LOG_MSG),
                **item
            );
        }
    }

    fn flush(&mut self) { }
}


#[test]
fn test_custom_fmt() {
    let logger = Logger::builder()
        .buf_size(100)
        .formatter(custom_formatter)
        .writer(Box::new(WriterTest {}))
        .build()
        .unwrap();

    log::set_boxed_logger(Box::new(logger)).expect("Failed to set logger");
    log::set_max_level(log::LevelFilter::Trace);

    debug!("{}", SAMPLE_LOG_MSG);

    log::logger().flush();
}
