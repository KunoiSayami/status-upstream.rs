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
use crate::connlib::ServiceChecker;
use crate::statuspagelib::ComponentStatus;
use clap::{arg, App};
use log4rs::append::file::FileAppender;
use log4rs::config::Appender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::Config;
use std::time::Duration;

mod configure;
#[allow(dead_code)]
mod connlib;
mod statuspagelib;

async fn main_work(config: Configure) -> anyhow::Result<()> {
    for service in config.services() {
        let ret = service.inner().ping(5).await;
        if let Err(ref e) = ret {
            // TODO: show address
            log::error!("Got error while ping {}: {:?}", service.report_uuid(), e);
        }
        config
            .upstream()
            .set_component_status(
                service.report_uuid(),
                if ret.unwrap_or(false) {
                    ComponentStatus::MajorOutage
                } else {
                    ComponentStatus::Operational
                },
            )
            .await?;
    }
    Ok(())
}

async fn async_main(config_file: Option<&str>) -> anyhow::Result<()> {
    let config_file = config_file.unwrap_or("config/default.toml");
    let config = TomlConfigure::init_from_path(config_file).await?;
    let interval = config.config().interval().unwrap_or(0);
    let config = Configure::try_from(config)?;
    let main_future = if interval == 0 {
        tokio::spawn(main_work(config.clone()))
    } else {
        tokio::spawn(async move {
            loop {
                main_work(config.clone()).await?;
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
    let log_target = matches.value_of("logfile");
    if log_target.is_some() {
        let log_file_requests = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(
                "{d(%Y-%m-%d %H:%M:%S)}- {h({l})} - {m}{n}",
            )))
            .build(log_target.unwrap());
        if let Err(ref e) = log_file_requests {
            eprintln!("Got error while create log file: {:?}", e);
        }
        let log_config = Config::builder()
            .appender(Appender::builder().build("logfile", Box::new(log_file_requests?)))
            .build(log4rs::config::Root::builder().appender("logfile").build(
                if matches.is_present("debug") {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Info
                },
            ))
            .unwrap();
        log4rs::init_config(log_config)?;
    } else {
        env_logger::init();
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main(matches.value_of("config").clone()))?;
    Ok(())
}
