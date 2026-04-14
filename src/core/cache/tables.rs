use redb::TableDefinition;

pub const DESKTOP_CACHE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("desktop_cache");
pub const NAME_INDEX_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("app_name_index");
pub const FILE_LIST_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("file_list_cache");
pub const HISTORY_TABLE: TableDefinition<&str, u64> = TableDefinition::new("history");
pub const PINNED_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("pinned_apps");
pub const FRECENCY_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("frecency");
