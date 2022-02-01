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
use crate::configure::{Configure, TomlConfigure};
use crate::statuspagelib::ComponentStatus;
use clap::{arg, App};
use spdlog::{default_logger, prelude::*, sink::FileSink};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

mod configure;
#[allow(dead_code)]
mod connlib;
mod statuspagelib;

async fn main_work(rw_config: Arc<Mutex<Configure>>, retries: u64, retries_interval: u64) -> anyhow::Result<()> {
    let mut config = rw_config.lock().await;
    let upstream = config.upstream().clone();
    //let mut services: &Vec<ServiceWrapper>  = config.services().as_mut();
    for times in 0..retries {
        for service in config.mut_services() {
            if times > 0 && !service.ongoing_recheck() {
                continue
            }
            let ret = service.ping(5).await;
            if let Err(ref e) = ret {
                error!("Got error while ping {}: {:?}", service.remote_address(), e);
            }
            let result = ret.unwrap_or(false);
            if service.update_last_status_condition(result, retries) {
                upstream
                    .set_component_status(service.report_uuid(), ComponentStatus::from(result))
                    .await?;
                debug!("Update api to {}", result);
            }
        }
        tokio::time::sleep(Duration::from_secs(retries_interval)).await;
    }
    for service in config.mut_services() {
        service.reset_count()
    }
    Ok(())
}

async fn async_main(config_file: Option<&str>) -> anyhow::Result<()> {
    let config_file = config_file.unwrap_or("config/default.toml");
    let config = TomlConfigure::init_from_path(config_file).await?;
    let interval = config.config().interval().unwrap_or(0);
    let retries = config.config().retries_times().unwrap_or(3);
    let retries_interval = config.config().retries_interval().unwrap_or(5);
    let config = Configure::try_from(config)?;
    let config = Arc::new(Mutex::new(config));
    let main_future = if interval == 0 {
        tokio::spawn(main_work(config.clone(), retries, retries_interval))
    } else {
        tokio::spawn(async move {
            loop {
                main_work(config.clone(), retries,retries_interval).await?;
                tokio::time::sleep(Duration::from_secs(interval)).await;
            }
        })
    };

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        ret = main_future => {ret??;}
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .args(&[
            arg!(--config [FILE] "Specify configure file"),
            arg!(--logfile [LOGFILE] "Specify log file out instead of output to stdout"),
            arg!(-d --debug ... "turns debug logging"),
        ])
        .get_matches();

    if let Some(log_target) = matches.value_of("logfile") {
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
    } else {
        default_logger().set_level_filter(LevelFilter::Equal(Level::Debug));
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main(matches.value_of("config").clone()))?;
    Ok(())
}
