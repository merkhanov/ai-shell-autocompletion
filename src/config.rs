fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn env_parse<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

#[derive(Debug, Clone)]
pub struct Config {
    pub min_chars: usize,
    pub history_lines: usize,
    pub ollama_url: String,
    pub model: String,
    pub max_tokens: u32,
    pub keep_alive: String,
    pub timeout: f64,
    pub debug: bool,
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            min_chars: env_parse("AI_AC_MIN_CHARS", 3),
            history_lines: env_parse("AI_AC_HISTORY_LINES", 30),
            ollama_url: env_or("AI_AC_OLLAMA_URL", "http://localhost:11434"),
            model: env_or("AI_AC_MODEL", "qwen2.5-coder:3b"),
            max_tokens: env_parse("AI_AC_MAX_TOKENS", 64),
            keep_alive: env_or("AI_AC_KEEP_ALIVE", "30m"),
            timeout: env_parse("AI_AC_TIMEOUT", 5.0),
            debug: env_or("AI_AC_DEBUG", "0") == "1",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_parse_falls_back_when_unset() {
        // A name that is never set, so this is deterministic under parallel tests.
        assert_eq!(env_parse::<usize>("AI_AC_NEVER_SET_PARSE_Q", 3), 3);
    }
    #[test]
    fn env_or_falls_back_when_unset() {
        assert_eq!(env_or("AI_AC_NEVER_SET_OR_Q", "def"), "def");
    }
    #[test]
    fn env_parse_falls_back_on_garbage() {
        std::env::set_var("AI_AC_GARBAGE_INT_Q", "not-a-number");
        assert_eq!(env_parse::<u32>("AI_AC_GARBAGE_INT_Q", 64), 64);
        std::env::remove_var("AI_AC_GARBAGE_INT_Q");
    }
}
