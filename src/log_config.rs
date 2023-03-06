use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Config, Logger, Root};

pub fn configure_log(level:LevelFilter) -> log4rs::Handle {
    let pattern = Box::new(PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S:%f)(local)} | {h({l})} | {f}:{L} | {m}{n}"));
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let stdout = ConsoleAppender::builder().encoder(pattern.clone()).build();
    let file_log = FileAppender::builder()
        .encoder(pattern)
        .build(format!("logs/{}-binance.log", date))
        .unwrap();
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("file_log", Box::new(file_log)))
        .logger(Logger::builder()
            .appender("stdout")
            .appender("file_log")
            .additive(true)
            .build("logs", level))
        .build(Root::builder().appender("stdout").appender("file_log").build(level))
        .unwrap();
    log4rs::init_config(config).unwrap()
}