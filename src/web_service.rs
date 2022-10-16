pub mod v1 {
    use super::TransferData;
    use axum::extract::Path;
    use axum::http::StatusCode;
    use axum::response::{IntoResponse, Response};
    use axum::{Json, Router};
    use serde_json::json;
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
        Path(path): Path<String>,
        Json(payload): Json<TransferData>,
        sql_conn: Arc<Mutex<SqliteConnection>>,
    ) -> impl IntoResponse {
        (StatusCode::OK, json!({"status": 200}).to_string()).into_response()
    }

    pub async fn get(Path(path): Path<String>, sql_conn: Arc<Mutex<SqliteConnection>>) -> Response {
        (
            StatusCode::OK,
            serde_json::to_string(&TransferData::default()).unwrap(),
        )
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
