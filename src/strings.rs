// String manipulation utilities

/// Extract executable name from a command string
/// Takes the first word and strips any path components
///
/// Examples:
/// - "/usr/bin/firefox" -> "firefox"
/// - "firefox --new-window" -> "firefox"
/// - "env FOO=bar firefox" -> "env"
///
/// Optimized to avoid unnecessary allocations
#[inline]
pub fn extract_exec_name(command: &str) -> &str {
    command
        .split_whitespace()
        .next()
        .and_then(|cmd| cmd.rsplit('/').next())
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_exec_name() {
        assert_eq!(extract_exec_name("/usr/bin/firefox"), "firefox");
        assert_eq!(extract_exec_name("firefox --new-window"), "firefox");
        assert_eq!(extract_exec_name("env FOO=bar firefox"), "env");
        assert_eq!(extract_exec_name(""), "");
        assert_eq!(extract_exec_name("   "), "");
    }
}
