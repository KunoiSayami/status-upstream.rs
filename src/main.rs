/*
 ** Copyright (C) 2021-2022 KunoiSayami
 **
 ** This program is free software: you can redistribute it and/or modify
 ** it under the terms of the GNU Affero General Public License as published by
 ** the Free Software Foundation, either version 3 of the License, or
 ** any later version.
 **
 ** This program is distributed in the hope that it will be useful,
 ** but WITHOUT ANY WARRANTY; without even the implied warranty of
 ** MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 ** GNU Affero General Public License for more details.
 **
 ** You should have received a copy of the GNU Affero General Public License
 ** along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

#[cfg(all(feature = "spdlog-rs", any(feature = "env_logger", feature = "log4rs")))]
compile_error!("You should choose only one log feature");

use crate::cache::{read_cache, CacheData};
use crate::configure::{Configure, TomlConfigure};
use crate::connlib::ServerLastStatus;
use crate::statuspagelib::ComponentStatus;
use anyhow::anyhow;
use clap::{arg, Command};
#[cfg(any(feature = "env_logger", feature = "log4rs"))]
use log::{debug, info};
#[cfg(feature = "spdlog-rs")]
use spdlog::{default_logger, init_log_crate_proxy, prelude::*, sink::FileSink};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

mod cache;
mod configure;
#[allow(dead_code)]
mod connlib;
mod statuspagelib;

const DEFAULT_CACHE_FILE: &str = "config/cache.json";
const DEFAULT_CACHE_WRITE_INTERVAL: u64 = 300;
const DEFAULT_PING_TIMEOUT: u64 = 5;

async fn main_work(
    rw_config: Arc<Mutex<Configure>>,
    retries: u64,
    retries_interval: u64,
) -> anyhow::Result<()> {
    let mut config = rw_config.lock().await;
    let upstream = config.upstream().clone();
    //let mut services: &Vec<ServiceWrapper>  = config.services().as_mut();
    for times in 0..retries {
        for service in config.mut_services() {
            if times > 0 && !service.ongoing_recheck() {
                continue;
            }
            let result = service.ping(DEFAULT_PING_TIMEOUT).await;
            let result = ServerLastStatus::from(result);
            debug!("Pinging {} {}", service.remote_address(), result);
            if service.update_last_status_condition(result, retries - 1) {
                upstream
                    .set_component_status(
                        service.report_uuid(),
                        service.page(),
                        ComponentStatus::from(service.last_status()),
                    )
                    .await?;
                info!("Update {} status to {}", service.remote_address(), result);
            }
        }
        tokio::time::sleep(Duration::from_secs(retries_interval)).await;
    }
    for service in config.mut_services() {
        service.reset_count()
    }
    Ok(())
}

async fn save_cache_file(config: Arc<Mutex<Configure>>, cache_file: &str) -> anyhow::Result<()> {
    let content = {
        let config = config.lock().await;
        CacheData::from_configure(&config)
    };
    let content = serde_json::to_string(&content);
    let content = if let Err(e) = content {
        return Err(anyhow!("Got error while create cache content: {:?}", e));
    } else {
        content.unwrap()
    };
    Ok(tokio::fs::write(cache_file, content.as_bytes()).await?)
}

async fn async_main(config_file: Option<&str>, cache_file: Option<&str>) -> anyhow::Result<()> {
    let config_file = config_file.unwrap_or("config/default.toml");
    let cache_file = cache_file.unwrap_or(DEFAULT_CACHE_FILE).to_string();
    let config = TomlConfigure::init_from_path(config_file).await?;

    if config.is_empty_services() {
        info!("Services list is empty, exit!");
        return Ok(());
    }

    let interval = config.config().interval().unwrap_or(0);

    let retries = config.config().retries_times().unwrap_or(3);
    let retries_interval = config.config().retries_interval().unwrap_or(5);

    let cache = read_cache(&cache_file).await;

    let config = Configure::try_from(
        config,
        if cache.is_ok() {
            Some(cache.unwrap())
        } else {
            None
        },
    )
    .await?;

    #[cfg(feature = "ping")]
    config
        .check_icmp_ping_available()
        .await
        .map_err(|e| anyhow!("Got error while check ICMP ping availability: {:?}", e))?;

    let config = Arc::new(Mutex::new(config));
    let alt_config = config.clone();
    let main_future = if interval == 0 {
        tokio::spawn(main_work(config.clone(), retries, retries_interval))
    } else {
        tokio::spawn(async move {
            loop {
                main_work(config.clone(), retries, retries_interval).await?;
                tokio::time::sleep(Duration::from_secs(interval)).await;
            }
        })
    };

    let save_cache_task: tokio::task::JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(DEFAULT_CACHE_WRITE_INTERVAL));
        let cache_file = cache_file.to_string();
        loop {
            interval.tick().await;
            save_cache_file(alt_config.clone(), &cache_file).await?;
        }
    });

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        ret = main_future => {ret??;}
        ret = save_cache_task => {ret??;}
    }
    Ok(())
}

#[cfg(feature = "spdlog-rs")]
fn init_spdlog_file(log_target: &str) {
    let file_sink = Arc::new(FileSink::new(log_target, false).unwrap_or_else(|e| {
        eprintln!("Got error while create log file: {:?}", e);
        std::process::exit(1);
    }));
    // stdout & stderr
    let default_sinks = spdlog::default_logger().sinks().to_owned();
    let logger = Arc::new(
        Logger::builder()
            .sinks(default_sinks)
            .sink(file_sink)
            .build(),
    );
    let level_filter = if matches.is_present("debug") {
        LevelFilter::MoreSevereEqual(Level::Debug)
    } else {
        LevelFilter::MoreSevereEqual(Level::Info)
    };
    logger.set_level_filter(level_filter);

    spdlog::set_default_logger(logger);
}

#[cfg(feature = "log4rs")]
fn init_log4rs(log_target: &str, debug: bool) -> anyhow::Result<()> {
    let log_file_requests = log4rs::append::file::FileAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)}- {h({l})} - {m}{n}",
        )))
        .build(log_target);
    if let Err(ref e) = log_file_requests {
        eprintln!("Got error while create log file: {:?}", e);
    }
    let log_config = log4rs::Config::builder()
        .appender(
            log4rs::config::Appender::builder().build("logfile", Box::new(log_file_requests?)),
        )
        .build(
            log4rs::config::Root::builder()
                .appender("logfile")
                .build(if debug {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Info
                }),
        )
        .unwrap();
    log4rs::init_config(log_config)?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .args(&[
            arg!(--config [FILE] "Specify configure file"),
            arg!(--logfile [LOGFILE] "Specify log file out instead of output to stdout"),
            arg!(-d --debug ... "turns debug logging"),
            arg!(--cache [CACHEFILE] "Specify cache file location"),
        ])
        .get_matches();

    #[cfg(feature = "spdlog-rs")]
    init_log_crate_proxy().expect("Init log crate got error");
    if let Some(log_target) = matches.value_of("logfile") {
        #[cfg(feature = "spdlog-rs")]
        init_spdlog_file(log_target);
        init_log4rs(log_target, matches.is_present("debug"))?;
    } else {
        #[cfg(feature = "spdlog-rs")]
        default_logger().set_level_filter(LevelFilter::MoreSevereEqual(Level::Debug));
        #[cfg(feature = "env_logger")]
        env_logger::Builder::from_default_env()
            .filter_module("rustls", log::LevelFilter::Warn)
            .init();
        info!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    }

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main(
            matches.value_of("config").clone(),
            matches.value_of("cache").clone(),
        ))?;
    Ok(())
}
