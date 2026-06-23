use crate::context::Context;

pub const FIM_PREFIX: &str = "<|fim_prefix|>";
pub const FIM_SUFFIX: &str = "<|fim_suffix|>";
pub const FIM_MIDDLE: &str = "<|fim_middle|>";

/// Stop at end of line or any FIM/EOT control token.
pub const STOP_TOKENS: &[&str] = &[
    "\n",
    "<|endoftext|>",
    "<|fim_pad|>",
    "<|file_sep|>",
    FIM_PREFIX,
    FIM_SUFFIX,
    FIM_MIDDLE,
];

/// Render context as shell-comment lines a coder model understands.
/// Empty strings / None / empty vecs are skipped (Python truthiness parity).
fn context_comments(ctx: &Context) -> Vec<String> {
    let mut out = Vec::new();
    if !ctx.cwd.is_empty() {
        out.push(format!("# cwd: {}", ctx.cwd));
    }
    if !ctx.os.is_empty() {
        let shell = if ctx.shell.is_empty() { "zsh" } else { ctx.shell.as_str() };
        out.push(format!("# os: {} shell: {}", ctx.os, shell));
    }
    if let Some(branch) = ctx.git_branch.as_deref().filter(|b| !b.is_empty()) {
        out.push(format!("# git branch: {branch}"));
        if let Some(status) = ctx.git_status.as_deref().filter(|s| !s.is_empty()) {
            out.push(format!("# changed files: {}", status.replace('\n', ", ")));
        }
    }
    if !ctx.dir_entries.is_empty() {
        out.push(format!("# files: {}", ctx.dir_entries.join(", ")));
    }
    if !ctx.history.is_empty() {
        out.push(format!("# recent: {}", ctx.history.join("; ")));
    }
    out
}

/// Assemble a FIM prompt: context comments + the partial command, with an empty
/// suffix so the model completes to end of line. Ports ailib/prompt.py.
pub fn build_prompt(prefix: &str, ctx: &Context) -> String {
    let mut head = context_comments(ctx).join("\n");
    if !head.is_empty() {
        head.push('\n');
    }
    format!("{FIM_PREFIX}{head}{prefix}{FIM_SUFFIX}{FIM_MIDDLE}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;

    fn ctx() -> Context {
        Context {
            cwd: "/home/u/proj".into(),
            os: "Darwin".into(),
            shell: "zsh".into(),
            git_branch: Some("main".into()),
            git_status: Some("M file.py".into()),
            dir_entries: vec!["src/".into(), "README.md".into()],
            history: vec!["cd proj".into(), "ls".into()],
        }
    }

    #[test]
    fn includes_prefix_and_context() {
        let p = build_prompt("git che", &ctx());
        for needle in ["git che", "/home/u/proj", "main", "README.md", "cd proj"] {
            assert!(p.contains(needle), "missing {needle}");
        }
    }
    #[test]
    fn uses_fim_format() {
        let p = build_prompt("ls", &ctx());
        assert!(p.starts_with("<|fim_prefix|>"));
        assert!(p.ends_with("<|fim_middle|>"));
        assert!(p.contains("<|fim_suffix|>"));
    }
    #[test]
    fn partial_command_sits_just_before_suffix_marker() {
        let p = build_prompt("git che", &ctx());
        assert!(p.contains("git che<|fim_suffix|>"));
    }
    #[test]
    fn handles_empty_context() {
        let p = build_prompt("ls", &Context::default());
        assert!(p.contains("ls"));
        assert_eq!(p, "<|fim_prefix|>ls<|fim_suffix|><|fim_middle|>");
    }
}
