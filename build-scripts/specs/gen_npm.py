"""Builds specs/npm.ts from npm's help output and bundled docs. To regenerate after an npm release:

    mkdir /tmp/npm-latest && cd /tmp/npm-latest && npm pack npm@latest && tar xzf npm-*.tgz
    git -C build/autocomplete-specs show HEAD:src/npm.ts > /tmp/npm.upstream.ts
    python3 build-scripts/specs/gen_npm.py /tmp/npm-latest/package /tmp/npm.upstream.ts > specs/npm.ts
    npx prettier --write specs/npm.ts --parser typescript

The first argument is an unpacked npm package (bin/npm-cli.js and docs/); only `npm <command> --help` is run.
Commands, aliases, usage lines and option flags come from that help; option descriptions come from the
`#### \\`<config>\\`` sections in docs/content (what `npm help <command>` shows), falling back to the help text.
Upstream's hand-written generators (registry search, scripts, dependencies, workspaces) are copied over verbatim
and attached through the tables below. Warnings about undescribed subcommands go to stderr."""

import os
import re
import subprocess
import sys
from collections import Counter

from _common import Raw, emit
from _common import tidy as base_tidy

PKG = sys.argv[1]
upstream = open(sys.argv[2]).read()

# Upstream's search handler and generators; the option constants and spec after them are regenerated.
# Fixes a typo in one of its comments (split so the typos hook doesn't flag this line).
PRELUDE = upstream[upstream.index("const atsInStr") : upstream.index("const globalOption")].replace(
    "sugge" + "tsions", "suggestions"
)


def npm(*args: str) -> str:
    cli = os.path.join(PKG, "bin", "npm-cli.js")
    result = subprocess.run(
        ["node", cli, *args],
        capture_output=True,
        text=True,
        stdin=subprocess.DEVNULL,
        timeout=30,
        env={**os.environ, "NO_COLOR": "1", "npm_config_update_notifier": "false"},
    )
    return result.stdout


# Aliases under `// typos` in npm's lib/utils/cmd-list.js (misspellings and install prefixes) would only
# clutter suggestions; a few real names are listed there too, and one misspelling is filed under short names
CMD_LIST = open(os.path.join(PKG, "lib", "utils", "cmd-list.js")).read()
CMD_ALIASES = dict(re.findall(r"^\s+'?([\w-]+)'?: '([\w-]+)',$", CMD_LIST.split("const aliases = {")[1], re.M))
TYPO_ALIASES = set(re.findall(r"^\s+'?([\w-]+)'?:", CMD_LIST.split("// typos")[1].split("}")[0], re.M)) - {
    "upgrade",
    "dist-tags",
    "install-clean",
} | {"veri" + "son"}  # split so the typos hook doesn't flag it


# Subcommands that only appear in usage lines, so their help has no description
SUB_DESCRIPTIONS = {
    "access list": "List packages or collaborators",
    "access list packages": "List all packages a user, scope or team can access",
    "access list collaborators": "List all users who have access to a package",
    "access get": "Show a package's access settings",
    "access get status": "Show whether a package is public or private",
    "access set": "Set a package's visibility or 2FA requirement",
    "access grant": "Give a team read-only or read-write access to a package",
    "access revoke": "Remove a team's access to a package",
    "audit fix": "Apply remediations to the package tree",
    "audit signatures": "Verify the signatures and attestations of installed packages",
    "cache add": "Add the specified packages to the local cache",
    "cache clean": "Delete a single entry or all entries from the cache folder",
    "cache ls": "List all entries in the npm cache",
    "cache verify": "Verify the cache folder, garbage collecting unneeded data and checking integrity",
    "cache npx": "Manage the npx cache",
    "cache npx ls": "List all entries in the npx cache",
    "cache npx rm": "Remove the given entries, or all entries, from the npx cache",
    "cache npx info": "Show detailed information about the given npx cache entries",
    "config set": "Set config keys to the given values",
    "config get": "Print config values to stdout",
    "config delete": "Delete keys from all configuration files",
    "config list": "Show all the config settings",
    "config edit": "Open the config file in an editor",
    "config fix": "Repair invalid configuration items",
    "dist-tag add": "Tag a version of a package",
    "dist-tag rm": "Remove a tag from a package",
    "dist-tag ls": "Show all of the dist-tags for a package",
    "install-scripts approve": "Allow install scripts to run for specific dependencies",
    "install-scripts deny": "Block install scripts for specific dependencies",
    "install-scripts ls": "List dependencies with install scripts and their approval status",
    "install-scripts prune": "Remove approvals for dependencies that are no longer installed",
    "org set": "Add a user to an org or change their role",
    "org rm": "Remove a user from an org",
    "org ls": "List users in an org, or see what role a user has in it",
    "owner add": "Add a new user as a maintainer of a package",
    "owner rm": "Remove a user from the package owner list",
    "owner ls": "List all the users who can modify a package and push new versions",
    "patch add": "Extract a clean copy of a package into a directory for editing",
    "patch commit": "Save the changes in an edit directory as a patch",
    "patch update": "Move a patch to a new version of the package",
    "patch ls": "List registered patches and how many installed packages each matches",
    "patch rm": "Remove a package's patch",
    "pkg set": "Set values in package.json",
    "pkg get": "Print values from package.json",
    "pkg delete": "Delete keys from package.json",
    "pkg fix": "Fix common errors in package.json",
    "profile enable-2fa": "Enable two-factor authentication",
    "profile disable-2fa": "Disable two-factor authentication",
    "profile get": "Display all of the properties of your profile, or one or more specific properties",
    "profile set": "Set the value of a profile property",
    "team create": "Create a new team",
    "team destroy": "Destroy an existing team",
    "team add": "Add a user to a team",
    "team rm": "Remove a user from a team",
    "team ls": "List teams in a scope, or users in a team",
    "token list": "Show all active authentication tokens",
    "token revoke": "Remove an authentication token from the registry",
    "token create": "Create a new authentication token",
}

# --no-<config> flags; the docs only describe the positive config
NO_DESCRIPTIONS = {
    "--no-save": "Don't save installed packages to package.json",
    "--no-package-lock": "Ignore package-lock.json files when installing",
    "--no-audit": "Don't submit audit reports to the registry",
    "--no-bin-links": "Don't create symlinks (or .cmd shims on Windows) for package executables",
    "--no-fund": "Don't show the message about dependencies looking for funding",
    "--no-browser": "Print URLs to the terminal instead of opening a browser",
    "--no-unicode": "Use ascii characters instead of unicode glyphs in the output",
    "--no-color": "Don't use colors in the output",
    "--no-description": "Don't show package descriptions in search results",
    "--no-allow-scripts-pin": "Approve install scripts without pinning the approved version",
    "--no-workspaces-update": "Don't update node_modules after adding or changing a workspace",
    "--no-commit-hooks": "Don't run git commit hooks when bumping the version",
    "--no-git-tag-version": "Don't commit and tag the version bump in git",
}

# Option values that look like {a|b} but aren't choices
NOT_CHOICES = {"pkg|glob"}

DANGEROUS = {"--force", "--dangerously-allow-all-scripts"}

# Positionals that complete packages from the registry, or installed dependencies
REGISTRY_PACKAGE = Raw("npmSearchGenerator")
DEPENDENCY = Raw("dependenciesGenerator")
REGISTRY_COMMANDS = {
    "install",
    "install-test",
    "bugs",
    "docs",
    "repo",
    "view",
    "deprecate",
    "undeprecate",
    "unpublish",
    "dist-tag add",
    "dist-tag rm",
    "dist-tag ls",
    "owner add",
    "owner rm",
    "owner ls",
    "cache add",
}
DEPENDENCY_COMMANDS = {
    "uninstall",
    "update",
    "explore",
    "edit",
    "explain",
    "ls",
    "ll",
    "outdated",
    "rebuild",
    "prune",
    "fund",
    "patch",
    "patch add",
    "patch update",
    "patch rm",
    "approve-scripts",
    "deny-scripts",
    "install-scripts approve",
    "install-scripts deny",
}
PACKAGE_ARG = re.compile(r"^(@scope/)?(pkg|pkgname|package|package-spec)(@version|@old-version)?$")

PROFILE_PROPERTIES = ["email", "fullname", "homepage", "freenode", "twitter", "github"]

# Completions the help text can't express, keyed by "<command path> <positional name or --flag>".
# Values are merged into the generated arg; None removes a key. A "<command path>" key replaces all args.
OVERRIDES = {
    "run command": {
        "name": "script",
        "description": "Script to run from your package.json",
        "filterStrategy": "fuzzy",
        "generators": Raw("npmScriptsGenerator"),
    },
    "exec": [
        {
            "name": "command",
            "description": "Package binary to run, with its arguments",
            "isOptional": True,
            "isVariadic": True,
        },
    ],
    "help": [{"name": "term", "isVariadic": True, "suggestions": Raw("commandNames")}],
    "doctor": [
        {
            "name": "check",
            "isOptional": True,
            "isVariadic": True,
            "suggestions": ["connection", "registry", "versions", "environment", "permissions", "cache"],
        }
    ],
    "access set": [
        {
            "name": "setting",
            "suggestions": ["status=public", "status=private", "mfa=none", "mfa=publish", "mfa=automation"],
        },
        {"name": "package", "isOptional": True},
    ],
    "token revoke id|token": {"suggestions": None, "name": "id|token"},
    "org set role": {"isOptional": True},
    "edit pkg/subpkg": {"name": "pkg", "isVariadic": None},
    "init package-spec": {"isOptional": True},
    "ls package-spec": {"isOptional": True, "isVariadic": True},
    "pack package-spec": {"isOptional": True},
    "publish package-spec": {
        "name": "tarball|folder",
        "isOptional": True,
        "template": "folders",
        "description": "A tarball or a folder containing a package.json file",
    },
    "stage publish package-spec": {
        "name": "tarball|folder",
        "isOptional": True,
        "template": "folders",
        "description": "A tarball or a folder containing a package.json file",
    },
    "link package-spec": {"template": "filepaths"},
    "patch commit edit-dir": {"template": "folders"},
    "profile set key": {"suggestions": PROFILE_PROPERTIES},
    "profile enable-2fa auth-only|auth-and-writes": {
        "name": "mode",
        "suggestions": [
            {
                "name": "auth-only",
                "description": "Require an OTP when logging in or making changes to your account's authentication",
            },
            {
                "name": "auth-and-writes",
                "description": "Also require an OTP when publishing, setting the latest dist-tag, or changing access",
            },
        ],
    },
    "--workspace": {"generators": Raw("workspaceGenerator")},
    "--pack-destination": {"template": "folders"},
    "--patches-dir": {"template": "folders"},
    "--edit-dir": {"template": "folders"},
    "--cache": {"template": "folders"},
    "--file": {"template": "filepaths"},
    "--provenance-file": {"template": "filepaths"},
    "--package": {"generators": REGISTRY_PACKAGE, "debounce": True},
    "--diff": {"generators": REGISTRY_PACKAGE, "debounce": True},
}

# Global configs worth offering on every command
GLOBAL_OPTIONS = [
    {"name": ["-h", "--help"], "description": "Show help for a command", "isPersistent": True},
    {
        "name": "--loglevel",
        "description": "What level of logs to report",
        "isPersistent": True,
        "args": {
            "name": "loglevel",
            "suggestions": ["silent", "error", "warn", "notice", "http", "info", "verbose", "silly"],
        },
    },
    {"name": ["-s", "--silent"], "description": "Don't log anything but errors", "isPersistent": True},
    {"name": "--verbose", "description": "Log extra information", "isPersistent": True},
    {
        "name": ["-C", "--prefix"],
        "description": "Run in this directory instead of the current one",
        "isPersistent": True,
        "args": {"name": "dir", "template": "folders"},
    },
    {
        "name": "--userconfig",
        "description": "The location of the user-level configuration settings",
        "isPersistent": True,
        "args": {"name": "file", "template": "filepaths"},
    },
]


def tidy(description: str) -> str:
    """The shared tidy, also without a trailing colon."""
    return base_tidy(description).removesuffix(":")


def strip_markdown(text: str) -> str:
    text = re.sub(r"\[([^\]]+)\]\([^)]*\)", r"\1", text)
    text = re.sub(r"\*\*([^*]+)\*\*", r"\1", text)
    return re.sub(r"(?<![\w])_([^_]+)_(?![\w])", r"\1", text)


def config_docs(path: str) -> dict[str, str]:
    """First sentence of each `#### \\`<config>\\`` section in a docs markdown file."""
    out = {}
    text = open(path).read()
    for m in re.finditer(r"^#### `([^`]+)`\n(.*?)(?=^#{1,4} |\Z)", text, re.M | re.S):
        paras = [p for p in re.split(r"\n\s*\n", m.group(2)) if p.strip() and not p.lstrip().startswith("* ")]
        if not paras:
            continue
        para = strip_markdown(" ".join(paras[0].split()))
        out[m.group(1)] = tidy(re.split(r"(?<!\be\.g)(?<!\bi\.e)(?<=[.!?])\s+", para)[0])
    return out


DOCS = os.path.join(PKG, "docs", "content")
CONFIG_DOCS = config_docs(os.path.join(DOCS, "using-npm", "config.md"))
# `-D`: `--save-dev` style shorthands
SHORTHANDS = {
    long: short
    for short, long in re.findall(
        r"^\* `(-\w)`: `(--[\w-]+)`$", open(os.path.join(DOCS, "using-npm", "config.md")).read(), re.M
    )
    if long not in {"--usage", "--version"}
}
# Multi-letter shorthands from the same table (config definitions/index.js), which the docs list as `--ws` and
# `--iwr`; nopt also accepts them with one dash, the form upstream's spec used
SHORTHANDS.update({"--workspaces": "-ws", "--include-workspace-root": "-iwr"})


def split_top(text: str, sep: str) -> list[str]:
    """Split on sep outside of [] <> () and quotes."""
    out, depth, cur, quote = [], 0, "", False
    for ch in text:
        if ch == "'":
            quote = not quote
        elif not quote and ch in "[<(":
            depth += 1
        elif not quote and ch in "]>)":
            depth -= 1
        if ch == sep and depth == 0 and not quote:
            out.append(cur)
            cur = ""
        else:
            cur += ch
    return [t.strip() for t in out + [cur] if t.strip()]


def is_wrapped(tok: str) -> bool:
    """True if tok is one [...] group, not e.g. `[<@scope>/]<pkg>`."""
    if not tok.startswith("[") or not tok.endswith("]"):
        return False
    depth = 0
    for i, ch in enumerate(tok):
        depth += ch == "["
        depth -= ch == "]"
        if depth == 0 and i < len(tok) - 1:
            return False
    return True


def parse_help(text: str) -> dict:
    lines = text.splitlines()
    usage, options, details, subs, aliases = [], [], {}, [], []
    section = "description"
    i = 0
    while i < len(lines):
        line = lines[i]
        if line in ("Usage:", "Options:", "Subcommands:"):
            section = line[:-1].lower()
        elif not line.strip():
            if section in ("usage", "options"):
                section = "details" if section == "options" else "between"
        elif m := re.match(r"^alias(?:es)?: (.*)$", line):
            aliases = [a.strip() for a in m.group(1).split(",")]
        elif section == "usage" and line.startswith("npm "):
            usage.append(line)
        elif section == "options":
            options.append(line)
        elif section == "details" and (m := re.match(r"^  (-\S+)( \(required\))?$", line)):
            desc = lines[i + 1].strip() if i + 1 < len(lines) else ""
            details[m.group(1)] = {"names": m.group(1).split("|"), "required": bool(m.group(2)), "description": desc}
        elif section == "subcommands" and (m := re.match(r"^  (\S+)$", line)):
            subs.append(m.group(1))
        i += 1
    return {
        "description": lines[0] if lines else "",
        "usage": usage,
        "options": " ".join(options),
        "details": details,
        "subcommands": subs,
        "aliases": aliases,
    }


def parse_options(block: str) -> list[dict]:
    opts: dict[str, dict] = {}
    for group in split_top(block, " "):
        inner = group[1:-1]
        repeat = re.search(r"\s\[\S+ <[^>]+> \.\.\.\]$", inner)
        if repeat:
            inner = inner[: repeat.start()]
        pending_short = None
        for alt in split_top(inner, "|"):
            flag, _, arg = alt.partition(" ")
            if re.fullmatch(r"-\w", flag) and not arg:
                pending_short = flag
                continue
            if flag in opts:  # `--color|--no-color|--color always`
                if arg:
                    opts[flag]["arg"] = {"name": arg, "optional": True}
                continue
            names = [pending_short, flag] if pending_short else [flag]
            pending_short = None
            opt = {"names": names, "repeatable": bool(repeat)}
            if arg:
                opt["arg"] = {"name": arg.strip("<>")}
            opts[flag] = opt
    return list(opts.values())


def option_description(long: str, cmd: str, details: dict) -> str:
    if long in NO_DESCRIPTIONS:
        return NO_DESCRIPTIONS[long]
    key = long[2:]
    cmd_docs = os.path.join(DOCS, "commands", f"npm-{cmd}.md")
    docs = config_docs(cmd_docs) if os.path.exists(cmd_docs) else {}
    if key in docs:
        return docs[key]
    if key in CONFIG_DOCS:
        return CONFIG_DOCS[key]
    detail = next((d for d in details.values() if long in d["names"]), None)
    return tidy(detail["description"]) if detail else ""


def option(opt: dict, cmd: str, details: dict) -> dict:
    long = opt["names"][-1]
    names = list(opt["names"])
    if long in SHORTHANDS and SHORTHANDS[long] not in names:
        names.insert(0, SHORTHANDS[long])
    detail = next((d for d in details.values() if set(d["names"]) & set(names)), None)
    if detail:
        names += [n for n in detail["names"] if n not in names]
    out: dict = {"name": names[0] if len(names) == 1 else names}
    if desc := option_description(long, cmd, details):
        out["description"] = desc
    if detail and detail["required"]:
        out["isRequired"] = True
    if long in DANGEROUS:
        out["isDangerous"] = True
    if arg := opt.get("arg"):
        args: dict = {"name": arg["name"]}
        if "|" in arg["name"] and arg["name"] not in NOT_CHOICES:
            values = arg["name"].split("|")
            args = {"name": long[2:], "suggestions": values}
            if opt["repeatable"]:
                out["isRepeatable"] = len(values)
        elif opt["repeatable"]:
            out["isRepeatable"] = True
        if arg.get("optional"):
            args["isOptional"] = True
            args["suggestions"] = [arg["name"]]
        args.update(OVERRIDES.get(long, {}))
        out["args"] = args
    elif opt["repeatable"]:
        out["isRepeatable"] = True
    return out


def usage_tokens(line: str) -> tuple[list[str], list[str]]:
    """(literal subcommand path, remaining positional tokens) for `npm <cmd> ...`."""
    toks = split_top(line, " ")[2:]
    path = []
    while toks and re.fullmatch(r"[a-z][a-z0-9-]*", toks[0]) and toks[0] not in {"orgname", "username"}:
        path.append(toks.pop(0))
    return path, toks


def positional(tok: str, optional: bool = False) -> list[dict]:
    if tok.startswith("(") or "--" in tok or tok.startswith("'"):
        return []
    if is_wrapped(tok):
        inner = tok[1:-1].strip()
        if len(split_top(inner, "|")) == 1 and len(split_top(inner, " ")) > 1:
            return [{**a, "isOptional": True} for a in positional_args(split_top(inner, " "))]
        return positional(inner, True)
    alts = split_top(tok, "|")
    if len(alts) == 1 and tok.startswith("<") and tok.endswith(">") and "|" in tok:
        alts = tok[1:-1].split("|")  # <read-only|read-write>
        arg: dict = {"name": tok[1:-1], "suggestions": alts}
    elif len(alts) > 1:
        literals = [a for a in alts if re.fullmatch(r"[\w-]+", a)]
        named = [a.strip("<>") for a in alts if a not in literals]
        arg = {"name": "|".join(named) if named else ("role" if "admin" in literals else "|".join(literals))}
        if literals:
            arg["suggestions"] = literals
    else:
        variadic = tok.endswith("...") or tok.endswith("..>") or tok.startswith("...")
        name = re.sub(r"\([^)]*\)|\.\.+|[<>\[\]]", "", tok).strip()
        arg = {"name": name}
        if variadic:
            arg["isVariadic"] = True
    if optional:
        arg["isOptional"] = True
    return [arg]


def merge_args(args: list[dict], new: list[dict]) -> list[dict]:
    for arg in new:
        if args and arg["name"] == args[-1]["name"]:  # `<pkg> [<pkg> ...]`
            args[-1]["isVariadic"] = True
        else:
            args.append(arg)
    return args


def positional_args(toks: list[str]) -> list[dict]:
    args: list[dict] = []
    skip_value = False
    for tok in toks:
        if skip_value and tok.startswith("<"):
            skip_value = False
            continue
        skip_value = False
        if tok.startswith("-"):
            skip_value = "=" not in tok
            continue
        if tok.startswith("[-"):
            continue
        if re.fullmatch(r"\.\.\.\]?", tok):  # `[<package-spec>] ...]`
            if args:
                args[-1]["isVariadic"] = True
            continue
        args = merge_args(args, positional(tok))
    return args


def complete(arg: dict, path: str) -> dict:
    key = f"{path} {arg['name']}"
    if PACKAGE_ARG.match(arg["name"]):
        if path in REGISTRY_COMMANDS:
            arg.update({"generators": REGISTRY_PACKAGE, "debounce": True})
        elif path in DEPENDENCY_COMMANDS:
            arg.update({"generators": DEPENDENCY, "filterStrategy": "fuzzy"})
    if path in ("config get", "config delete", "get") and arg["name"] == "key":
        arg["suggestions"] = Raw("configKeys")
    if path in ("config set", "set") and arg["name"] == "key=value":
        arg["generators"] = Raw('keyValue({ keys: configKeys, separator: "=" })')
    for k, v in OVERRIDES.get(key, {}).items():
        if v is None:
            arg.pop(k, None)
        else:
            arg[k] = v
    return arg


def optional_subcommands(toks: list[str]) -> list[str]:
    """`npm audit [fix|signatures]`: optional literal subcommands, or []."""
    if not toks or not is_wrapped(toks[0]):
        return []
    alts = split_top(toks[0][1:-1], "|")
    return alts if len(alts) > 1 and all(re.fullmatch(r"[a-z-]+", a) for a in alts) else []


def node(name: str, description: str, path: list[str], usage: list[str], options: list[dict]) -> dict:
    """A command built from usage lines; subcommands that only appear in usage share its options."""
    key = " ".join(path)
    out: dict = {"name": name}
    if description:
        out["description"] = tidy(description)
    else:
        print(f"warning: no description for {key}", file=sys.stderr)
    children: dict[str, list[str]] = {}
    own_args: list[dict] = []
    for line in usage:
        sub_path, toks = usage_tokens(line)
        if not sub_path and len(path) == 1 and (alts := optional_subcommands(toks)):
            for a in alts:
                children.setdefault(a, [])
            continue
        rel = sub_path[len(path) - 1 :]
        if rel:
            children.setdefault(rel[0], []).append(line)
        elif not own_args:
            own_args = positional_args(toks)
    if children:
        out["subcommands"] = [
            node(c, SUB_DESCRIPTIONS.get(f"{key} {c}", ""), [*path, c], lines, []) for c, lines in children.items()
        ]
    if options:
        out["options"] = options
    if key in OVERRIDES and isinstance(OVERRIDES[key], list):
        own_args = [dict(a) for a in OVERRIDES[key]]
    else:
        own_args = [complete(a, key) for a in own_args]
    if own_args:
        out["args"] = own_args[0] if len(own_args) == 1 else own_args
    if any("[-- <args>]" in line for line in usage):
        out.setdefault("options", []).append(
            {
                "name": "--",
                "description": "Pass the remaining arguments to the script",
                "args": {"name": "args", "isVariadic": True},
            }
        )
    return out


def command(cmd: str, path: list[str], text: str) -> dict:
    h = parse_help(text)
    opts = [option(o, path[0], h["details"]) for o in parse_options(h["options"])]
    if h["subcommands"]:
        # trust/stage: each subcommand has its own help
        out = node(cmd, h["description"], path, [], [])
        out["subcommands"] = [command(s, [*path, s], npm(*path, s, "--help")) for s in h["subcommands"]]
        own = [line for line in h["usage"] if usage_tokens(line)[0] == path[1:]]
        if own:
            out.update({k: v for k, v in node(cmd, h["description"], path, own, []).items() if k == "args"})
    else:
        has_subs = any(
            usage_tokens(line)[0][len(path) - 1 :] or (len(path) == 1 and optional_subcommands(usage_tokens(line)[1]))
            for line in h["usage"]
        )
        if has_subs:
            opts = [{**o, "isPersistent": True} for o in opts]
        out = node(cmd, h["description"], path, h["usage"], opts)
    # Only top-level aliases resolve: base-cmd prints view's aliases for `npm stage view` too (it matches the
    # name `view`), but stage looks its subcommands up by exact name
    aliases = h["aliases"] if len(path) == 1 else []
    names = [cmd, *[a for a in aliases if a != cmd and a not in TYPO_ALIASES]]
    if len(path) == 1:
        expected = {a for a, c in CMD_ALIASES.items() if c == cmd and a not in TYPO_ALIASES}
        if set(names[1:]) != expected:
            sys.exit(f"error: {cmd} aliases {names[1:]} don't match cmd-list.js {sorted(expected)}")
    out["name"] = names[0] if len(names) == 1 else names
    return out


def share_options(spec: dict) -> list[str]:
    """Hoist options repeated across commands into constants, keeping one variant per flag."""
    counts: Counter = Counter()

    def walk(n, fn):
        for i, o in enumerate(n.get("options", [])):
            fn(n, i, o)
        for s in n.get("subcommands", []):
            walk(s, fn)

    walk(spec, lambda n, i, o: counts.update([emit(o)]))
    consts, chosen = {}, {}
    for text, count in counts.most_common():
        if count < 3:
            break
        flag = re.search(r'"(--[\w-]+)"', text)
        if not flag or flag.group(1) in chosen:
            continue
        ident = re.sub(r"-(\w)", lambda m: m.group(1).upper(), flag.group(1)[2:]) + "Option"
        chosen[flag.group(1)] = ident
        consts[text] = ident

    def replace(n, i, o):
        if (ident := consts.get(emit(o))) is not None:
            n["options"][i] = Raw(ident)

    walk(spec, replace)
    return [f"const {ident}: Fig.Option = {text};\n" for text, ident in consts.items()]


top = npm("help")
commands = re.sub(r"\s", "", top.split("All commands:")[1].split("Specify")[0]).split(",")
spec = {
    "name": "npm",
    "parserDirectives": {"flagsArePosixNoncompliant": True},
    "description": "Node package manager",
    "subcommands": [command(c, [c], npm(c, "--help")) for c in commands],
    "options": [
        {"name": ["-v", "--version"], "description": "Show npm's version"},
        {"name": "-l", "description": "Display usage info for all commands"},
        *GLOBAL_OPTIONS,
    ],
}
shared = share_options(spec)
config_keys = sorted(k for k in CONFIG_DOCS if not k.startswith("_"))
config_suggestions = [{"name": k, "description": CONFIG_DOCS[k]} for k in config_keys]

version = npm("--version").strip()
print(f"// Generated by build-scripts/specs/gen_npm.py from npm {version}; don't edit by hand")
print('import { keyValue } from "@fig/autocomplete-generators";\n')
print(PRELUDE)
print(f"const configKeys: Fig.Suggestion[] = {emit(config_suggestions)};\n")
print(f"const commandNames: string[] = {emit(commands)};\n")
print("\n".join(shared))
print(f"const completionSpec: Fig.Spec = {emit(spec)};\n")
print("export default completionSpec;")
