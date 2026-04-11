use super::State;
use ratatui::style::Color;
use std::time::SystemTime;

impl State {
    /// Update info text based on the selected app.
    pub fn update_info(&mut self, _highlight_color: Color, fancy_mode: bool, verbose: u64) {
        if let Some(selected) = self.selected
            && let Some(app) = self.shown.get(selected)
        {
            self.text = if fancy_mode {
                app.description.clone()
            } else {
                format!("{}\n\n{}", app.name, app.description)
            };

            if verbose > 1 {
                self.text.push_str("\n\n");

                if app.is_terminal {
                    self.text
                        .push_str(&format!("Exec (terminal): {}\n", app.command));
                } else {
                    self.text.push_str(&format!("Exec: {}\n", app.command));
                }

                if let Some(generic_name) = &app.generic_name {
                    self.text
                        .push_str(&format!("Generic Name: {}\n", generic_name));
                }

                if !app.categories.is_empty() {
                    self.text
                        .push_str(&format!("Categories: {}\n", app.categories.join(", ")));
                }

                if !app.keywords.is_empty() {
                    self.text
                        .push_str(&format!("Keywords: {}\n", app.keywords.join(", ")));
                }

                if verbose > 2 {
                    if !app.mime_types.is_empty() {
                        self.text
                            .push_str(&format!("MIME Types: {}\n", app.mime_types.join(", ")));
                    }
                    self.text.push_str(&format!("Type: {}\n", app.entry_type));
                    if let Some(icon) = &app.icon {
                        self.text.push_str(&format!("Icon: {}\n", icon));
                    }
                    if let Some(timestamp) = app.last_access {
                        self.text
                            .push_str(&format!("Last Run: {}\n", format_recency(timestamp)));
                    }
                    self.text.push_str(&format!("Times run: {}\n", app.history));
                    self.text
                        .push_str(&format!("Matching score: {}\n", app.score));
                }
            }
        }
    }
}

fn format_recency(timestamp: u64) -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let diff = now.saturating_sub(timestamp);

    if diff < 60 {
        format!("{}s ago", diff)
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}
