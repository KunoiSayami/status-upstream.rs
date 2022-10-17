pub mod v1 {
    use super::TransferData;
    use crate::database::get_current_timestamp;
    use axum::extract::Path;
    use axum::http::StatusCode;
    use axum::response::{IntoResponse, Response};
    use axum::{Json, Router};
    #[cfg(any(feature = "env_logger", feature = "log4rs"))]
    use log::error;
    use serde_json::json;
    #[cfg(feature = "spdlog-rs")]
    use spdlog::prelude::*;
    use sqlx::SqliteConnection;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tower::ServiceBuilder;
    use tower_http::trace::TraceLayer;

    pub const VERSION: &str = "1";

    pub fn make_router(conn: SqliteConnection) -> Router {
        let conn = Arc::new(Mutex::new(conn));
        Router::new()
            .route(
                "/v1/components/:component_id",
                axum::routing::get({
                    let conn = conn.clone();
                    |path| async move { get(Path(path), conn).await }
                })
                .post({
                    let conn = conn.clone();
                    |path, payload| async move { post(path, payload, conn).await }
                }),
            )
            .route(
                "/",
                axum::routing::get(|| async { Json(json!({ "version": VERSION, "status": 200 })) }),
            )
            .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
    }

    pub async fn post(
        Path(uuid): Path<String>,
        Json(payload): Json<TransferData>,
        sql_conn: Arc<Mutex<SqliteConnection>>,
    ) -> impl IntoResponse {
        let mut sql_conn = sql_conn.lock().await;

        let query = sqlx::query(
            r#"UPDATE "machines" SET "status" = ?, "last_update" = ? WHERE "uuid" = ?"#,
        )
        .bind(payload.status())
        .bind(get_current_timestamp() as u32)
        .bind(&uuid)
        .execute(&mut *sql_conn)
        .await
        .map_err(|e| {
            error!(
                "Update database for {} to {} error: {:?}",
                &uuid,
                payload.status(),
                e
            )
        });
        if query.is_ok() {
            if sqlx::query_as::<_, (bool,)>(
                r#"SELECT "need_upload" FROM "matchines" WHERE "uuid" = ?"#,
            )
            .bind(&uuid)
            .fetch_optional(&mut *sql_conn)
            .await
            .map_err(|e| error!("Fetch {} need_upload field error: {:?}", &uuid, e))
            {}
            (StatusCode::OK, json!({"status": 200}).to_string())
        } else {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"status": 500}).to_string(),
            )
        }
        .into_response()
    }

    pub async fn get(Path(uuid): Path<String>, sql_conn: Arc<Mutex<SqliteConnection>>) -> Response {
        let mut sql_conn = sql_conn.lock().await;
        let query_result =
            sqlx::query_as::<_, (String,)>(r#"SELECT "status" FROM "machines" WHERE "uuid" = ? "#)
                .bind(&uuid)
                .fetch_optional(&mut *sql_conn)
                .await
                .map_err(|e| {
                    error!(
                        "Got error while fetching component {} status: {:?}",
                        &uuid, e
                    )
                });
        if let Ok(query_result) = query_result {
            match query_result {
                None => (
                    StatusCode::NOT_FOUND,
                    serde_json::to_string(&TransferData::not_found()).unwrap(),
                ),
                Some((result,)) => (
                    StatusCode::OK,
                    serde_json::to_string(&TransferData::new(result)).unwrap(),
                ),
            }
        } else {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"status": 500}).to_string(),
            )
        }
        .into_response()
    }
}

pub mod datastructure_v1 {
    use serde_derive::{Deserialize, Serialize};
    use std::fmt::Formatter;

    #[derive(Clone, Debug, Default, Deserialize, Serialize)]
    pub struct TransferData {
        status: String,
    }

    impl TransferData {
        pub fn new(status: String) -> Self {
            Self { status }
        }

        pub fn not_found() -> Self {
            Self {
                status: "NOT_FOUND".to_string(),
            }
        }
        pub fn status(&self) -> &str {
            &self.status
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum ServerLastStatus {
        Optional,
        Outage,
        DegradedPerformance,
        PartialOutage,
        Unknown,
    }

    impl TryFrom<&String> for ServerLastStatus {
        type Error = anyhow::Error;

        fn try_from(value: &String) -> Result<Self, Self::Error> {
            Self::try_from(value.as_str())
        }
    }

    impl TryFrom<&str> for ServerLastStatus {
        type Error = anyhow::Error;

        fn try_from(value: &str) -> Result<Self, Self::Error> {
            Ok(match value {
                "operational" => Self::Optional,
                "major_outage" => Self::Outage,
                "partial_outage" => Self::PartialOutage,
                "degraded_performance" => ServerLastStatus::DegradedPerformance,
                _ => Self::Unknown,
            })
        }
    }

    impl std::fmt::Display for ServerLastStatus {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    ServerLastStatus::Optional => "operational",
                    ServerLastStatus::Outage => "major_outage",
                    ServerLastStatus::DegradedPerformance => "degraded_performance",
                    ServerLastStatus::PartialOutage => "partial_outage",
                    ServerLastStatus::Unknown => "unknown",
                }
            )
        }
    }
}

pub use current::VERSION as CURRENT_VERSION;
pub use datastructure_current::TransferData;
pub use datastructure_v1 as datastructure_current;
pub use v1 as current;
