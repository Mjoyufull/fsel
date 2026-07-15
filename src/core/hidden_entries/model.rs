use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub(crate) struct EntryKey(String);

impl EntryKey {
    pub(crate) fn desktop(source_path: &Path, desktop_id: &str) -> Self {
        Self(format!(
            "v1:desktop:{}:{desktop_id}",
            crate::core::path_key::encode(source_path)
        ))
    }

    pub(crate) fn executable(source_path: &Path) -> Self {
        Self(format!(
            "v1:executable:{}",
            crate::core::path_key::encode(source_path)
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct HiddenEntryId(u64);

impl HiddenEntryId {
    pub(crate) fn new(value: u64) -> Self {
        Self(value)
    }

    pub(crate) fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct NewHiddenEntry {
    entry_key: EntryKey,
    display_name: String,
    source_display: String,
    hidden_at_unix_ms: u64,
}

impl NewHiddenEntry {
    pub(crate) fn new(
        entry_key: EntryKey,
        display_name: impl Into<String>,
        source_display: impl Into<String>,
        hidden_at_unix_ms: u64,
    ) -> Self {
        Self {
            entry_key,
            display_name: display_name.into(),
            source_display: source_display.into(),
            hidden_at_unix_ms,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HiddenEntry {
    id: HiddenEntryId,
    data: StoredHiddenEntry,
}

impl HiddenEntry {
    pub(crate) fn id(&self) -> HiddenEntryId {
        self.id
    }

    pub(crate) fn entry_key(&self) -> &EntryKey {
        &self.data.entry_key
    }

    pub(crate) fn display_name(&self) -> &str {
        &self.data.display_name
    }

    pub(crate) fn source_display(&self) -> &str {
        &self.data.source_display
    }

    pub(crate) fn hidden_at_unix_ms(&self) -> u64 {
        self.data.hidden_at_unix_ms
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct StoredHiddenEntry {
    entry_key: EntryKey,
    display_name: String,
    source_display: String,
    hidden_at_unix_ms: u64,
}

impl StoredHiddenEntry {
    pub(super) fn from_new(entry: NewHiddenEntry) -> Self {
        Self {
            entry_key: entry.entry_key,
            display_name: entry.display_name,
            source_display: entry.source_display,
            hidden_at_unix_ms: entry.hidden_at_unix_ms,
        }
    }

    pub(super) fn into_entry(self, id: u64) -> HiddenEntry {
        HiddenEntry {
            id: HiddenEntryId::new(id),
            data: self,
        }
    }

    pub(super) fn entry_key(&self) -> &EntryKey {
        &self.entry_key
    }
}

#[cfg(test)]
mod tests {
    use super::EntryKey;
    use std::path::Path;

    #[test]
    fn entry_keys_distinguish_sources_and_kinds() {
        let first_path = Path::new("/strata/arch/applications/editor.desktop");
        let second_path = Path::new("/strata/debian/applications/editor.desktop");

        let first = EntryKey::desktop(first_path, "editor.desktop");
        let second = EntryKey::desktop(second_path, "editor.desktop");
        let executable = EntryKey::executable(first_path);

        assert_ne!(first, second);
        assert_ne!(first, executable);
    }
}
