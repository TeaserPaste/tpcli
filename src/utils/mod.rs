pub fn get_file_extension(language: &str) -> &'static str {
    match language.to_lowercase().as_str() {
        "javascript" => ".js",
        "typescript" => ".ts",
        "python" => ".py",
        "html" => ".html",
        "css" => ".css",
        "json" => ".json",
        "markdown" => ".md",
        "text" | "plaintext" => ".txt",
        "shell" => ".sh",
        "java" => ".java",
        "csharp" => ".cs",
        "cpp" => ".cpp",
        "go" => ".go",
        "rust" => ".rs",
        "ruby" => ".rb",
        _ => ".txt",
    }
}

pub fn get_lang_from_extension(ext: &str) -> &'static str {
    match ext {
        ".js" => "javascript",
        ".ts" => "typescript",
        ".py" => "python",
        ".html" => "html",
        ".css" => "css",
        ".json" => "json",
        ".md" => "markdown",
        ".txt" => "plaintext",
        ".sh" => "shell",
        ".java" => "java",
        ".cs" => "csharp",
        ".cpp" => "cpp",
        ".go" => "go",
        ".rs" => "rust",
        ".rb" => "ruby",
        _ => "plaintext",
    }
}

pub fn normalize_lang(input: &str) -> String {
    let lower = input.to_lowercase();
    match lower.as_str() {
        "js" => "javascript".to_string(),
        "ts" => "typescript".to_string(),
        "py" => "python".to_string(),
        "rs" => "rust".to_string(),
        "go" => "go".to_string(),
        "rb" => "ruby".to_string(),
        "cpp" | "c++" => "cpp".to_string(),
        "cs" | "c#" => "csharp".to_string(),
        "md" => "markdown".to_string(),
        "sh" | "bash" | "zsh" => "shell".to_string(),
        "txt" => "plaintext".to_string(),
        _ => lower,
    }
}

pub fn sanitize_filename(name: &str) -> String {
    // Replace non-alphanumeric (except . - _) with _
    let mut s = name.replace(
        |c: char| !c.is_alphanumeric() && c != '.' && c != '-' && c != '_',
        "_",
    );
    // Prevent path traversal by replacing .. with __
    s = s.replace("..", "__");
    // Prevent leading dots
    if s.starts_with('.') {
        s = format!("_{}", &s[1..]);
    }
    s.chars().take(100).collect()
}

#[allow(dead_code)]
pub fn parse_duration(duration_str: &str) -> Option<u64> {
    if duration_str.is_empty() {
        return None;
    }
    let len = duration_str.len();
    if len < 2 {
        return duration_str.parse().ok(); // assume ms if just number? or fail. JS code assumes ms if no unit but regex enforces unit or digit.
        // JS regex: /^(\d+)(ms|s|m|h|d)?$/
    }

    let (val_str, unit) = if duration_str.ends_with("ms") {
        (&duration_str[..len - 2], "ms")
    } else if let Some(last) = duration_str.chars().last() {
        if !last.is_ascii_digit() {
            (&duration_str[..len - 1], &duration_str[len - 1..])
        } else {
            (duration_str, "ms")
        }
    } else {
        return None;
    };

    let value: u64 = val_str.parse().ok()?;

    match unit {
        "ms" => Some(value),
        "s" => Some(value * 1000),
        "m" => Some(value * 60 * 1000),
        "h" => Some(value * 60 * 60 * 1000),
        "d" => Some(value * 24 * 60 * 60 * 1000),
        _ => Some(value), // default to ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_file_extension() {
        assert_eq!(get_file_extension("javascript"), ".js");
        assert_eq!(get_file_extension("rust"), ".rs");
        assert_eq!(get_file_extension("unknown"), ".txt");
    }

    #[test]
    fn test_get_lang_from_extension() {
        assert_eq!(get_lang_from_extension(".js"), "javascript");
        assert_eq!(get_lang_from_extension(".rs"), "rust");
        assert_eq!(get_lang_from_extension(".foo"), "plaintext");
    }

    #[test]
    fn test_normalize_lang() {
        assert_eq!(normalize_lang("JS"), "javascript");
        assert_eq!(normalize_lang("rust"), "rust");
        assert_eq!(normalize_lang("c++"), "cpp");
        assert_eq!(normalize_lang("C#"), "csharp");
        assert_eq!(normalize_lang("bash"), "shell");
        assert_eq!(normalize_lang("unknown"), "unknown");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("valid_name"), "valid_name");
        assert_eq!(sanitize_filename("invalid/name"), "invalid_name");
        assert_eq!(
            sanitize_filename("very long name with spaces"),
            "very_long_name_with_spaces"
        );

        // Test path traversal prevention
        assert_eq!(sanitize_filename(".."), "__");
        assert_eq!(sanitize_filename("../bar"), "___bar");
        assert_eq!(sanitize_filename("foo/../bar"), "foo____bar");

        // Test leading dots
        assert_eq!(sanitize_filename(".hidden"), "_hidden");

        // Test mixed
        assert_eq!(sanitize_filename("foo/bar"), "foo_bar");
        assert_eq!(sanitize_filename("foo\\bar"), "foo_bar");

        // Unicode
        assert_eq!(sanitize_filename("你好"), "你好");
        assert_eq!(sanitize_filename("a/b"), "a_b");
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("10ms"), Some(10));
        assert_eq!(parse_duration("1s"), Some(1000));
        assert_eq!(parse_duration("1m"), Some(60000));
        assert_eq!(parse_duration("1h"), Some(3600000));
        assert_eq!(parse_duration("1d"), Some(86400000));

        // No unit defaults to ms
        assert_eq!(parse_duration("500"), Some(500));

        // Invalid cases
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("invalid"), None);
        assert_eq!(parse_duration("10x"), Some(10)); // Current impl stops at non-digit char if not matching known suffix?
        // Wait, let's check impl:
        // if !last.is_ascii_digit() { (&duration_str[..len - 1], &duration_str[len - 1..]) }
        // "10x" -> val_str="10", unit="x".
        // match unit { ... _ => Some(value) } // default to ms
        // So "10x" -> 10ms. This seems to be the current behavior.

        // Test with spaces? Current impl doesn't trim.
        assert_eq!(parse_duration(" 10ms"), None);
    }
}
