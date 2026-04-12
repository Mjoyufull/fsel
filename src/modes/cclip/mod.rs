// Cclip mode - clipboard history browser

mod commands;
mod events;
mod image;
mod items;
mod metadata;
mod model;
pub mod preview;
mod render;
pub mod run;
pub mod scan;
pub mod select;
mod session;
mod state;
mod tags;

// Re-export main entry point
pub use run::run;
pub(crate) use session::CclipSession;

// Re-export commonly used scan functions
pub use scan::{check_cclip_available, check_cclip_database};

pub use metadata::{TagMetadata, TagMetadataFormatter, load_tag_metadata, save_tag_metadata};
pub use model::CclipItem;
