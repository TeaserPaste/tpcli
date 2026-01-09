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
    let s = name.replace(
        |c: char| !c.is_alphanumeric() && c != '.' && c != '-' && c != '_',
        "_",
    );
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
