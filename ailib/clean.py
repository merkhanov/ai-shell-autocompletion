def clean_suggestion(raw: str, prefix: str) -> str:
    """Turn raw FIM model output into the suffix that follows ``prefix``.

    FIM output is already a continuation (no echo, no fences), so cleaning is
    light: take the first line, drop stray markdown, and preserve leading
    whitespace because it can be significant (e.g. the space in ``grep -r`` ->
    `` "pattern"``). Returns ``""`` when the output is unusable.
    """
    if not raw:
        return ""
    s = raw.split("\n", 1)[0]            # first line only
    if s.strip().startswith("```"):      # reject stray markdown fences
        return ""
    s = s.rstrip()                       # keep leading space; trim the tail
    if s.startswith(prefix):             # defensive: strip an echoed prefix
        s = s[len(prefix):]
    if not s.strip() or s == prefix:
        return ""
    return s
