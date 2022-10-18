pub mod v1 {
    pub const CREATE_TABLE: &str = r#"CREATE TABLE "machines" (
            "uuid"	TEXT NOT NULL,
            "status"	TEXT NOT NULL,
            "last_update"	INTEGER NOT NULL,
            "page"   TEXT,
            "component_id" TEXT,
        );
        CREATE TABLE "upstream_meta" (
            "key"	TEXT NOT NULL,
            "value"	TEXT NOT NULL,
            PRIMARY KEY("key")
        );
        INSERT INTO "fdc_meta" VALUES ("version", "1");
        "#;
    pub const VERSION: &str = "1";
}

pub fn get_current_timestamp() -> u64 {
    let start = std::time::SystemTime::now();
    let since_the_epoch = start
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards");
    since_the_epoch.as_secs()
}
