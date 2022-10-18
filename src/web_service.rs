pub mod v1 {
    use crate::configure::Component;
    use crate::database::get_current_timestamp;
    use crate::datastructures::{ServerLastStatus, TransferData, UpstreamTrait};
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
    pub type FetchReturnType = (String, Option<String>, Option<String>);

    pub fn make_router(conn: SqliteConnection, upstream: Box<dyn UpstreamTrait>) -> Router {
        let conn = Arc::new(Mutex::new(conn));
        let upstream = Arc::new(upstream);
        Router::new()
            .route(
                "/v1/components/:component_id",
                axum::routing::get({
                    let conn = conn.clone();
                    |path| async move { get(Path(path), conn).await }
                })
                .post({
                    let conn = conn.clone();
                    let upstream = upstream.clone();
                    |path, payload| async move { post(path, payload, upstream, conn).await }
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
        upstream: Arc<Box<dyn UpstreamTrait>>,
        sql_conn: Arc<Mutex<SqliteConnection>>,
    ) -> impl IntoResponse {
        let last_status = ServerLastStatus::try_from(payload.status())
            .map_err(|e| error!("Got error while read data: {:?}", e));

        let last_status = match last_status {
            Ok(status) => status,
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({"status": 500}).to_string(),
                )
                    .into_response()
            }
        };

        let mut sql_conn = sql_conn.lock().await;

        let ret = sqlx::query_as::<_, FetchReturnType>(
            r#"SELECT "uuid", "page", "component_id" FROM "matchines" WHERE "uuid" = ?"#,
        )
        .bind(&uuid)
        .fetch_optional(&mut *sql_conn)
        .await
        .map_err(|e| error!("Fetch {} component error: {:?}", &uuid, e))
        .map(|r| {
            if r.is_none() {
                error!("Fetch component {} is null", &uuid)
            }
            r
        });

        let component = match ret {
            Ok(Some(ret)) => Component::from(ret),
            _ => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({"status": 500}).to_string(),
                )
                    .into_response();
            }
        };

        let query_ret = sqlx::query(
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

        let upstream_ret = upstream
            .set_component_status(component.report_id(), component.page(), last_status.into())
            .await
            .map_err(|e| error!("Got error while upload status to server: {:?}", e));

        if query_ret.is_ok() && upstream_ret.is_ok() {
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

pub use current::VERSION as CURRENT_VERSION;
pub use v1 as current;
