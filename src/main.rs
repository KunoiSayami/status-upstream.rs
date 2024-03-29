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

use crate::configure::Configure;
use crate::database::get_current_timestamp;
use crate::datastructures::{EmptyUpstream, UpstreamTrait};
use crate::statuspagelib::StatusPageUpstream;
use crate::web_service::v1::make_router;
use anyhow::anyhow;
use clap::{arg, Command};
#[cfg(any(feature = "env_logger", feature = "log4rs"))]
use log::{info, warn};
#[cfg(feature = "spdlog-rs")]
use spdlog::{default_logger, init_log_crate_proxy, prelude::*, sink::FileSink};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{ConnectOptions, SqliteConnection};

mod configure;
mod database;
mod datastructures;
mod statuspagelib;
mod web_service;

const DEFAULT_DATABASE_LOCATION: &str = "database.db";

async fn check_database(
    config: &Configure,
    mut conn: SqliteConnection,
) -> anyhow::Result<SqliteConnection> {
    for component in config.components() {
        let ret = sqlx::query_as::<_, (i32,)>(r#"SELECT 1 FROM "machines" WHERE "uuid" = ?"#)
            .bind(component.uuid())
            .fetch_optional(&mut conn)
            .await
            .map_err(|e| {
                anyhow!(
                    "Get component error in check_database function {}: {:?}",
                    component.uuid(),
                    e
                )
            })?;
        if ret.is_none() {
            sqlx::query(r#"INSERT INTO "machines" VALUES (?, 'unknown', ?, ?, ?, ?)"#)
                .bind(component.uuid())
                .bind(get_current_timestamp() as u32)
                .bind(component.need_push())
                .bind(if component.page().is_empty() {
                    None
                } else {
                    Some(component.page().to_string())
                })
                .bind(if component.report_id().is_empty() {
                    None
                } else {
                    Some(component.report_id().to_string())
                })
                .execute(&mut conn)
                .await
                .map_err(|e| {
                    anyhow!(
                        "Insert component error in check_database function {}: {:?}",
                        component.uuid(),
                        e
                    )
                })?;
            info!("Insert {} into database", component.uuid())
        }
        // Current not check uuid not in database.
    }
    Ok(conn)
}

async fn async_main(config_file: &str) -> anyhow::Result<()> {
    let config = Configure::init_from_path(config_file)
        .await
        .map_err(|e| anyhow!("Read configure file failure: {:?}", e))?;

    let upstream: Box<dyn UpstreamTrait> = if config.statuspage().enabled() {
        Box::new(StatusPageUpstream::from_configure(&config)?.unwrap())
    } else {
        Box::new(EmptyUpstream::default())
    };

    let sqlite_connection = SqliteConnectOptions::new()
        .filename(config.server().database_location())
        .connect()
        .await
        .map_err(|e| {
            anyhow!(
                "Open database {} error: {:?}",
                config.server().database_location(),
                e
            )
        })?;

    let router = make_router(check_database(&config, sqlite_connection).await?, upstream);
    let bind = format!("{}:{}", config.server().addr(), config.server().port());
    let server_handler = axum_server::Handle::new();
    let server = tokio::spawn(
        axum_server::bind(bind.parse().unwrap())
            .handle(server_handler.clone())
            .serve(router.into_make_service()),
    );

    tokio::select! {
        _ = async {
            tokio::signal::ctrl_c().await.unwrap();
            info!("Recv Control-C send graceful shutdown command.");
            server_handler.graceful_shutdown(None);
            tokio::signal::ctrl_c().await.unwrap();
            warn!("Force to exit!");
            std::process::exit(137)
        } => {
        },
        _ = server => {
        }
    }
    Ok(())
}

#[cfg(feature = "spdlog-rs")]
fn init_spdlog_file(log_target: &str, is_debug: bool) {
    let file_sink = std::sync::Arc::new(FileSink::new(log_target, false).unwrap_or_else(|e| {
        eprintln!("Got error while create log file: {:?}", e);
        std::process::exit(1);
    }));
    // stdout & stderr
    let default_sinks = default_logger().sinks().to_owned();
    let logger = std::sync::Arc::new(
        Logger::builder()
            .sinks(default_sinks)
            .sink(file_sink)
            .build(),
    );
    let level_filter = if is_debug {
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
    if let Some(log_target) = matches.get_one::<String>("logfile") {
        #[cfg(feature = "spdlog-rs")]
        init_spdlog_file(log_target, matches.contains_id("debug"));
        init_log4rs(log_target, matches.contains_id("debug"))?;
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
            matches
                .get_one::<String>("config")
                .map(|s| s.as_str())
                .unwrap_or("config/default.toml"),
        ))?;
    Ok(())
}
