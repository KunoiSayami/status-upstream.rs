pub mod v1 {
    pub const CREATE_TABLE: &str = r#"CREATE TABLE "machines" (
            "uuid"	TEXT NOT NULL,
            "status"	TEXT NOT NULL,
            "last_update"	TEXT,
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
