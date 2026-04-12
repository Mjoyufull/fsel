mod keyboard;
mod mouse;
mod selection;

use super::image::ImageRuntime;
use super::state::CclipOptions;
use crate::cli::Opts;
use crate::ui::{AsyncInput, DmenuUI, InputEvent as Event};
use crossterm::event::KeyEvent;
use eyre::Result;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::collections::HashMap;
use std::io;

use super::{TagMetadata, TagMetadataFormatter};

pub(super) enum LoopControl {
    Continue,
    Exit,
}

pub(super) struct EventOutcome {
    pub(super) control: LoopControl,
    pub(super) needs_redraw: bool,
}

pub(super) struct EventContext<'a, 'ui> {
    pub(super) ui: &'a mut DmenuUI<'ui>,
    pub(super) terminal: &'a mut Terminal<CrosstermBackend<io::Stderr>>,
    pub(super) cli: &'a Opts,
    pub(super) options: &'a CclipOptions,
    pub(super) db: &'a std::sync::Arc<redb::Database>,
    pub(super) tag_metadata_map: &'a mut HashMap<String, TagMetadata>,
    pub(super) tag_metadata_formatter: &'a mut TagMetadataFormatter,
    pub(super) image_runtime: &'a mut ImageRuntime,
    pub(super) max_visible: usize,
}

pub(super) async fn handle_event(
    mut ctx: EventContext<'_, '_>,
    event: Event<KeyEvent>,
    input: &mut AsyncInput,
) -> Result<EventOutcome> {
    match event {
        Event::Input(key) => keyboard::handle_key_event(&mut ctx, input, key).await,
        Event::Mouse(mouse_event) => mouse::handle_mouse_event(&mut ctx, mouse_event),
        Event::Tick => Ok(EventOutcome {
            control: LoopControl::Continue,
            needs_redraw: ctx.ui.temp_message.is_some(),
        }),
        Event::Render => Ok(EventOutcome {
            control: LoopControl::Continue,
            needs_redraw: true,
        }),
    }
}
