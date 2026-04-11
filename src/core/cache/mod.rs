mod desktop;
mod history;
mod tables;

#[allow(unused_imports)]
pub use desktop::DesktopCache;
#[allow(unused_imports)]
pub use history::HistoryCache;
#[allow(unused_imports)]
pub use tables::{
    DESKTOP_CACHE_TABLE, FILE_LIST_TABLE, FRECENCY_TABLE, HISTORY_TABLE, NAME_INDEX_TABLE,
    PINNED_TABLE,
};
