// general functional test

extern crate async_logger_log;

#[cfg(test)]
extern crate regex;

use async_logger_log::*;
use log::*;
use std::path::Path;
use std::io::{BufRead, BufReader};
use std::fs::File;
use std::thread;
use std::sync::{Once, Mutex, MutexGuard};
use std::mem::MaybeUninit;
use regex::Regex;


const LOG_DIR: &str = "/tmp/AsyncLoggerNBTest_49873205451691";

const LOG_FILE_SIZE: usize = 10*1024*1024;

static mut TEST_MUTEX: MaybeUninit<Mutex<()>> = MaybeUninit::uninit();

static INIT_MUTEX: Once = Once::new();


fn prepare<'a>() -> MutexGuard<'a, ()> {

    INIT_MUTEX.call_once(|| {
        unsafe { TEST_MUTEX = MaybeUninit::new(Mutex::new(())) };
    });

    let mtx: &Mutex<()> = unsafe { TEST_MUTEX.as_ptr().as_ref().expect("Test mutex is not initialized") };
    let guard = mtx.lock().expect("Test mutex is poisoned");

    if Path::new(LOG_DIR).exists() {

        cleanup();
    }

    std::fs::create_dir(LOG_DIR).expect("Failed to create test dir");

    guard
}

fn cleanup() {
    std::fs::remove_dir_all(LOG_DIR).expect("Failed to delete test dir on cleanup");
}

fn get_resulting_file_path() -> String {

    String::from(Path::new(LOG_DIR)
        .read_dir()
        .expect("Failed to list files in test directory")
        .next()
        .expect("No files found in test directory")
        .expect("Failed to get entry inside test directory")
        .path()
        .to_str()
        .expect("Failed to get file path as str"))
}


#[test]
fn test_logger() {
    let write_line = "testing log record";
    let _guard = prepare();
    let cnt = 1000u32;
    let buf_sz = 1024;

    let mut matches = vec![];

    for s in &["main", "thread"] {
        for l in &["DEBUG", "ERROR", "INFO", "TRACE", "WARN"] {
            let re_str;
            #[cfg(feature="time")]
            {
                re_str = format!("\\[\\d{{4}}-\\d{{2}}-\\d{{2}} \\d{{2}}:\\d{{2}}:\\d{{2}}.\\d{{9}}[+-]\\d{{4}} {} {}\\]: {}\n",
                             l, s, write_line);
            }
            #[cfg(not(feature="time"))]
            {
                re_str = format!("\\[\\d+ {} {}\\]: {}\n",
                             l, s, write_line);
            }

            let re = Regex::new(&re_str).unwrap();

            matches.push((s, l, re, 0u32));
        }
    }


    let logger = Logger::new(LOG_DIR, buf_sz, LOG_FILE_SIZE).expect("Failed to create Logger instance");
    log::set_boxed_logger(Box::new(logger)).expect("Failed to set logger");
    log::set_max_level(log::LevelFilter::Trace);

    let handle = thread::spawn(move || {
        for _ in 0..cnt {
            debug!(target: "thread", "{}", write_line);
            error!(target: "thread", "{}", write_line);
            info!(target: "thread", "{}", write_line);
            trace!(target: "thread", "{}", write_line);
            warn!(target: "thread", "{}", write_line);
            log!(target: "thread", Level::Error, "{}", write_line);
        }

        log::logger().flush();
    });

    for _ in 0..cnt {
        debug!(target: "main", "{}", write_line);
        error!(target: "main", "{}", write_line);
        info!(target: "main", "{}", write_line);
        trace!(target: "main", "{}", write_line);
        warn!(target: "main", "{}", write_line);
        log!(target: "main", Level::Error, "{}", write_line);
    }

    log::logger().flush();

    handle.join().expect("Failed on thread join");


    let out_file = get_resulting_file_path();

    let mut reader = BufReader::new(File::open(out_file).expect("Failed to open resulting file"));

    let mut line = String::new();

    loop {

        let len = reader.read_line(&mut line).expect("Failed to read line from the reslting file");

        if len == 0 {

            break;
        }

        for (_, _, re, n) in matches.iter_mut() {
            if re.is_match(&line) {
                *n += 1;
            }
        }

        line.clear();
    }

    for (thread, level, _, n) in matches.iter() {
        assert_eq!(
            cnt*(if **level == "ERROR" {2} else {1}), 
            *n, 
            "Line number mismatch for level {}, thread {}", level, thread
        );
    }

    cleanup();
}
