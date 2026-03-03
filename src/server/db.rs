use crate::model::{ComponentInfo, ComponentStatus, HistoryEntry};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS "components" (
    "id"             TEXT NOT NULL PRIMARY KEY,
    "name"           TEXT NOT NULL,
    "current_status" TEXT NOT NULL DEFAULT 'unknown',
    "last_updated"   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS "check_history" (
    "id"            INTEGER PRIMARY KEY AUTOINCREMENT,
    "component_id"  TEXT NOT NULL REFERENCES "components"("id"),
    "status"        TEXT NOT NULL,
    "message"       TEXT,
    "latency_ms"    INTEGER,
    "reported_by"   TEXT,
    "created_at"    INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS "idx_history_component"
    ON "check_history"("component_id", "created_at" DESC);

CREATE TABLE IF NOT EXISTS "notifier_state" (
    "notifier"       TEXT NOT NULL,
    "component_id"   TEXT NOT NULL,
    "last_status"    TEXT NOT NULL,
    "last_notified"  INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY ("notifier", "component_id")
);
"#;

pub async fn connect(database_path: &str) -> anyhow::Result<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(database_path)
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    sqlx::raw_sql(SCHEMA).execute(&pool).await?;

    Ok(pool)
}

pub async fn ensure_component(pool: &SqlitePool, id: &str, name: &str) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT OR IGNORE INTO "components" ("id", "name", "current_status", "last_updated")
           VALUES (?, ?, 'unknown', 0)"#,
    )
    .bind(id)
    .bind(name)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_component(pool: &SqlitePool, id: &str) -> anyhow::Result<Option<ComponentInfo>> {
    let row = sqlx::query(
        r#"SELECT "id", "name", "current_status", "last_updated"
           FROM "components" WHERE "id" = ?"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let status_str: String = row.get("current_status");
            let status: ComponentStatus =
                serde_json::from_value(serde_json::Value::String(status_str))
                    .unwrap_or(ComponentStatus::Unknown);
            Ok(Some(ComponentInfo::new(
                row.get("id"),
                row.get("name"),
                status,
                row.get::<i64, _>("last_updated") as u64,
            )))
        }
        None => Ok(None),
    }
}

pub async fn get_all_components(pool: &SqlitePool) -> anyhow::Result<Vec<ComponentInfo>> {
    let rows = sqlx::query(
        r#"SELECT "id", "name", "current_status", "last_updated" FROM "components" ORDER BY "id""#,
    )
    .fetch_all(pool)
    .await?;

    let mut components = Vec::with_capacity(rows.len());
    for row in rows {
        let status_str: String = row.get("current_status");
        let status: ComponentStatus = serde_json::from_value(serde_json::Value::String(status_str))
            .unwrap_or(ComponentStatus::Unknown);
        components.push(ComponentInfo::new(
            row.get("id"),
            row.get("name"),
            status,
            row.get::<i64, _>("last_updated") as u64,
        ));
    }
    Ok(components)
}

/// Insert a check result into history and update component status if changed.
/// Returns the previous status if it changed, or None.
pub async fn record_check(
    pool: &SqlitePool,
    component_id: &str,
    status: ComponentStatus,
    message: Option<&str>,
    latency_ms: Option<u64>,
    reported_by: Option<&str>,
    timestamp: u64,
) -> anyhow::Result<Option<ComponentStatus>> {
    let status_str = status.to_string();

    sqlx::query(
        r#"INSERT INTO "check_history"
           ("component_id", "status", "message", "latency_ms", "reported_by", "created_at")
           VALUES (?, ?, ?, ?, ?, ?)"#,
    )
    .bind(component_id)
    .bind(&status_str)
    .bind(message)
    .bind(latency_ms.map(|v| v as i64))
    .bind(reported_by)
    .bind(timestamp as i64)
    .execute(pool)
    .await?;

    let row = sqlx::query(r#"SELECT "current_status" FROM "components" WHERE "id" = ?"#)
        .bind(component_id)
        .fetch_optional(pool)
        .await?;

    let old_status = row.map(|r| {
        let s: String = r.get("current_status");
        serde_json::from_value::<ComponentStatus>(serde_json::Value::String(s))
            .unwrap_or(ComponentStatus::Unknown)
    });

    let changed = old_status.map_or(false, |old| old != status);

    sqlx::query(
        r#"UPDATE "components" SET "current_status" = ?, "last_updated" = ? WHERE "id" = ?"#,
    )
    .bind(&status_str)
    .bind(timestamp as i64)
    .bind(component_id)
    .execute(pool)
    .await?;

    if changed {
        Ok(old_status)
    } else {
        Ok(None)
    }
}

pub async fn get_history(
    pool: &SqlitePool,
    component_id: &str,
    limit: i64,
    since: Option<u64>,
) -> anyhow::Result<Vec<HistoryEntry>> {
    let rows = if let Some(since) = since {
        sqlx::query(
            r#"SELECT "status", "message", "latency_ms", "reported_by", "created_at"
               FROM "check_history"
               WHERE "component_id" = ? AND "created_at" >= ?
               ORDER BY "created_at" DESC
               LIMIT ?"#,
        )
        .bind(component_id)
        .bind(since as i64)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"SELECT "status", "message", "latency_ms", "reported_by", "created_at"
               FROM "check_history"
               WHERE "component_id" = ?
               ORDER BY "created_at" DESC
               LIMIT ?"#,
        )
        .bind(component_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let status_str: String = row.get("status");
        let status: ComponentStatus = serde_json::from_value(serde_json::Value::String(status_str))
            .unwrap_or(ComponentStatus::Unknown);
        entries.push(HistoryEntry::new(
            status,
            row.get("message"),
            row.get::<Option<i64>, _>("latency_ms").map(|v| v as u64),
            row.get("reported_by"),
            row.get::<i64, _>("created_at") as u64,
        ));
    }
    Ok(entries)
}
