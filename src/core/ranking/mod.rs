//! Pure ranking and frecency policy for launcher search.

mod frecency;
mod query;
mod sort;

pub use frecency::{FrecencyEntry, age_entries, current_unix_seconds};
pub use query::{FilterOptions, ScoreBreakdown, filter_apps};
pub use sort::sort_by_ranking;
