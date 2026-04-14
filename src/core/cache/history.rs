use super::tables::{HISTORY_TABLE, PINNED_TABLE};
use crate::desktop::App;
use eyre::Result;
use redb::{Database, ReadableDatabase, ReadableTable};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

const PINNED_APPS_KEY: &str = "pinned_apps";

pub struct HistoryCache {
    pub history: HashMap<String, u64>,
    pub pinned: HashSet<String>,
}

impl HistoryCache {
    pub fn load(db: &Arc<Database>) -> Result<Self> {
        let mut history = HashMap::new();
        let mut pinned = HashSet::new();

        let read_txn = db.begin_read()?;

        if let Ok(history_table) = read_txn.open_table(HISTORY_TABLE) {
            for entry in history_table.iter()? {
                let (key, value) = entry?;
                history.insert(key.value().to_string(), value.value());
            }
        }

        if let Ok(pinned_table) = read_txn.open_table(PINNED_TABLE)
            && let Some(data) = pinned_table.get(PINNED_APPS_KEY)?
            && let Ok(apps) = postcard::from_bytes::<Vec<String>>(data.value())
        {
            pinned.extend(apps);
        }

        Ok(Self { history, pinned })
    }

    pub fn get_history(&self, app_name: &str) -> u64 {
        self.history.get(app_name).copied().unwrap_or(0)
    }

    pub fn is_pinned(&self, app_name: &str) -> bool {
        self.pinned.contains(app_name)
    }

    pub fn apply_to_app(&self, mut app: App) -> App {
        app.history = self.get_history(&app.name);
        app.pinned = self.is_pinned(&app.name);
        app
    }

    pub fn get_best_match(&self, prefix: &str) -> Option<(&String, u64)> {
        let prefix_lower = prefix.to_lowercase();

        self.history
            .iter()
            .filter(|(name, _)| name.to_lowercase().starts_with(&prefix_lower))
            .max_by_key(|(_, count)| *count)
            .map(|(name, count)| (name, *count))
    }
}
