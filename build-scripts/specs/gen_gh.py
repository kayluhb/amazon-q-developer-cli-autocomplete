"""Builds specs/gh.ts from the installed gh's help output. To regenerate after a gh upgrade:

    python3 build-scripts/specs/crawl_gh.py > /tmp/gh-tree.json
    git -C build/autocomplete-specs show HEAD:src/gh.ts > /tmp/gh.upstream.ts
    python3 build-scripts/specs/gen_gh.py /tmp/gh-tree.json /tmp/gh.upstream.ts > specs/gh.ts
    npx prettier --write specs/gh.ts --parser typescript

Dynamic suggestions reuse upstream's hand-written generators, which are copied over verbatim."""

import json
import re
import subprocess
import sys

from _common import Raw, emit, tidy

tree = json.load(open(sys.argv[1]))
upstream = open(sys.argv[2]).read()

# Upstream's helpers and ghGenerators, everything before its option/spec definitions
PRELUDE = upstream[upstream.index("const filterMessages") : upstream.index("const codespaceOption")]

LIST_ISSUE = """const listIssue: Fig.Generator = {
  cache: { strategy: "stale-while-revalidate" },
  script: ["gh", "issue", "list", "--json=number,title,state"],
  postProcess: (out) => {
    interface Issue {
      number: number;
      state: string;
      title: string;
    }
    const items = JSON.parse(out) as Issue[];
    return items.map(({ number, title, state }) => ({
      name: number.toString(),
      displayName: title,
      description: `#${number}`,
      icon: state === "OPEN" ? "🟢" : "🟣",
    }));
  },
};
"""

# Keys in `gh config --help`, e.g. "- `git_protocol`: the protocol ..."
CONFIG_KEYS = re.findall(
    r"^- `(\w+)`: (.*)$",
    next(c for c in tree["subcommands"] if c["name"] == "config")["longDescription"],
    re.M,
)
# repo commands where upstream used the owner-aware repository generator
CUSTOM_REPO_COMMANDS = {"clone", "fork", "view", "set-default"}
BRANCH_FLAGS = {"--base", "--head", "--branch"}


# Completions the help text can't tell us about, keyed by "<command path> <--flag or positional name>"
OVERRIDES = {
    "gist edit --add": {"template": "filepaths"},
    "codespace create --devcontainer-path": {"generators": Raw('filepaths({ extensions: ["json"] })')},
    "codespace ports visibility port:{public|private|org}": {
        "generators": Raw('keyValue({ separator: ":", values: ["public", "private", "org"], cache: true })'),
        "isVariadic": True,
    },
}


def choices(description: str) -> list[str] | None:
    m = re.search(r"\{([^{}<>]+)\}", description)
    if not m or not re.search(r"[|,]", m.group(1)):
        return None
    return [c.strip() for c in re.split(r"[|,]", m.group(1)) if c.strip()]


def option_args(flag: dict, node: dict, path: list[str]) -> dict:
    arg = flag["arg"]
    args: dict = {"name": arg.strip("<>")}
    if override := OVERRIDES.get(" ".join([*path, flag["long"]])):
        args.update(override)
    elif values := choices(flag["description"]):
        args["suggestions"] = values
    elif flag["long"] == "--json" and node["jsonFields"]:
        args["generators"] = Raw(f"valueList({{ values: {json.dumps(node['jsonFields'])} }})")
    elif arg == "file" or flag["long"].endswith("-file"):
        args["template"] = "filepaths"
    elif arg in {"directory", "dir"} or flag["long"] in {"--dir", "--directory"}:
        args["template"] = "folders"
    elif flag["long"] in BRANCH_FLAGS and path[:1] in (["pr"], ["browse"]):
        args["generators"] = Raw("ghGenerators.remoteBranches")
    return args


def option(flag: dict, node: dict, path: list[str]) -> dict:
    names = [n for n in (flag["short"], flag["long"]) if n]
    desc = re.sub(r"\s*\(default [^)]*\)$", "", flag["description"])
    out: dict = {"name": names[0] if len(names) == 1 else names, "description": tidy(desc)}
    if flag["arg"]:
        out["args"] = option_args(flag, node, path)
        if flag["arg"] in {"strings", "stringArray"}:
            out["isRepeatable"] = True
    return out


def tokens(usage: str) -> list[str]:
    """Split usage into top-level groups, keeping [...] / {...} together."""
    out, depth, cur = [], 0, ""
    for ch in usage:
        if ch in "[{":
            depth += 1
        elif ch in "]}":
            depth -= 1
        if ch == " " and depth == 0:
            out.append(cur)
            cur = ""
        else:
            cur += ch
    return [t for t in out + [cur] if t]


def positional_args(node: dict, path: list[str]) -> list[dict]:
    toks = tokens(node["usage"])[1 + len(path) :]
    args = []
    skip_next = False
    for tok in toks:
        if skip_next:
            skip_next = False
            continue
        if tok.startswith("-"):
            # a flag in the usage line, e.g. `-s <shell>`; its value isn't positional
            skip_next = not tok.startswith("--") or "=" not in tok
            continue
        inner = tok.strip("[]{}")
        if "flags" in inner or inner.startswith("-") or inner == "--":
            continue
        name = re.sub(r"[<>]|\.\.\.", "", inner).strip()
        arg: dict = {"name": name}
        if tok.startswith("["):
            arg["isOptional"] = True
        if "..." in tok:
            arg["isVariadic"] = True
        arg.update(positional_completion(name, path))
        args.append(arg)
    return args


def positional_completion(name: str, path: list[str]) -> dict:
    if override := OVERRIDES.get(" ".join([*path, name])):
        return override
    words = set(re.findall(r"\w+", name))
    if path[:1] == ["pr"] and words & {"number", "url", "branch"}:
        return {"generators": Raw("ghGenerators.listPR")}
    if path[:1] == ["issue"] and "number" in words:
        return {"generators": Raw("listIssue")}
    if path[:1] == ["repo"] and words & {"repository", "repo"}:
        custom = path[-1] in CUSTOM_REPO_COMMANDS
        return {"generators": Raw(f"ghGenerators.{'listCustomRepositories' if custom else 'listRepositories'}")}
    if "alias" in words:
        return {"generators": Raw("ghGenerators.listAlias")}
    if path[:1] == ["config"] and name == "key":
        return {"suggestions": [{"name": k, "description": tidy(d)} for k, d in CONFIG_KEYS]}
    if words & {"directory", "dir"}:
        return {"template": "folders"}
    if words & {"file", "filename", "path"} or "file-path" in name:
        return {"template": "filepaths"}
    return {}


def command(node: dict, path: list[str]) -> dict:
    # ALIASES repeats the leaf name under each parent alias, e.g. `gh ext ls`, `gh extensions ls`
    names = list(dict.fromkeys([node["name"], *node["aliases"]]))
    out: dict = {"name": names[0] if len(names) == 1 else names, "description": tidy(node["description"])}
    if node["subcommands"]:
        out["subcommands"] = [command(c, [*path, c["name"]]) for c in node["subcommands"]]
    flags = [f for f in node["flags"] + node["inheritedFlags"] if f["long"] != "--help"]
    if flags:
        out["options"] = [option(f, node, path) for f in flags]
    if args := positional_args(node, path):
        out["args"] = args[0] if len(args) == 1 else args
    return out


spec = command(tree, [])
spec["args"] = {
    "name": "alias",
    "description": "Custom user defined gh alias",
    "isOptional": True,
    "generators": Raw("ghGenerators.listAlias"),
}
spec["options"] = [
    {"name": "--help", "description": "Show help for command", "isPersistent": True},
    *spec.get("options", []),
]

version = subprocess.run(["gh", "--version"], capture_output=True, text=True).stdout.split("\n")[0]
print(f"// Generated by build-scripts/specs/gen_gh.py from {version}; don't edit by hand")
print('import { filepaths, keyValue, valueList } from "@fig/autocomplete-generators";\n')
print(PRELUDE)
print(LIST_ISSUE)
print(f"const completionSpec: Fig.Spec = {emit(spec)};\n")
print("export default completionSpec;")
