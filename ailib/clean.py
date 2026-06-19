def clean_suggestion(raw: str, prefix: str) -> str:
    """Turn raw model output into the suffix that follows ``prefix``.

    Returns ``""`` when the output is unusable (blank, fences only, or just
    an echo of what the user already typed).
    """
    if not raw or not raw.strip():
        return ""
    s = raw.strip()
    if s.startswith("```"):
        lines = s.splitlines()[1:]
        if lines and lines[-1].strip().startswith("```"):
            lines = lines[:-1]
        s = "\n".join(lines).strip()
    # first non-empty line only
    s = next((ln for ln in s.splitlines() if ln.strip()), "").strip()
    s = s.strip("`").strip()
    if not s:
        return ""
    suffix = s[len(prefix):] if s.startswith(prefix) else s
    if not suffix or suffix == prefix:
        return ""
    return suffix
