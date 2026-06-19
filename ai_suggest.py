#!/usr/bin/env python3
"""Entry point: print the AI completion suffix for a partial zsh command.

Usage:   python3 ai_suggest.py -- "<partial command>"
Config:  read from AI_AC_* environment variables (see README).
Output:  the suffix to append after the partial command, no trailing newline.
         Prints nothing (and exits 0) on any failure.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from ailib.context import gather_context
from ailib.prompt import build_prompt, STOP_TOKENS
from ailib.ollama import query_ollama
from ailib.clean import clean_suggestion


def _env(name, default):
    return os.environ.get(name, default)


def _log(msg):
    if _env("AI_AC_DEBUG", "0") == "1":
        try:
            with open("/tmp/ai-ac.log", "a") as f:
                f.write(msg + "\n")
        except Exception:
            pass


def main():
    args = sys.argv[1:]
    if args and args[0] == "--":
        args = args[1:]
    prefix = args[0] if args else ""
    if len(prefix) < int(_env("AI_AC_MIN_CHARS", "3")):
        return

    hist = _env("AI_AC_HISTORY", "")
    history = [h for h in hist.splitlines() if h.strip()] or None
    ctx = gather_context(
        history_lines=int(_env("AI_AC_HISTORY_LINES", "30")),
        max_dir_entries=50,
        history=history,
    )
    prompt = build_prompt(prefix, ctx)
    try:
        raw = query_ollama(
            prompt,
            url=_env("AI_AC_OLLAMA_URL", "http://localhost:11434"),
            model=_env("AI_AC_MODEL", "qwen2.5-coder:3b"),
            max_tokens=int(_env("AI_AC_MAX_TOKENS", "64")),
            keep_alive=_env("AI_AC_KEEP_ALIVE", "30m"),
            timeout=float(_env("AI_AC_TIMEOUT", "5")),
            raw=True,
            stop=STOP_TOKENS,
        )
    except Exception as e:
        _log(f"query failed: {e}")
        return

    suffix = clean_suggestion(raw, prefix)
    _log(f"prefix={prefix!r} suffix={suffix!r}")
    sys.stdout.write(suffix)


if __name__ == "__main__":
    main()
