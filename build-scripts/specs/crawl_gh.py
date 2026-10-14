"""Usage: python3 crawl_gh.py > gh-tree.json  (input for gen_gh.py)

Walks `gh <path> --help` for every command and prints the command tree as JSON."""

import json
import os
import re
import subprocess
import sys

# Sections listing subcommands we don't want: per-user aliases, installed extensions, help topics
SKIP_COMMAND_SECTIONS = {"ALIAS COMMANDS", "EXTENSION COMMANDS", "HELP TOPICS"}
FLAG_RE = re.compile(r"^\s+(?:(-\w), )?(--[\w-]+)(?: (\S+))?\s{2,}(.*)$")
ENV = {**os.environ, "NO_COLOR": "1", "GH_NO_UPDATE_NOTIFIER": "1", "GH_PROMPT_DISABLED": "1"}


def sections(text: str) -> dict[str, list[str]]:
    """Split help output into {HEADER: lines}; text before the first header goes under ''."""
    out: dict[str, list[str]] = {"": []}
    current = ""
    for line in text.splitlines():
        if re.fullmatch(r"[A-Z][A-Z ]+", line):
            current = line
            out[current] = []
        else:
            out[current].append(line)
    return out


def parse_flags(lines: list[str]) -> list[dict]:
    flags = []
    for line in lines:
        m = FLAG_RE.match(line)
        if m:
            short, long, arg, desc = m.groups()
            flags.append({"short": short, "long": long, "arg": arg, "description": desc.strip()})
    return flags


def crawl(path: list[str], description: str) -> dict:
    result = subprocess.run(["gh", *path, "--help"], capture_output=True, text=True, env=ENV)
    s = sections(result.stdout)
    usage = next((line.strip() for line in s.get("USAGE", []) if line.strip()), "")
    node = {
        "name": path[-1] if path else "gh",
        "description": description,
        "usage": usage,
        "aliases": [a.strip().split()[-1] for a in ",".join(s.get("ALIASES", [])).split(",") if a.strip()],
        "flags": parse_flags(s.get("FLAGS", [])),
        "inheritedFlags": parse_flags(s.get("INHERITED FLAGS", [])),
        "jsonFields": re.findall(r"[\w]+", " ".join(s.get("JSON FIELDS", []))),
        "longDescription": "\n".join(s[""]).strip(),
        "subcommands": [],
    }
    for header, lines in s.items():
        if not header.endswith("COMMANDS") or header in SKIP_COMMAND_SECTIONS:
            continue
        for line in lines:
            m = re.match(r"^\s+([\w-]+):\s+(.*)$", line)
            if m:
                print(" ".join([*path, m.group(1)]), file=sys.stderr)
                node["subcommands"].append(crawl([*path, m.group(1)], m.group(2).strip()))
    return node


print(json.dumps(crawl([], "GitHub's CLI tool"), indent=1))
