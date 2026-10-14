"""Builds specs/pnpm.ts from pnpm's help output. To regenerate after a pnpm release:

    mkdir -p /tmp/pnpm && npm install --prefix /tmp/pnpm --no-save --ignore-scripts pnpm@latest
    git -C build/autocomplete-specs show HEAD:src/pnpm.ts > /tmp/pnpm.upstream.ts
    python3 build-scripts/specs/gen_pnpm.py /tmp/pnpm/node_modules/@pnpm/exe.darwin-arm64/pnpm \\
        /tmp/pnpm.upstream.ts > specs/pnpm.ts
    npx prettier --write specs/pnpm.ts --parser typescript

The first argument is pnpm's native binary (pnpm 11+ ships one per platform under @pnpm/exe.*).
`pnpm help <command>` is walked for every command (from an empty temp directory, so no project
script can shadow a built-in). Options every command accepts become persistent root options.
Dynamic suggestions reuse upstream's hand-written generators, which are copied over verbatim."""

import os
import re
import subprocess
import sys
import tempfile

from _common import Raw, emit, tidy

PNPM = sys.argv[1]
upstream = open(sys.argv[2]).read()

# Upstream's imports and generators, everything before its option lists
PRELUDE = upstream[: upstream.index("const FILTER_OPTION")].replace("// GENERATORS\n\n", "")
# Upstream's spec-level generateSpec, which offers installed node CLIs (`pnpm tsc`, ...) as subcommands
GENERATE_SPEC = re.search(r"\n  generateSpec: (async .*?\n  }),\n  subcommands,", upstream, re.S).group(1)

# Subcommands whose own help lists nothing useful: `help` repeats the whole tree
SKIP_SUBCOMMANDS = {"help"}

HEADER_RE = re.compile(r"^([A-Z][A-Za-z ]*):$")
# e.g. "  -F, --filter <FILTER>", "      --color[=<COLOR>]", "      --fix [<METHOD>]", "      --cpu <CPU>..."
OPTION_RE = re.compile(
    r"^ {2,6}(-[\w?](?=[ ,]|$)|--[\w.-]+)((?:, --?[\w.-]+)*)"
    r"(\[=<[^>]+>\]|=<[^>]+>| \[<[^>]+>\]| <[^>]+>(?:\.\.\.)?)?(?: {2,}(.*))?$"
)
COMMAND_RE = re.compile(r"^ {2}([\w-]+)(?: {2,}(.*))?$")
ARGUMENT_RE = re.compile(r"^ {2}([<\[][^ ]+)(?: {2,}(.*))?$")
CWD = tempfile.mkdtemp()


def run_help(path: list[str]) -> str:
    result = subprocess.run(
        [PNPM, "help", *path],
        capture_output=True,
        text=True,
        cwd=CWD,
        stdin=subprocess.DEVNULL,
        timeout=30,
        env={**os.environ, "NO_COLOR": "1"},
    )
    return result.stdout


def entries(lines: list[str], head_re: re.Pattern) -> list[tuple[re.Match, list[str]]]:
    """Group a section's lines into (header match, indented description lines)."""
    out = []
    for line in lines:
        m = head_re.match(line)
        if m and not line.startswith(" " * 8):
            out.append((m, []))
        elif out:
            out[-1][1].append(line.strip())
    return out


def describe(inline: str | None, body: list[str]) -> dict:
    """Split a clap entry's text into description, aliases, defaults and possible values."""
    text = "\n".join([inline or "", *body]).strip()
    info: dict = {"aliases": [], "values": []}
    for kind, names in re.findall(r"\[(alias(?:es)?): ([^\]]+)\]", text):
        info["aliases"] += [n.strip() for n in names.split(",")]
    if m := re.search(r"\[possible values: ([^\]]+)\]", text):
        info["values"] = [{"name": v.strip()} for v in m.group(1).split(",")]
    if m := re.search(r"^Possible values:\n((?:- .*\n?)+)", text, re.M):
        for v in re.finditer(r"^- ([^:\n]+)(?::\s*(.*))?$", m.group(1), re.M):
            info["values"].append({"name": v.group(1).strip(), "description": (v.group(2) or "").strip()})
    if m := re.search(r"\[default: ([^\]]+)\]", text):
        info["default"] = m.group(1).strip()
    text = re.sub(r"\s*\[(alias(?:es)?|default|possible values): [^\]]+\]", "", text)
    text = re.sub(r"\n?Possible values:\n(- .*\n?)+", "", text)
    info["description"] = text.split("\n\n")[0].replace("\n", " ").strip()
    return info


def parse(path: list[str], description: str, aliases: list[str]) -> dict:
    text = run_help(path)
    sections: dict[str, list[str]] = {"": []}
    current = ""
    for line in text.splitlines():
        if m := HEADER_RE.match(line):
            current = m.group(1)
            sections[current] = []
        elif line.startswith("Usage: "):
            current = "Usage"
            sections[current] = [line.removeprefix("Usage: ")]
        else:
            sections[current].append(line)
    node = {
        "name": path[-1] if path else "pnpm",
        "aliases": aliases,
        "description": description,
        "usage": (sections.get("Usage") or [""])[0],
        "options": [],
        "args": [],
        "subcommands": [],
    }
    for m, body in entries(sections.get("Options", []), OPTION_RE):
        names = [m.group(1), *[n.strip() for n in (m.group(2) or "").split(",") if n.strip()]]
        info = describe(m.group(4), body)
        arg = m.group(3)
        node["options"].append(
            {
                "names": names + info["aliases"],
                "arg": arg
                and {
                    "name": re.search(r"<([^>]+)>", arg).group(1),
                    "optional": "[" in arg,
                    "equals": "=" in arg,
                    "variadic": arg.endswith("..."),
                },
                **{k: info[k] for k in ("description", "values", "default") if k in info},
            }
        )
    for m, body in entries(sections.get("Arguments", []), ARGUMENT_RE):
        info = describe(m.group(2), body)
        token = m.group(1)
        node["args"].append(
            {
                "name": re.sub(r"[<>\[\].]", "", token),
                "optional": token.startswith("["),
                "variadic": token.endswith("..."),
                **{k: info[k] for k in ("description", "values", "default") if k in info},
            }
        )
    for m, body in entries(sections.get("Commands", []), COMMAND_RE):
        name = m.group(1)
        if name in SKIP_SUBCOMMANDS:
            continue
        info = describe(m.group(2), body)
        print(" ".join([*path, name]), file=sys.stderr)
        node["subcommands"].append(parse([*path, name], info["description"], info["aliases"]))
    return node


def words(*values: str) -> list[dict]:
    return [{"name": v} for v in values]


FUZZY_DEPENDENCIES = {"filterStrategy": "fuzzy", "generators": Raw("dependenciesGenerator")}
REGISTRY_PACKAGES = {"generators": Raw("npmSearchGenerator"), "debounce": True}
# Completions the help text can't express, keyed by "<command path> <ARG NAME or --flag>"
OVERRIDES = {
    "add PACKAGE_NAMES": REGISTRY_PACKAGES,
    "dlx COMMAND": REGISTRY_PACKAGES,
    "view PARAMS": REGISTRY_PACKAGES,
    "docs PACKAGE": REGISTRY_PACKAGES,
    "repo PACKAGES": REGISTRY_PACKAGES,
    "bugs PACKAGES": REGISTRY_PACKAGES,
    "update PACKAGES": FUZZY_DEPENDENCIES,
    "remove PACKAGE_NAMES": FUZZY_DEPENDENCIES,
    "unlink PACKAGE_NAMES": FUZZY_DEPENDENCIES,
    "rebuild PACKAGES": FUZZY_DEPENDENCIES,
    "rb PACKAGES": FUZZY_DEPENDENCIES,
    "why PACKAGES": FUZZY_DEPENDENCIES,
    "list PACKAGES": FUZZY_DEPENDENCIES,
    "ll PACKAGES": FUZZY_DEPENDENCIES,
    "outdated PACKAGES": FUZZY_DEPENDENCIES,
    "exec COMMAND": FUZZY_DEPENDENCIES,
    "patch PACKAGE_NAME": {"generators": Raw("generatorInstalledPackages")},
    "run SCRIPT": {"filterStrategy": "fuzzy", "generators": Raw("npmScriptsGenerator")},
    "publish PACKAGE": {"template": ["filepaths", "folders"]},
    "link PACKAGE_PATHS": {"template": "folders"},
    "patch-commit PATCH_DIR": {"template": "folders"},
    "deploy DIR": {"template": "folders"},
    # Subcommand words named in each command's argument description
    "audit PARAMS": {"suggestions": words("signatures")},
    "peers PARAMS": {"suggestions": words("check")},
    "change PARAMS": {"suggestions": words("status", "check")},
    "shim PARAMS": {"suggestions": words("add", "rm", "ls")},
    "env PARAMS": {"suggestions": words("use", "list", "remove")},
    "runtime PARAMS": {"suggestions": words("set")},
    "version PARAMS": {
        "suggestions": words("major", "minor", "patch", "premajor", "preminor", "prepatch", "prerelease", "from-git")
    },
    "audit --fix": {"suggestions": words("override", "update")},
    "change --bump": {"suggestions": words("none", "patch", "minor", "major")},
    # Not listed by the help text. The completion shells are unverified: they are the shells that
    # @pnpm/tabtab (pnpm's completion backend) is believed to support.
    "completion SHELL": {"suggestions": words("bash", "fish", "pwsh", "zsh")},
    "publish --publish-branch": {"generators": Raw("searchBranches")},
    "stage --publish-branch": {"generators": Raw("searchBranches")},
    " --filter": {"template": "filepaths"},
    " --npmrc-auth-file": {"template": "filepaths"},
}


def suggestions(values: list[dict]) -> list:
    return [
        {"name": v["name"], "description": tidy(v["description"])} if v.get("description") else v["name"]
        for v in values
    ]


def arg_spec(name: str, description: str, info: dict, key: str) -> dict:
    out: dict = {"name": name.lower().replace("_", "-")}
    if description:
        out["description"] = tidy(description)
    if info.get("optional"):
        out["isOptional"] = True
    if info.get("variadic"):
        out["isVariadic"] = True
    if info.get("values"):
        out["suggestions"] = suggestions(info["values"])
    if "default" in info:
        out["default"] = info["default"]
    if override := OVERRIDES.get(key):
        out.update(override)
    elif "dir" in name.lower().split("_"):
        out["template"] = "folders"
    return out


def option(opt: dict, path: list[str]) -> dict:
    names = opt["names"]
    out: dict = {"name": names[0] if len(names) == 1 else names}
    if opt["description"]:
        out["description"] = tidy(opt["description"])
    if arg := opt["arg"]:
        key = " ".join(path) + " " + next(n for n in names if n.startswith("--"))
        out["args"] = arg_spec(arg["name"], "", {**opt, "optional": arg["optional"]}, key)
        if arg["equals"]:
            out["requiresSeparator"] = True
        if arg["variadic"] or re.search(r"\bRepeat\b|may be repeated", opt["description"], re.I):
            out["isRepeatable"] = True
    return out


def command(node: dict, path: list[str], persistent: set[str]) -> dict:
    names = [node["name"], *node["aliases"]]
    out: dict = {"name": names[0] if len(names) == 1 else names}
    if node["description"]:
        out["description"] = tidy(node["description"])
    if node["subcommands"]:
        out["subcommands"] = [command(c, [*path, c["name"]], persistent) for c in node["subcommands"]]
    options = [option(o, path) for o in node["options"] if o["names"][-1] not in persistent]
    if options:
        out["options"] = options
    args = [arg_spec(a["name"], a["description"], a, " ".join([*path, a["name"]])) for a in node["args"]]
    if args:
        out["args"] = args[0] if len(args) == 1 else args
    return out


def all_nodes(node: dict):
    yield node
    for child in node["subcommands"]:
        yield from all_nodes(child)


tree = parse([], "", [])
# Options every command accepts (clap globals); offered once, as persistent root options
persistent = set.intersection(*({o["names"][-1] for o in n["options"]} for n in all_nodes(tree)))
spec = {
    "name": "pnpm",
    "description": "Fast, disk space efficient package manager",
    "args": {
        "name": "script",
        "isOptional": True,
        "filterStrategy": "fuzzy",
        "generators": Raw("npmScriptsGenerator"),
        "isVariadic": True,
    },
    "filterStrategy": "fuzzy",
    "generateSpec": Raw(GENERATE_SPEC),
    "subcommands": [command(c, [c["name"]], persistent) for c in tree["subcommands"]],
    "options": [
        {**option(o, []), **({"isPersistent": True} if o["names"][-1] in persistent else {})} for o in tree["options"]
    ],
}

version = subprocess.run(
    [PNPM, "--version"], capture_output=True, text=True, cwd=CWD, stdin=subprocess.DEVNULL, timeout=30
).stdout.strip()
print(f"// Generated by build-scripts/specs/gen_pnpm.py from pnpm {version}; don't edit by hand")
print(PRELUDE)
print(f"const completionSpec: Fig.Spec = {emit(spec)};\n")
print("export default completionSpec;")
