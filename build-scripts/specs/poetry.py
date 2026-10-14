"""Builds specs/poetry.ts from poetry's help output. To regenerate after a poetry release:

    python3 build-scripts/specs/poetry.py crawl uvx --from poetry@latest poetry > /tmp/poetry-tree.json
    python3 build-scripts/specs/poetry.py gen /tmp/poetry-tree.json > specs/poetry.ts
    npx prettier --write specs/poetry.ts --parser typescript

`crawl` takes the command that runs poetry and only ever runs `list` and `help <command>`.
Upstream's spec had no generators or custom suggestions, so everything here comes from the help
text plus the small OVERRIDES table below."""

import json
import re
import subprocess
import sys


def run(poetry, *args):
    return subprocess.run(
        [*poetry, *args, "--no-ansi", "--no-interaction"],
        stdin=subprocess.DEVNULL,
        capture_output=True,
        text=True,
        timeout=120,
        check=True,
    ).stdout


def sections(text):
    """Splits cleo help into {"Description": [...lines], "Usage": [...], ...}."""
    out, current = {}, None
    for line in text.splitlines():
        m = re.match(r"^(\w[\w ]*):$", line)
        if m:
            current = out.setdefault(m.group(1), [])
        elif current is not None:
            current.append(line)
    return out


OPTION_RE = re.compile(r"^  (?:(-[\w|]+), )?\s*(--[\w-]+)(\[?=([A-Z_-]+)\]?)?\s{2,}(.*)$")
ARG_RE = re.compile(r"^  (\S+)\s{2,}(.*)$")


def parse_help(text):
    s = sections(text)
    options = []
    for line in s.get("Options", []):
        m = OPTION_RE.match(line)
        if not m:
            continue
        short, long, valspec, value, desc = m.groups()
        options.append(
            {
                "short": short,
                "long": long,
                "value": value,
                "optionalValue": bool(valspec and valspec.startswith("[")),
                "description": desc.strip(),
            }
        )
    args = [
        {"name": m.group(1), "description": m.group(2).strip()} for m in map(ARG_RE.match, s.get("Arguments", [])) if m
    ]
    usage = " ".join(line.strip() for line in s.get("Usage", []) if line.strip())
    for a in args:
        name = re.escape(a["name"])
        a["optional"] = bool(re.search(rf"\[<{name}>", usage))
        a["variadic"] = bool(re.search(rf"<{name}>\]?\.\.\.", usage))
    return {
        "description": " ".join(line.strip() for line in s.get("Description", []) if line.strip()),
        "options": options,
        "args": args,
    }


def crawl(poetry):
    listing = run(poetry, "list")
    version = re.search(r"version ([\w.]+)", listing).group(1)
    s = sections(listing)
    commands = [m.group(1) for m in map(re.compile(r"^  (\S+(?: \S+)*?)\s{2,}").match, s["Available commands"]) if m]
    tree = {"version": version, "global": parse_help(listing)["options"], "commands": {}}
    for cmd in commands:
        tree["commands"][cmd] = parse_help(run(poetry, "help", cmd))
    return tree


# Descriptions for namespaces, which cleo doesn't describe
NAMESPACES = {
    "cache": "Interact with Poetry's cache",
    "debug": "Debug Poetry and dependency resolution",
    "env": "Interact with the virtualenvs associated with the project",
    "python": "Manage Python versions installed by Poetry (experimental)",
    "self": "Manage the Poetry installation itself",
    "self show": "Show packages from Poetry's runtime environment",
    "source": "Manage repository sources for the project",
}


class Raw(str):
    """Emitted as-is instead of as a string literal."""


CACHE_NAMES = """const CACHE_NAMES: Fig.Generator = {
  script: ["poetry", "cache", "list"],
  postProcess: (out) =>
    out
      .split("\\n")
      .map((line) => line.trim())
      .filter((line) => line && !line.includes(" "))
      .map((name) => ({ name, icon: "fig://icon?type=box" })),
};
"""

FOLDERS = {"template": "folders"}
FILES = {"template": "filepaths"}

# Completions the help text can't tell us about, keyed by "<command path> <--flag or arg name>"
OVERRIDES = {
    " --project": FOLDERS,
    " --directory": FOLDERS,
    "new path": FOLDERS,
    "new --readme": {"suggestions": ["md", "rst"]},
    "show --format": {"suggestions": ["text", "json"]},
    "self show --format": {"suggestions": ["text", "json"]},
    "python install --implementation": {"suggestions": ["cpython", "pypy"]},
    "python list --implementation": {"suggestions": ["cpython", "pypy"]},
    "python remove --implementation": {"suggestions": ["cpython", "pypy"]},
    "cache clear cache": {"generators": Raw("CACHE_NAMES")},
    "build --format": {"suggestions": ["sdist", "wheel"]},
    "build --output": FOLDERS,
    "publish --cert": FILES,
    "publish --client-cert": FILES,
    "publish --dist-dir": FOLDERS,
    "run args": {"isCommand": True},
    "help command_name": {"suggestions": Raw("COMMAND_NAMES")},
    "list namespace": {"suggestions": Raw("NAMESPACE_NAMES")},
    "version version": {"suggestions": ["patch", "minor", "major", "prepatch", "preminor", "premajor", "prerelease"]},
    "source add --priority": {"suggestions": ["primary", "supplemental", "explicit"]},
    "config key": {"isOptional": True},
}


DEFAULT_RE = re.compile(r'\s*\[default: "?([^"\]]*)"?\]')


def clean(desc):
    desc = DEFAULT_RE.sub("", re.sub(r"\s*\(multiple values allowed\)", "", desc))
    return desc[:-1] if desc.endswith(".") and not desc.endswith("..") else desc


def option(cmd, o):
    names = [f"-{n.lstrip('-')}" for n in o["short"].split("|")][:1] if o["short"] else []
    out = {"name": names + [o["long"]] if names else o["long"]}
    out["description"] = clean(o["description"])
    if "(multiple values allowed)" in o["description"]:
        out["isRepeatable"] = True
    if o["long"] == "--verbose":
        out["isRepeatable"] = 3
    if o["value"]:
        arg = {"name": o["value"].lower()}
        if o["optionalValue"]:
            arg["isOptional"] = True
        default = DEFAULT_RE.search(o["description"])
        if default:
            arg["default"] = default.group(1)
        arg.update(OVERRIDES.get(f"{cmd} {o['long']}", {}))
        out["args"] = arg
    return out


def argument(cmd, a):
    out = {"name": a["name"], "description": clean(a["description"])}
    if a["optional"]:
        out["isOptional"] = True
    if a["variadic"]:
        out["isVariadic"] = True
    default = DEFAULT_RE.search(a["description"])
    if default:
        out["default"] = default.group(1)
    out.update(OVERRIDES.get(f"{cmd} {a['name']}", {}))
    return out


def ts(value, indent=0):
    pad = "  " * indent
    if isinstance(value, Raw):
        return str(value)
    if isinstance(value, dict):
        items = [f"{pad}  {k}: {ts(v, indent + 1)}," for k, v in value.items()]
        return "{\n" + "\n".join(items) + f"\n{pad}}}"
    if isinstance(value, list):
        return "[" + ", ".join(ts(v, indent) for v in value) + "]"
    return json.dumps(value, ensure_ascii=False)


def generate(tree):
    global_longs = {o["long"] for o in tree["global"]}
    root = {
        "name": "poetry",
        "description": "Python packaging and dependency management made easy",
        "subcommands": [],
        "options": [],
    }
    nodes = {"": root}

    def node(path):
        if path not in nodes:
            parent, _, name = path.rpartition(" ")
            nodes[path] = {"name": name, "description": NAMESPACES[path], "subcommands": []}
            node(parent)["subcommands"].append(nodes[path])
        nodes[path].setdefault("subcommands", [])
        return nodes[path]

    for o in tree["global"]:
        opt = option("", o)
        opt["isPersistent"] = True
        root["options"].append(opt)
    for path, info in tree["commands"].items():
        parent, _, name = path.rpartition(" ")
        spec = nodes.get(path) or {"name": name}
        spec["description"] = clean(info["description"])
        opts = [option(path, o) for o in info["options"] if o["long"] not in global_longs]
        if opts:
            spec["options"] = opts
        args = [argument(path, a) for a in info["args"]]
        if args:
            spec["args"] = args[0] if len(args) == 1 else args
        if path not in nodes:
            nodes[path] = spec
            node(parent)["subcommands"].append(spec)
        else:  # a namespace that is also a command, e.g. `self show`; keep subcommands last
            subs = spec.pop("subcommands")
            spec["subcommands"] = subs
    for spec in nodes.values():
        if "subcommands" in spec and not spec["subcommands"]:
            del spec["subcommands"]
        elif "subcommands" in spec:
            spec["subcommands"].sort(key=lambda s: s["name"])

    top = sorted(c for c in tree["commands"] if " " not in c)
    namespaces = sorted({c.split(" ")[0] for c in tree["commands"] if " " in c})
    return (
        f"// Generated by build-scripts/specs/poetry.py from poetry version {tree['version']}; don't edit by hand\n"
        f"const COMMAND_NAMES = {ts(top)};\n\n"
        f"const NAMESPACE_NAMES = {ts(namespaces)};\n\n"
        f"{CACHE_NAMES}\n"
        f"const completionSpec: Fig.Spec = {ts(root)};\n\n"
        "export default completionSpec;\n"
    )


if __name__ == "__main__":
    if sys.argv[1] == "crawl":
        json.dump(crawl(sys.argv[2:]), sys.stdout, indent=1)
    elif sys.argv[1] == "gen":
        sys.stdout.write(generate(json.load(open(sys.argv[2]))))
    else:
        sys.exit(__doc__)
