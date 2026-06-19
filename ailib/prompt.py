"""Build a fill-in-middle (FIM) completion prompt for qwen2.5-coder.

Coder models do clean continuation via FIM, not instruction-following: an
instruct prompt makes small models echo the input or wrap it in markdown. FIM
asks the model to fill the gap between a prefix and a (here empty) suffix, which
is exactly "continue this command to the end of the line."
"""

FIM_PREFIX = "<|fim_prefix|>"
FIM_SUFFIX = "<|fim_suffix|>"
FIM_MIDDLE = "<|fim_middle|>"

# Stop generation at end of line or any FIM/EOT control token.
STOP_TOKENS = [
    "\n", "<|endoftext|>", "<|fim_pad|>", "<|file_sep|>",
    FIM_PREFIX, FIM_SUFFIX, FIM_MIDDLE,
]


def _context_comments(ctx: dict) -> list:
    """Render context as shell comment lines a coder model understands."""
    out = []
    if ctx.get("cwd"):
        out.append(f"# cwd: {ctx['cwd']}")
    if ctx.get("os"):
        out.append(f"# os: {ctx['os']} shell: {ctx.get('shell', 'zsh')}")
    if ctx.get("git_branch"):
        out.append(f"# git branch: {ctx['git_branch']}")
        if ctx.get("git_status"):
            out.append("# changed files: " + ctx["git_status"].replace("\n", ", "))
    if ctx.get("dir_entries"):
        out.append("# files: " + ", ".join(ctx["dir_entries"]))
    if ctx.get("history"):
        out.append("# recent: " + "; ".join(ctx["history"]))
    return out


def build_prompt(prefix: str, ctx: dict) -> str:
    """Assemble a FIM prompt: context comments + the partial command, with an
    empty suffix so the model completes to the end of the line."""
    head = "\n".join(_context_comments(ctx))
    if head:
        head += "\n"
    return f"{FIM_PREFIX}{head}{prefix}{FIM_SUFFIX}{FIM_MIDDLE}"
