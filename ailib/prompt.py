SYSTEM = (
    "You are a zsh command-line autocompletion engine. "
    "Given a partial command and shell context, output ONLY the text that "
    "should continue the partial command. Do not repeat what the user typed. "
    "No explanation, no markdown, no code fences. Output a single line."
)


def build_prompt(prefix: str, ctx: dict) -> str:
    """Assemble the Ollama prompt from the partial command and context dict."""
    lines = [SYSTEM, ""]
    if ctx.get("cwd"):
        lines.append(f"cwd: {ctx['cwd']}")
    if ctx.get("os"):
        lines.append(f"os: {ctx['os']}  shell: {ctx.get('shell', 'zsh')}")
    if ctx.get("git_branch"):
        lines.append(f"git branch: {ctx['git_branch']}")
    if ctx.get("git_status"):
        lines.append(f"git status:\n{ctx['git_status']}")
    if ctx.get("dir_entries"):
        lines.append("files: " + ", ".join(ctx["dir_entries"]))
    if ctx.get("history"):
        lines.append("recent commands:\n" + "\n".join(ctx["history"]))
    lines += ["", f"partial command: {prefix}", "completion:"]
    return "\n".join(lines)
