import os
import platform
import subprocess


def _truncate(items, n):
    return list(items)[:n]


def _run(cmd, timeout=0.5):
    """Run a command best-effort; return stripped stdout or ``""`` on any error."""
    try:
        out = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        return out.stdout.strip()
    except Exception:
        return ""


def gather_context(*, history_lines, max_dir_entries, history=None):
    """Collect best-effort shell context for the prompt. Never raises."""
    cwd = os.getcwd()
    ctx = {
        "cwd": cwd,
        "os": platform.system(),
        "shell": os.environ.get("SHELL", "zsh"),
    }
    branch = _run(["git", "branch", "--show-current"])
    if branch:
        ctx["git_branch"] = branch
        status = _run(["git", "status", "--porcelain"])
        if status:
            ctx["git_status"] = "\n".join(_truncate(status.splitlines(), 10))
    try:
        entries = sorted(os.listdir(cwd))
        ctx["dir_entries"] = _truncate(
            [e + "/" if os.path.isdir(os.path.join(cwd, e)) else e
             for e in entries if not e.startswith(".")],
            max_dir_entries,
        )
    except Exception:
        pass
    if history:
        ctx["history"] = _truncate(history, history_lines)
    return ctx
