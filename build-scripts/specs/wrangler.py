"""Builds specs/wrangler.ts from wrangler's yargs help output. To regenerate after a wrangler release:

    npm install --prefix /tmp/wrangler wrangler@latest
    python3 build-scripts/specs/wrangler.py crawl /tmp/wrangler/node_modules/.bin/wrangler > /tmp/wrangler-tree.json
    python3 build-scripts/specs/wrangler.py gen /tmp/wrangler-tree.json > specs/wrangler.ts
    npx prettier --write specs/wrangler.ts --parser typescript

`crawl` walks `wrangler <path> --help` for every command; hidden and deprecated commands don't appear in
help, so they're left out. Upstream withfig's spec was written for the retired Rust wrangler v1 and had no
generators; its filepath templates carry over through FILE_ARGS and FOLDER_ARGS."""

import json
import os
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

ENV = {**os.environ, "WRANGLER_SEND_METRICS": "false", "NO_COLOR": "1", "FORCE_COLOR": "0"}

# Option and positional names whose values are files or directories
FILE_ARGS = {
    "--config",
    "--env-file",
    "--file",
    "--path",
    "--secrets-file",
    "--tsconfig",
    "--outfile",
    "--metafile",
    "script",
    "file",
    "path",
}
FOLDER_ARGS = {"--cwd", "--outdir", "--persist-to", "--site", "--assets", "--output-dir", "directory"}

# Trailing yargs annotations such as [string], [required], [default: []], [choices: "a", "b"]
ANNOTATION_RE = re.compile(
    r"\s*\[(string|boolean|number|array|count|required|deprecated(?:: [^\]]*)?"
    r"|default: (?:\[[^\]]*\]|[^\]])*|choices: [^\]]*|aliases: [^\]]*)\]\s*$"
)
OPTION_RE = re.compile(r"^((?:-{1,2}[\w.-]+(?:, )?)+)(?:\s{2,}(.*))?$")
POSITIONAL_RE = re.compile(r"^  ([\w.-]+)(?:, [\w.-]+)*\s{2,}(.*)$")
USAGE_ARG_RE = re.compile(r"([<\[])([\w.-]+?)(\.\.)?[>\]]")


# ---------- crawl ----------


def split_annotations(text):
    """'desc  [string] [choices: "a", "b"]' -> ('desc', {'string': True, 'choices': ['a', 'b']})"""
    annotations = {}
    while m := ANNOTATION_RE.search(text):
        key, _, value = m.group(1).partition(": ")
        annotations[key] = json.loads(f"[{value}]") if key == "choices" else (value or True)
        text = text[: m.start()]
    return text.strip(), annotations


def sections(text):
    """Split help output into {HEADER: lines}; the usage line and description go under ''."""
    out, current = {"": []}, ""
    for line in text.splitlines():
        if re.fullmatch(r"[A-Z][A-Z &]+", line):
            current = line
            out.setdefault(current, [])
        else:
            out[current].append(line)
    return out


def entries(lines, is_start):
    """Group lines into [first_line, description] entries; deeper-indented lines continue the description.
    Unindented text after the list is the command's epilogue and ends it."""
    items = []
    for line in lines:
        if not line.strip():
            continue
        if not line.startswith(" "):
            break
        if (m := is_start(line)) is not None:
            items.append(list(m))
        elif items:
            items[-1][1] += " " + line.strip()
    return items


def parse_options(lines):
    def start(line):
        stripped = line.lstrip()
        if len(line) - len(stripped) <= 6 and (m := OPTION_RE.match(stripped)):
            return m.group(1), m.group(2) or ""
        return None

    options = []
    for names, desc in entries(lines, start):
        desc, annotations = split_annotations(desc)
        options.append(
            {"names": [n.strip() for n in names.split(",") if n.strip()], "description": desc, **annotations}
        )
    return options


def parse_positionals(lines):
    out = {}
    for name, desc in entries(lines, lambda line: (m := POSITIONAL_RE.match(line)) and m.groups()):
        desc, annotations = split_annotations(desc)
        out[name] = {"name": name, "description": desc, **annotations}
    return out


def crawl(wrangler, path, description):
    text = subprocess.run(
        [wrangler, *path, "--help"], stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=60, env=ENV
    ).stdout
    s = sections(text)
    positionals = parse_positionals(s.get("POSITIONALS", []))
    usage = text.splitlines()[0] if text else ""  # e.g. "wrangler kv key put <key> [value]"
    node = {
        "name": path[-1] if path else "wrangler",
        "description": description,
        "args": [
            {
                **positionals.get(name, {"name": name, "description": ""}),
                "optional": bracket == "[",
                "variadic": bool(dots),
            }
            for bracket, name, dots in USAGE_ARG_RE.findall(usage)
        ],
        "options": parse_options(s.get("OPTIONS", [])),
        "subcommands": [],
    }
    if not path:
        node["globalFlags"] = parse_options(s.get("GLOBAL FLAGS", []))

    # Subcommands are listed as "  wrangler <path> <name> [args]  description" under COMMANDS or a category
    children = []
    prefix = "  " + " ".join(["wrangler", *path]) + " "
    for header, lines in s.items():
        if header in ("OPTIONS", "GLOBAL FLAGS", "POSITIONALS", "EXAMPLES"):
            continue
        for line in lines:
            if line.startswith(prefix):
                rest = line[len(prefix) :]
                desc, annotations = split_annotations(rest.split("  ", 1)[1] if "  " in rest else "")
                children.append((rest.split()[0], desc, annotations))
    with ThreadPoolExecutor(8) as pool:
        results = pool.map(lambda c: crawl(wrangler, [*path, c[0]], c[1]), children)
        for (_name, _desc, annotations), child in zip(children, results):
            child["aliases"] = [a.strip() for a in str(annotations.get("aliases", "")).split(",") if a.strip()]
            child["deprecated"] = "deprecated" in annotations
            node["subcommands"].append(child)
    print(" ".join(path) or "wrangler", file=sys.stderr)
    return node


# ---------- gen ----------


def js(value):
    return json.dumps(value, ensure_ascii=False)


def clean(description):
    """Match the fig linter's description style: capitalised, no trailing dot."""
    # Fix a typo in wrangler's help ("sepe" + "cific") that the pre-commit `typos` check rejects
    description = re.sub(r"\bsepe(?=cific)", "spe", description.strip())
    description = description[:-3] + "…" if description.endswith("...") else description.removesuffix(".")
    return description[:1].upper() + description[1:]


def split_icon(description):
    """Top-level commands start with an emoji; use it as the suggestion icon."""
    first, _, rest = description.partition(" ")
    if first and not first.isascii() and not any(c.isalnum() for c in first):
        return first, rest
    return None, description


def obj(parts):
    return "{ " + ", ".join(parts) + " }"


def emit_arg(name, arg, template_key):
    parts = [f"name: {js(name)}"]
    if arg.get("description"):
        parts.append(f"description: {js(clean(arg['description']))}")
    if arg.get("choices"):
        parts.append(f"suggestions: {js([str(c) for c in arg['choices']])}")
    if template_key in FILE_ARGS:
        parts.append('template: "filepaths"')
    elif template_key in FOLDER_ARGS:
        parts.append('template: "folders"')
    if arg.get("optional"):
        parts.append("isOptional: true")
    if arg.get("variadic"):
        parts.append("isVariadic: true")
    default = arg.get("default")
    if isinstance(default, str) and default not in ("", "[]", "{}"):
        parts.append(f"default: {js(default.strip(chr(34)))}")
    return obj(parts)


def emit_option(option, persistent=False):
    names = option["names"]
    long = next((n for n in names if n.startswith("--")), names[0])
    parts = [f"name: {js(names if len(names) > 1 else names[0])}"]
    if option["description"]:
        parts.append(f"description: {js(clean(option['description']))}")
    # Booleans and untyped options are flags
    if any(k in option for k in ("string", "number", "array", "choices")):
        arg = {k: option.get(k) for k in ("choices", "default")}
        # e.g. `basin pipelines sinks create --path` is a prefix inside an R2 bucket, not a local path
        template_key = None if "bucket" in option["description"] else long
        parts.append(f"args: {emit_arg(long.lstrip('-'), arg, template_key)}")
    if option.get("array"):
        parts.append("isRepeatable: true")
    if option.get("required"):
        parts.append("isRequired: true")
    if option.get("deprecated"):
        parts.append("deprecated: true")
    if persistent:
        parts.append("isPersistent: true")
    return obj(parts)


def emit_command(command):
    names = [command["name"], *command.get("aliases", [])]
    parts = [f"name: {js(names if len(names) > 1 else names[0])}"]
    icon, desc = split_icon(command["description"])
    if desc:
        parts.append(f"description: {js(clean(desc))}")
    if icon:
        parts.append(f"icon: {js(icon)}")
    if command.get("deprecated"):
        parts.append("deprecated: true")
    if command["args"]:
        args = [emit_arg(a["name"], a, a["name"]) for a in command["args"]]
        parts.append(f"args: {args[0] if len(args) == 1 else '[' + ', '.join(args) + ']'}")
    # Global flags are persistent options on the root
    options = [emit_option(o) for o in command["options"]]
    options += [emit_option(o, persistent=True) for o in command.get("globalFlags", [])]
    if options:
        parts.append(f"options: [{', '.join(options)}]")
    if command["subcommands"]:
        parts.append(f"subcommands: [{', '.join(emit_command(c) for c in command['subcommands'])}]")
    return obj(parts)


def main():
    mode, source = sys.argv[1], sys.argv[2]
    if mode == "crawl":
        version = subprocess.run(
            [source, "--version"], stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=60, env=ENV
        ).stdout.split()[-1]
        tree = crawl(source, [], "Cloudflare Workers CLI")
        print(json.dumps({"version": version, **tree}, indent=1, ensure_ascii=False))
    elif mode == "gen":
        tree = json.load(open(source))
        print(f"// Generated by build-scripts/specs/wrangler.py from wrangler {tree['version']}; don't edit by hand")
        print(f"const completionSpec: Fig.Spec = {emit_command(tree)};\n\nexport default completionSpec;")
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
