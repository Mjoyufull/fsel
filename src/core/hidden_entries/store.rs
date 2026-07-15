use super::{EntryKey, HiddenEntry, HiddenEntryId, NewHiddenEntry};
use crate::core::cache::{HIDDEN_ENTRIES_TABLE, HIDDEN_ENTRY_META_TABLE};
use eyre::{Result, WrapErr, eyre};
use redb::{ReadableDatabase, ReadableTable};
use std::collections::HashSet;
use std::sync::Arc;

const NEXT_ID_KEY: &str = "next_id";

pub(crate) struct HiddenEntryStore {
    db: Arc<redb::Database>,
}

impl HiddenEntryStore {
    pub(crate) fn new(db: Arc<redb::Database>) -> Result<Self> {
        let write_txn = db.begin_write()?;
        write_txn.open_table(HIDDEN_ENTRIES_TABLE)?;
        write_txn.open_table(HIDDEN_ENTRY_META_TABLE)?;
        write_txn.commit()?;
        Ok(Self { db })
    }

    pub(crate) fn insert(&self, entry: NewHiddenEntry) -> Result<HiddenEntry> {
        let write_txn = self.db.begin_write()?;
        let stored_entry = super::model::StoredHiddenEntry::from_new(entry);
        let greatest_id = {
            let table = write_txn.open_table(HIDDEN_ENTRIES_TABLE)?;
            let mut greatest_id = 0;
            for row in table.iter()? {
                let (id_guard, value_guard) = row?;
                let id = id_guard.value();
                let existing = decode_entry(id, value_guard.value())?;
                if existing.entry_key() == stored_entry.entry_key() {
                    return Ok(existing);
                }
                greatest_id = greatest_id.max(id);
            }
            greatest_id
        };

        let next_id = {
            let mut meta_table = write_txn.open_table(HIDDEN_ENTRY_META_TABLE)?;
            let next_id = meta_table
                .get(NEXT_ID_KEY)?
                .map(|value| value.value())
                .unwrap_or_else(|| greatest_id.saturating_add(1).max(1));
            let following_id = next_id
                .checked_add(1)
                .ok_or_else(|| eyre!("hidden entry ID space is exhausted"))?;
            meta_table.insert(NEXT_ID_KEY, following_id)?;
            next_id
        };

        let inserted_entry = {
            let mut table = write_txn.open_table(HIDDEN_ENTRIES_TABLE)?;
            let encoded = postcard::to_allocvec(&stored_entry)?;
            table.insert(next_id, encoded.as_slice())?;
            stored_entry.into_entry(next_id)
        };
        write_txn.commit()?;
        Ok(inserted_entry)
    }

    pub(crate) fn list(&self) -> Result<Vec<HiddenEntry>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(HIDDEN_ENTRIES_TABLE)?;
        let mut entries = Vec::new();

        for row in table.iter()? {
            let (id_guard, value_guard) = row?;
            entries.push(decode_entry(id_guard.value(), value_guard.value())?);
        }

        Ok(entries)
    }

    pub(crate) fn entry_keys(&self) -> Result<HashSet<EntryKey>> {
        self.list().map(|entries| {
            entries
                .into_iter()
                .map(|entry| entry.entry_key().clone())
                .collect()
        })
    }

    pub(crate) fn remove(&self, id: HiddenEntryId) -> Result<Option<HiddenEntry>> {
        let write_txn = self.db.begin_write()?;
        let removed_entry = {
            let mut table = write_txn.open_table(HIDDEN_ENTRIES_TABLE)?;
            let entry = table
                .get(id.value())?
                .map(|value| decode_entry(id.value(), value.value()))
                .transpose()?;
            if entry.is_some() {
                table.remove(id.value())?;
            }
            entry
        };
        write_txn.commit()?;
        Ok(removed_entry)
    }

    pub(crate) fn remove_last(&self) -> Result<Option<HiddenEntry>> {
        let write_txn = self.db.begin_write()?;
        let removed_entry = {
            let mut table = write_txn.open_table(HIDDEN_ENTRIES_TABLE)?;
            let mut last_entry = None;
            for row in table.iter()? {
                let (id_guard, value_guard) = row?;
                last_entry = Some(decode_entry(id_guard.value(), value_guard.value())?);
            }
            if let Some(entry) = &last_entry {
                table.remove(entry.id().value())?;
            }
            last_entry
        };
        write_txn.commit()?;
        Ok(removed_entry)
    }

    pub(crate) fn remove_all(&self) -> Result<usize> {
        let write_txn = self.db.begin_write()?;
        let removed_count = {
            let mut table = write_txn.open_table(HIDDEN_ENTRIES_TABLE)?;
            let ids = table
                .iter()?
                .map(|row| row.map(|(id, _)| id.value()))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            for id in &ids {
                table.remove(*id)?;
            }
            ids.len()
        };
        write_txn.commit()?;
        Ok(removed_count)
    }
}

fn decode_entry(id: u64, encoded: &[u8]) -> Result<HiddenEntry> {
    postcard::from_bytes::<super::model::StoredHiddenEntry>(encoded)
        .map(|entry| entry.into_entry(id))
        .wrap_err_with(|| format!("failed to decode hidden entry {id}"))
}

#[cfg(test)]
mod tests {
    use super::HiddenEntryStore;
    use crate::core::hidden_entries::{EntryKey, HiddenEntryId, NewHiddenEntry};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_store(label: &str) -> (HiddenEntryStore, PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "fsel-hidden-{label}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("test directory should be created");
        let db_path = dir.join("history.redb");
        let db = Arc::new(redb::Database::create(db_path).expect("database should be created"));
        let store = HiddenEntryStore::new(db).expect("hidden entry store should initialize");
        (store, dir)
    }

    fn new_entry(path: &str, name: &str, timestamp: u64) -> NewHiddenEntry {
        let source = Path::new(path);
        NewHiddenEntry::new(
            EntryKey::desktop(source, "example.desktop"),
            name,
            path,
            timestamp,
        )
    }

    #[test]
    fn insert_is_idempotent_for_the_same_entry_key() {
        let (store, dir) = test_store("idempotent");

        let first = store
            .insert(new_entry("/one/example.desktop", "One", 10))
            .expect("first insert should succeed");
        let duplicate = store
            .insert(new_entry("/one/example.desktop", "Renamed", 20))
            .expect("duplicate insert should succeed");

        assert_eq!(first.id(), duplicate.id());
        assert_eq!(store.list().expect("list should succeed").len(), 1);

        drop(store);
        fs::remove_dir_all(dir).expect("test directory should be removed");
    }

    #[test]
    fn remove_last_uses_the_greatest_record_id() {
        let (store, dir) = test_store("remove-last");
        let first = store
            .insert(new_entry("/one/example.desktop", "One", 10))
            .expect("first insert should succeed");
        let second = store
            .insert(new_entry("/two/example.desktop", "Two", 20))
            .expect("second insert should succeed");

        let removed = store
            .remove_last()
            .expect("remove last should succeed")
            .expect("last entry should exist");

        assert_eq!(removed.id(), second.id());
        assert_eq!(
            store.list().expect("list should succeed")[0].id(),
            first.id()
        );

        drop(store);
        fs::remove_dir_all(dir).expect("test directory should be removed");
    }

    #[test]
    fn remove_and_remove_all_report_affected_records() {
        let (store, dir) = test_store("remove");
        let first = store
            .insert(new_entry("/one/example.desktop", "One", 10))
            .expect("first insert should succeed");
        store
            .insert(new_entry("/two/example.desktop", "Two", 20))
            .expect("second insert should succeed");

        assert!(
            store
                .remove(HiddenEntryId::new(first.id().value()))
                .expect("remove should succeed")
                .is_some()
        );
        assert_eq!(store.remove_all().expect("remove all should succeed"), 1);
        assert!(store.list().expect("list should succeed").is_empty());

        drop(store);
        fs::remove_dir_all(dir).expect("test directory should be removed");
    }

    #[test]
    fn record_ids_are_not_reused_after_restore() {
        let (store, dir) = test_store("monotonic");
        let first = store
            .insert(new_entry("/one/example.desktop", "One", 10))
            .expect("first insert should succeed");
        store
            .remove(first.id())
            .expect("remove should succeed")
            .expect("record should exist");

        let second = store
            .insert(new_entry("/two/example.desktop", "Two", 20))
            .expect("second insert should succeed");

        assert!(second.id() > first.id());

        drop(store);
        fs::remove_dir_all(dir).expect("test directory should be removed");
    }

    #[test]
    fn records_survive_database_reopen() {
        let (store, dir) = test_store("reopen");
        let expected = store
            .insert(new_entry("/one/example.desktop", "One", 10))
            .expect("record should be inserted");
        drop(store);

        let db = Arc::new(
            redb::Database::open(dir.join("history.redb")).expect("database should reopen"),
        );
        let reopened = HiddenEntryStore::new(db).expect("store should reopen");

        assert_eq!(reopened.list().expect("record should load"), vec![expected]);

        drop(reopened);
        fs::remove_dir_all(dir).expect("test directory should be removed");
    }
}
