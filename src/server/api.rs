use crate::model::{ClientReport, ComponentStatus, ReportResponse};
use crate::server::notifier::NotifierRegistry;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

pub struct AppState {
    pool: SqlitePool,
    notifiers: NotifierRegistry,
    auth_token: String,
    public_status_page: bool,
}

impl AppState {
    pub fn new(
        pool: SqlitePool,
        notifiers: NotifierRegistry,
        auth_token: String,
        public_status_page: bool,
    ) -> Self {
        Self {
            pool,
            notifiers,
            auth_token,
            public_status_page,
        }
    }
}

pub fn make_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", axum::routing::get(root))
        .route("/api/v1/health", axum::routing::get(health))
        .route("/api/v1/report", axum::routing::post(report))
        .route("/api/v1/components", axum::routing::get(list_components))
        .route("/api/v1/components/{id}", axum::routing::get(get_component))
        .route(
            "/api/v1/components/{id}/history",
            axum::routing::get(get_history),
        )
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
        .with_state(state)
}

async fn root() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "status": 200,
    }))
}

async fn health() -> StatusCode {
    StatusCode::OK
}

fn verify_bearer_token(headers: &HeaderMap, expected: &str) -> bool {
    if expected.is_empty() {
        return true;
    }
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map_or(false, |token| token == expected)
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error": "unauthorized"})),
    )
        .into_response()
}

async fn report(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(report): Json<ClientReport>,
) -> Response {
    if !verify_bearer_token(&headers, &state.auth_token) {
        return unauthorized();
    }

    let mut accepted = 0usize;
    let mut status_changes = Vec::new();

    for check in report.checks() {
        let result = super::db::record_check(
            &state.pool,
            check.component_id(),
            check.status(),
            check.message(),
            check.latency_ms(),
            Some(report.client_id()),
            check.timestamp(),
        )
        .await;

        match result {
            Ok(old_status) => {
                accepted += 1;
                if let Some(old) = old_status {
                    status_changes.push(check.component_id().to_string());
                    notify_status_change(&state, check.component_id(), old, check.status()).await;
                }
            }
            Err(e) => {
                tracing::error!("Failed to record check for {}: {e}", check.component_id());
            }
        }
    }

    (
        StatusCode::OK,
        Json(ReportResponse::new(accepted, status_changes)),
    )
        .into_response()
}

async fn notify_status_change(
    state: &AppState,
    component_id: &str,
    old: ComponentStatus,
    new: ComponentStatus,
) {
    let component_name = super::db::get_component(&state.pool, component_id)
        .await
        .ok()
        .flatten()
        .map(|c| c.name().to_string())
        .unwrap_or_else(|| component_id.to_string());

    state
        .notifiers
        .notify_all(component_id, &component_name, old, new)
        .await;
}

async fn list_components(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if !state.public_status_page && !verify_bearer_token(&headers, &state.auth_token) {
        return unauthorized();
    }

    match super::db::get_all_components(&state.pool).await {
        Ok(components) => (StatusCode::OK, Json(components)).into_response(),
        Err(e) => {
            tracing::error!("Failed to list components: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "internal server error"})),
            )
                .into_response()
        }
    }
}

async fn get_component(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    if !state.public_status_page && !verify_bearer_token(&headers, &state.auth_token) {
        return unauthorized();
    }

    match super::db::get_component(&state.pool, &id).await {
        Ok(Some(component)) => (StatusCode::OK, Json(component)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "component not found"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to get component {id}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "internal server error"})),
            )
                .into_response()
        }
    }
}

#[derive(Deserialize)]
struct HistoryQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    since: Option<u64>,
}

fn default_limit() -> i64 {
    50
}

async fn get_history(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> Response {
    if !state.public_status_page && !verify_bearer_token(&headers, &state.auth_token) {
        return unauthorized();
    }

    match super::db::get_history(&state.pool, &id, query.limit, query.since).await {
        Ok(history) => (StatusCode::OK, Json(history)).into_response(),
        Err(e) => {
            tracing::error!("Failed to get history for {id}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "internal server error"})),
            )
                .into_response()
        }
    }
}
