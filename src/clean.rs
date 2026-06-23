/// Turn raw FIM model output into the suffix that follows `prefix`.
/// First line only, reject stray ``` fences, trim only the tail (leading
/// whitespace is significant), strip an echoed prefix. Ports ailib/clean.py.
pub fn clean_suggestion(raw: &str, prefix: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    let first_line = raw.split('\n').next().unwrap_or("");
    if first_line.trim_start().starts_with("```") {
        return String::new();
    }
    let trimmed = first_line.trim_end(); // keep leading space; trim the tail
    let body = trimmed.strip_prefix(prefix).unwrap_or(trimmed);
    if body.trim().is_empty() || body == prefix {
        return String::new();
    }
    body.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_prefix_when_model_echoes_full_command() {
        assert_eq!(clean_suggestion("git checkout main", "git che"), "ckout main");
    }
    #[test]
    fn returns_suffix_directly_when_not_echoed() {
        assert_eq!(clean_suggestion("ckout main", "git che"), "ckout main");
    }
    #[test]
    fn takes_first_line_only() {
        assert_eq!(clean_suggestion("ckout main\nrm -rf /", "git che"), "ckout main");
    }
    #[test]
    fn preserves_significant_leading_space() {
        assert_eq!(clean_suggestion(" \"pattern\" .", "grep -r"), " \"pattern\" .");
    }
    #[test]
    fn rejects_code_fence() {
        assert_eq!(clean_suggestion("```bash", "git che"), "");
    }
    #[test]
    fn empty_when_equals_prefix() {
        assert_eq!(clean_suggestion("git che", "git che"), "");
    }
    #[test]
    fn empty_when_blank() {
        assert_eq!(clean_suggestion("   ", "git che"), "");
    }
}
