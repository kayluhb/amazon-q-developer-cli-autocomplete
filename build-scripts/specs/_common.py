"""Helpers shared by the spec generators in this directory."""

import json


class Raw(str):
    """Emitted as-is instead of as a string literal."""


def emit(value) -> str:
    """Render a value as a TypeScript literal on one line (prettier formats the result)."""
    if isinstance(value, Raw):
        return str(value)
    if isinstance(value, dict):
        return "{" + ", ".join(f"{k}: {emit(v)}" for k, v in value.items()) + "}"
    if isinstance(value, list):
        return "[" + ", ".join(emit(v) for v in value) + "]"
    return json.dumps(value, ensure_ascii=False)


def tidy(description: str) -> str:
    """Match the fig linter's description style: one line, capitalised, no trailing dots."""
    description = " ".join(description.split()).rstrip(".")
    return description[:1].upper() + description[1:]
