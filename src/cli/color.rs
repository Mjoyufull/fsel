/// Parses a [String] into a ratatui [color].
///
/// Case-insensitive.
pub fn string_to_color<T: Into<String>>(val: T) -> Result<ratatui::style::Color, &'static str> {
    let color_str = val.into();
    let color_lower = color_str.to_lowercase();

    if let Some(hex_color) = parse_hex_color(&color_str) {
        return Ok(hex_color);
    }

    if let Some(rgb_color) = parse_rgb_color(&color_str) {
        return Ok(rgb_color);
    }

    if let Ok(index) = color_str.parse::<u8>() {
        return Ok(ratatui::style::Color::Indexed(index));
    }

    match color_lower.as_ref() {
        "black" => Ok(ratatui::style::Color::Black),
        "red" => Ok(ratatui::style::Color::Red),
        "green" => Ok(ratatui::style::Color::Green),
        "yellow" => Ok(ratatui::style::Color::Yellow),
        "blue" => Ok(ratatui::style::Color::Blue),
        "magenta" | "purple" => Ok(ratatui::style::Color::Magenta),
        "cyan" | "teal" => Ok(ratatui::style::Color::Cyan),
        "gray" | "grey" => Ok(ratatui::style::Color::Gray),
        "darkgray" | "darkgrey" => Ok(ratatui::style::Color::DarkGray),
        "lightred" => Ok(ratatui::style::Color::LightRed),
        "lightgreen" => Ok(ratatui::style::Color::LightGreen),
        "lightyellow" => Ok(ratatui::style::Color::LightYellow),
        "lightblue" => Ok(ratatui::style::Color::LightBlue),
        "lightmagenta" | "pink" => Ok(ratatui::style::Color::LightMagenta),
        "lightcyan" => Ok(ratatui::style::Color::LightCyan),
        "white" => Ok(ratatui::style::Color::White),
        "reset" => Ok(ratatui::style::Color::Reset),
        "orange" => Ok(ratatui::style::Color::Indexed(208)),
        "brown" => Ok(ratatui::style::Color::Indexed(130)),
        "lime" => Ok(ratatui::style::Color::Indexed(46)),
        "navy" => Ok(ratatui::style::Color::Indexed(17)),
        "maroon" => Ok(ratatui::style::Color::Indexed(88)),
        "olive" => Ok(ratatui::style::Color::Indexed(58)),
        "silver" => Ok(ratatui::style::Color::Indexed(7)),
        _ => Err(
            "unknown color format. Use: named colors (red, blue, etc.), hex (#ff0000), RGB (rgb(255,0,0)), or 8-bit index (0-255)",
        ),
    }
}

fn parse_hex_color(color_str: &str) -> Option<ratatui::style::Color> {
    let hex = color_str.strip_prefix('#').unwrap_or(color_str);

    if hex.len() == 6
        && hex.chars().all(|c| c.is_ascii_hexdigit())
        && let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        )
    {
        return Some(ratatui::style::Color::Rgb(r, g, b));
    }

    if hex.len() == 3
        && hex.chars().all(|c| c.is_ascii_hexdigit())
        && let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&format!("{}{}", &hex[0..1], &hex[0..1]), 16),
            u8::from_str_radix(&format!("{}{}", &hex[1..2], &hex[1..2]), 16),
            u8::from_str_radix(&format!("{}{}", &hex[2..3], &hex[2..3]), 16),
        )
    {
        return Some(ratatui::style::Color::Rgb(r, g, b));
    }

    None
}

fn parse_rgb_color(color_str: &str) -> Option<ratatui::style::Color> {
    let rgb_str = color_str.trim();
    let rgb_lower = rgb_str.to_ascii_lowercase();

    if rgb_lower.starts_with("rgb(") && rgb_str.ends_with(')') {
        let values = &rgb_str[4..rgb_str.len() - 1];
        return parse_rgb_values(values);
    }

    if rgb_str.starts_with('(') && rgb_str.ends_with(')') {
        let values = &rgb_str[1..rgb_str.len() - 1];
        return parse_rgb_values(values);
    }

    None
}

fn parse_rgb_values(values: &str) -> Option<ratatui::style::Color> {
    let parts: Vec<&str> = values.split(',').map(str::trim).collect();
    if parts.len() == 3
        && let (Ok(r), Ok(g), Ok(b)) = (
            parts[0].parse::<u8>(),
            parts[1].parse::<u8>(),
            parts[2].parse::<u8>(),
        )
    {
        return Some(ratatui::style::Color::Rgb(r, g, b));
    }

    None
}
