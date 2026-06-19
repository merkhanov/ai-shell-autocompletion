import json
import urllib.request


def parse_response(body: str) -> str:
    """Extract the ``response`` field from an Ollama JSON body; ``""`` on error."""
    try:
        return str(json.loads(body).get("response", "") or "")
    except Exception:
        return ""


def query_ollama(prompt, *, url, model, max_tokens, keep_alive, timeout,
                 raw=False, stop=None):
    """POST ``prompt`` to ``{url}/api/generate`` and return the raw response text.

    ``raw=True`` bypasses the chat template (required for FIM prompts). ``stop``
    is a list of stop strings. Raises on network/HTTP errors — the caller is
    expected to catch and treat any failure as "no suggestion".
    """
    options = {"temperature": 0.2, "num_predict": max_tokens}
    if stop:
        options["stop"] = stop
    payload = json.dumps({
        "model": model,
        "prompt": prompt,
        "stream": False,
        "keep_alive": keep_alive,
        "raw": raw,
        "options": options,
    }).encode()
    req = urllib.request.Request(
        f"{url}/api/generate",
        data=payload,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return parse_response(resp.read().decode())
