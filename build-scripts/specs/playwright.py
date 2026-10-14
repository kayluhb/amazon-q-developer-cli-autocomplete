"""Builds specs/playwright.ts from Playwright's help output. To regenerate after a release:

    npm install --prefix /tmp/pw --ignore-scripts @playwright/test@latest
    python3 build-scripts/specs/playwright.py /tmp/pw/node_modules > specs/playwright.ts
    npx prettier --write specs/playwright.ts --parser typescript

The command tree comes from `playwright [<command>...] --help` (commander). `playwright cli`
prints its own help from playwright-core's help.json, so its commands are read from that file,
which also says which flags take a value. Browser names and a few option values aren't in the
help text and are hand-written below."""

import json
import os
import re
import subprocess
import sys

from _common import Raw, emit, tidy

NODE_MODULES = sys.argv[1]
PLAYWRIGHT = os.path.join(NODE_MODULES, ".bin", "playwright")
CLI_HELP = os.path.join(NODE_MODULES, "playwright-core", "lib", "tools", "cli-client", "help.json")

# What `playwright install` accepts: registry executables whose installType isn't "none", plus
# chromiumAliases (see suggestedBrowsersToInstall in playwright-core). chrome-dev, chrome-canary and
# msedge-canary are valid --channel values but can't be installed. webkit-wsl is Windows-only.
PRELUDE = """const browserSuggestions: Fig.Suggestion[] = [
  { name: "chromium", displayName: "Chromium" },
  { name: "chromium-headless-shell", displayName: "Chromium Headless Shell" },
  { name: "chrome", displayName: "Chrome" },
  { name: "chrome-beta", displayName: "Chrome Beta" },
  { name: "chrome-for-testing", displayName: "Chrome for Testing" },
  { name: "msedge", displayName: "Microsoft Edge" },
  { name: "msedge-beta", displayName: "Microsoft Edge Beta" },
  { name: "msedge-dev", displayName: "Microsoft Edge Dev" },
  { name: "firefox", displayName: "Firefox" },
  { name: "webkit", displayName: "WebKit" },
  { name: "ffmpeg", displayName: "FFmpeg" },
  { name: "android", displayName: "Android" },
];

const helpOption: Fig.Option = {
  name: ["--help", "-h"],
  description: "Display help for command",
  priority: 1,
};
"""

REPORTERS = ["list", "line", "dot", "json", "junit", "null", "github", "html", "blob", "perfetto", "coverage"]
CHANNELS = [
    "chrome",
    "chrome-beta",
    "chrome-dev",
    "chrome-canary",
    "msedge",
    "msedge-beta",
    "msedge-dev",
    "msedge-canary",
]


# Completions the help text can't tell us about, keyed by "<command path> <--flag or positional name>"
OVERRIDES = {
    "install browser": {"suggestions": Raw("browserSuggestions")},
    "install-deps browser": {"suggestions": Raw("browserSuggestions")},
    "cli install-browser browser": {"suggestions": Raw("browserSuggestions")},
    "test test-filter": {"template": ["filepaths", "folders"]},
    "test --reporter": {"suggestions": REPORTERS},
    "test --add-reporter": {"suggestions": REPORTERS},
    "merge-reports --reporter": {"suggestions": REPORTERS},
    "show-report report": {"template": ["folders", "filepaths"]},
    "show-trace trace": {"template": "filepaths"},
    "trace open trace": {"template": "filepaths"},
    "trace snapshot --phase": {"suggestions": ["before", "action", "after"]},
    "mcp --console-level": {"suggestions": ["error", "warning", "info", "debug"]},
    "mcp --file-paths": {"suggestions": ["relative", "absolute"]},
    "pdf --paper-format": {
        "suggestions": ["Letter", "Legal", "Tabloid", "Ledger", "A0", "A1", "A2", "A3", "A4", "A5", "A6"]
    },
}
# Options shared by open, codegen, cr, ff, wk, screenshot and pdf
SHARED_OVERRIDES = {
    "--channel": {"suggestions": CHANNELS},
    "--color-scheme": {"suggestions": ["light", "dark"]},
}


def choices(description: str) -> list[str] | None:
    """Values listed in a description, e.g. `(choices: "a", "b")` or `one of a, b or c`."""
    for pattern in (
        r"\(choices: ([^)]*?)(?:, preset: [^)]*)?(?:, default: [^)]*)?\)",
        r"one of:? ([^().]*)",
        r"possible values: ([^().]*?)(?:\.|$| Default)",
        r"[Cc]an be ((?:\"[\w-]+\"(?:, | or )?)+)",
    ):
        if m := re.search(pattern, description):
            values = [v.strip().strip('"') for v in re.split(r",|\bor\b", m.group(1))]
            values = [v for v in values if v]
            if values and all(re.fullmatch(r"[\w.-]+", v) for v in values):
                return values
    return None


def strip_choices(description: str) -> str:
    return re.sub(r"\s*\(choices: [^)]*\)", "", description)


def completion(name: str, key: str, description: str) -> dict:
    if override := OVERRIDES.get(key):
        return dict(override)
    if values := choices(description):
        return {"suggestions": values}
    if (lead := description.lower().split(".")[0]).startswith("path to"):
        return {"template": "folders" if "directory" in lead else "filepaths"}
    words = set(re.findall(r"[a-z]+", name.lower()))
    if words & {"dir", "directory"}:
        return {"template": "folders"}
    if words & {"file", "filename", "path"}:
        return {"template": "filepaths"}
    return {}


def arg(token: str, key: str, description: str = "") -> dict:
    """`<name>`, `[name]` or `<name...>` from a usage or flags column."""
    name = token.strip("<>[]").removesuffix("...")
    out: dict = {"name": name}
    if description:
        out["description"] = tidy(description)
    if token.startswith("["):
        out["isOptional"] = True
    if token.endswith("...>") or token.endswith("...]"):
        out["isVariadic"] = True
    out.update(completion(name, key, description))
    return out


def sections(text: str) -> dict[str, list[str]]:
    """Help text split into its `Heading:` blocks. Indented lines belong to the block above."""
    out: dict[str, list[str]] = {"": []}
    current = ""
    for line in text.splitlines():
        if re.match(r"^\S.*:$", line):
            current = line[:-1]
            out[current] = []
        else:
            out[current].append(line)
    return out


def entries(lines: list[str]) -> list[tuple[str, str]]:
    """Two-column `  term   description` entries, joining wrapped description lines."""
    out: list[tuple[str, str]] = []
    for line in lines:
        if not line.strip():
            if out:  # the block ends at its first blank line; examples may follow
                break
            continue
        if m := re.match(r"^  (\S.*?)(?:\s{2,}(.*))?$", line):
            out.append((m.group(1), m.group(2) or ""))
        elif out:
            out[-1] = (out[-1][0], f"{out[-1][1]} {line.strip()}")
    return out


def option(term: str, description: str, path: list[str]) -> dict | Raw:
    # e.g. `-b, --browser <browserType>`, `--ui`, `--user-agent <ua string>`
    flags, value = re.match(r"^(.*?)(?: ([<\[].*))?$", term).groups()
    names = sorted((n.strip() for n in flags.split(",")), key=lambda n: not n.startswith("--"))
    if names == ["--help", "-h"]:
        return Raw("helpOption")
    out: dict = {"name": names[0] if len(names) == 1 else names, "description": tidy(strip_choices(description))}
    if value:
        key = " ".join([*path, names[0]])
        a = arg(value, key, description)
        del a["description"]  # the option already shows it
        if path and path[0] in {"open", "codegen", "cr", "ff", "wk", "screenshot", "pdf"} and not OVERRIDES.get(key):
            a.update(SHARED_OVERRIDES.get(names[0], {}))
        if " " in a["name"]:  # e.g. `<ua string>`
            a["name"] = a["name"].replace(" ", "-")
        out["args"] = a
    return out


def help_text(path: list[str]) -> str:
    result = subprocess.run(
        [PLAYWRIGHT, *path, "--help"], stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=60
    )
    return result.stdout


def command(path: list[str], description: str) -> dict:
    text = help_text(path)
    blocks = sections(text)
    usage = re.search(r"^Usage: npx playwright (.*)$", text, re.M).group(1).split()[len(path) :]
    out: dict = {"name": path[-1] if path else "playwright", "description": tidy(description)}
    commands = [(t.split()[0], d) for t, d in entries(blocks.get("Commands", [])) if not t.startswith("help ")]
    if commands:
        # `playwright cli` prints its own help, not commander's
        out["subcommands"] = [cli(d) if [*path, name] == ["cli"] else command([*path, name], d) for name, d in commands]
    if options := entries(blocks.get("Options", [])):
        out["options"] = [option(t, d, path) for t, d in options]
    arg_help = {
        k.split()[1].strip("[]<>:").removesuffix("..."): " ".join(v).strip()
        for k, v in blocks.items()
        if k.startswith("Arguments ")
    }
    args = []
    for tok in usage:
        if tok in {"[options]", "[command]"}:
            continue
        name = tok.strip("<>[]").removesuffix("...")
        args.append(arg(tok, " ".join([*path, name]), arg_help.get(name, "")))
    if args:
        out["args"] = args[0] if len(args) == 1 else args
    return out


def cli_command(name: str, info: dict) -> dict:
    blocks = sections(info["help"])
    lines = [line for line in blocks[""] if line.strip()]
    usage, description = lines[0].split()[2:], lines[1] if len(lines) > 1 else ""
    out: dict = {"name": name, "description": tidy(description)}
    arg_help = {t.strip("<>[]").removesuffix("..."): d for t, d in entries(blocks.get("Arguments", []))}
    args = [
        arg(t, f"cli {name} {t.strip('<>[]')}", arg_help.get(t.strip("<>[]").removesuffix("..."), "")) for t in usage
    ]
    opt_help = dict(entries(blocks.get("Options", [])))
    options = []
    for flag, kind in info["flags"].items():
        desc = opt_help.get(f"--{flag}", "")
        o: dict = {"name": f"--{flag}", "description": tidy(desc)}
        if kind != "boolean":
            o["args"] = {"name": flag, **completion(flag, f"cli {name} --{flag}", desc)}
        options.append(o)
    if options:
        out["options"] = options
    if args:
        out["args"] = args[0] if len(args) == 1 else args
    return out


def cli(description: str) -> dict:
    data = json.load(open(CLI_HELP))
    blocks = sections(data["global"])
    listed = [
        t.split()[0]
        for heading, lines in blocks.items()
        if heading not in {"", "Global options"}
        for t, _ in entries(lines)
    ]
    globals_ = []
    for term, desc in entries(blocks["Global options"]):
        flag, _, value = term.partition(" ")
        o: dict = {"name": flag, "description": tidy(desc), "isPersistent": True}
        if value:
            o["args"] = arg(value, "")
        globals_.append(o)
    globals_.append(
        {
            "name": "-s",
            "description": "Browser session to use",
            "requiresSeparator": True,
            "isPersistent": True,
            "args": {"name": "session"},
        }
    )
    return {
        "name": "cli",
        "description": tidy(description),
        "subcommands": [cli_command(n, data["commands"][n]) for n in listed if n in data["commands"]],
        "options": globals_,
    }


spec = command([], "Playwright end-to-end testing and browser automation")

version = subprocess.run([PLAYWRIGHT, "--version"], stdin=subprocess.DEVNULL, capture_output=True, text=True).stdout
print(f"// Generated by build-scripts/specs/playwright.py from Playwright {version.split()[-1]}; don't edit by hand")
print(PRELUDE)
print(f"const completionSpec: Fig.Spec = {emit(spec)};\n")
print("export default completionSpec;")
