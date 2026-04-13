use eyre::{Result, WrapErr, eyre};

use crate::cli::Opts;

pub(super) fn validate_environment() -> Result<()> {
    if !super::scan::check_cclip_available() {
        return Err(eyre!(
            "cclip is not available. Please install cclip and ensure it's in your PATH."
        ));
    }

    super::scan::check_cclip_database().wrap_err("cclip database check failed")
}

pub(super) fn handle_noninteractive_mode(cli: &Opts) -> Result<bool> {
    if cli.cclip_clear_tags {
        clear_tag_metadata()?;
        println!("Cleared all tag metadata from fsel database");
        println!();
        println!("Note: To wipe tags from cclip entries too, use:");
        println!("  fsel --cclip --tag wipe");
        return Ok(true);
    }

    if cli.cclip_wipe_tags {
        super::select::wipe_all_tags().wrap_err("Failed to wipe cclip tags")?;
        println!("Wiped all tags from cclip entries");
        clear_tag_metadata()?;
        println!("Cleared all tag metadata from fsel database");
        return Ok(true);
    }

    if cli.cclip_tag_list {
        print_tag_list(cli)?;
        return Ok(true);
    }

    Ok(false)
}

pub(super) fn load_history(cli: &Opts) -> Result<Vec<super::CclipItem>> {
    if let Some(ref tag_name) = cli.cclip_tag {
        super::scan::get_clipboard_history_by_tag(tag_name).wrap_err(format!(
            "Failed to get clipboard history for tag '{}'",
            tag_name
        ))
    } else {
        super::scan::get_clipboard_history().wrap_err("Failed to get clipboard history from cclip")
    }
}

fn clear_tag_metadata() -> Result<()> {
    let (db, _) = crate::core::database::open_history_db()?;
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(super::metadata::TAG_METADATA_TABLE)?;
        let _ = table.remove("tag_metadata");
    }
    write_txn.commit()?;
    Ok(())
}

fn print_tag_list(cli: &Opts) -> Result<()> {
    let tags = super::scan::get_all_tags().wrap_err("Failed to get tags from cclip")?;

    if tags.is_empty() {
        println!("No tags found");
        return Ok(());
    }

    if let Some(ref tag_name) = cli.cclip_tag {
        println!("Items tagged with '{}':", tag_name);
        let items = super::scan::get_clipboard_history_by_tag(tag_name)
            .wrap_err("Failed to get items by tag")?;

        if items.is_empty() {
            println!("  (no items)");
            return Ok(());
        }

        for item in items {
            if cli.verbose.unwrap_or(0) >= 2 {
                println!("  [{}] {} - {}", item.rowid, item.mime_type, item.preview);
            } else {
                println!("  {}", item.preview);
            }
        }
        return Ok(());
    }

    println!("Available tags:");
    for tag in tags {
        if cli.verbose.unwrap_or(0) >= 2 {
            let items = match super::scan::get_clipboard_history_by_tag(&tag) {
                Ok(items) => items,
                Err(error) => {
                    eprintln!(
                        "Failed to load clipboard history for tag '{}': {}",
                        tag, error
                    );
                    Vec::new()
                }
            };
            println!("  {} ({} items)", tag, items.len());
        } else {
            println!("  {}", tag);
        }
    }

    Ok(())
}
