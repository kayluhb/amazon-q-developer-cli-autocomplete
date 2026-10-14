"""Builds specs/sanity.ts from the oclif manifests of @sanity/cli and the plugins it bundles.

The sanity CLI is oclif-based, and each package ships an oclif.manifest.json with every
command, flag and argument, so this reads those instead of crawling `--help`. To regenerate
after a sanity release:

    V=$(npm view @sanity/cli version); cd "$(mktemp -d)"
    npm pack @sanity/cli@$V && mkdir cli && tar xzf sanity-cli-$V.tgz -C cli
    # the plugin versions @sanity/cli pins
    for p in runtime-cli workflow-cli; do
      pv=$(npm view @sanity/cli@$V dependencies.@sanity/$p)
      npm pack @sanity/$p@$pv && mkdir $p && tar xzf sanity-$p-$pv.tgz -C $p
    done
    python3 -I <repo>/build-scripts/specs/gen_sanity.py cli/package runtime-cli/package \\
      workflow-cli/package > <repo>/specs/sanity.ts
    npx prettier --write <repo>/specs/sanity.ts --parser typescript

The first argument is @sanity/cli's package directory; the rest are its oclif plugins.
Hidden commands and flags are left out. Path completions, which the manifests can't tell
us about, come from PATH_ARGS / PATH_FLAGS below."""

import json
import pathlib
import re
import sys

cli_dir, *plugin_dirs = (pathlib.Path(p) for p in sys.argv[1:])

# Completions keyed by "<command id> <arg name>" / "<command id> --<flag>"
PATH_ARGS = {
    "build outputDir": "folders",
    "preview outputDir": "folders",
    "deploy sourceDir": "folders",
    "blueprints:init dir": "folders",
    "exec script": "filepaths",
    "datasets:export destination": "filepaths",
    "datasets:import source": "filepaths",
    "documents:create file": "filepaths",
    "media:export destination": "filepaths",
    "media:import source": "filepaths",
}
PATH_FLAGS = {
    "init --output-path": "folders",
    "schemas:deploy --manifest-dir": "folders",
}
DANGEROUS = {"delete", "destroy", "nuke"}
# Topics with no description of their own (oclif's help shows the first command's summary)
TOPIC_FALLBACKS = {
    "context:imports": "Manage imports for a knowledge base",
    "context:jobs": "Inspect knowledge base jobs",
    "datasets:alias": "Manage dataset aliases",
    "datasets:embeddings": "Manage embeddings for a dataset",
    "datasets:visibility": "Get or set the visibility of a dataset",
    "functions:env": "Manage environment variables for a deployed function",
}


def tidy(description: str) -> str:
    """First paragraph, capitalised, no trailing dot (the fig linter's style)."""
    description = re.split(r"\n\s*\n", description.strip())[0]
    description = " ".join(description.split()).removesuffix(".")
    return description[:1].upper() + description[1:]


def load(pkg: pathlib.Path) -> tuple[dict, dict, dict]:
    pkg_json = json.loads((pkg / "package.json").read_text())
    manifest = json.loads((pkg / "oclif.manifest.json").read_text())
    oclif = pkg_json.get("oclif", {})
    return manifest, oclif.get("topics", {}), pkg_json


manifest, _, cli_pkg = load(cli_dir)
commands = dict(manifest["commands"])
# @sanity/cli keeps its topics in oclif.config.js rather than package.json
TOPICS = {
    m.group(1): m.group(2)
    for m in re.finditer(r"(\w[\w-]*): \{description: '([^']*)'\}", (cli_dir / "oclif.config.js").read_text())
}
for plugin in plugin_dirs:
    plugin_manifest, plugin_topics, _ = load(plugin)
    commands.update(plugin_manifest["commands"])
    pending = [(k, v) for k, v in plugin_topics.items()]
    while pending:
        topic_id, topic = pending.pop()
        TOPICS[topic_id] = topic["description"]
        pending += [(f"{topic_id}:{k}", v) for k, v in topic.get("subtopics", {}).items()]

# Singular topic names, eg "dataset" for "datasets" (resolved by a command_not_found hook)
alias_src = (cli_dir / "dist" / "topicAliases.js").read_text()
alias_src = alias_src[alias_src.index("topicAliases = {") : alias_src.index("};")]
TOPIC_ALIASES = {
    m.group(1): re.findall(r"'([^']+)'", m.group(2)) for m in re.finditer(r"([\w-]+): \[([^\]]*)\]", alias_src)
}


def flag_options(cmd_id: str, flags: dict) -> list[dict]:
    visible = {n: f for n, f in flags.items() if not f.get("hidden")}

    def names_of(name: str) -> list[str]:
        f = visible.get(name)
        if not f:
            return []
        return [f"--{name}", *([f"--no-{name}"] if f.get("allowNo") else [])]

    out = []
    for name, f in visible.items():
        long = f"--{name}"
        # An option whose negation is a separate hidden boolean, eg init's "--[no-]git=<message>"
        if "[no-]" in f.get("helpLabel", "") and f.get("type") == "option":
            f = {**f, "allowNo": True}
        names = [f"-{f['char']}", long] if f.get("char") else long
        opt: dict = {"name": names, "description": tidy(f.get("description", name))}
        if f["type"] == "option":
            arg: dict = {"name": (f.get("helpValue") or name).strip("<>").removesuffix("...")}
            if f.get("options"):
                arg["suggestions"] = f["options"]
            if isinstance(f.get("default"), str) and f["default"]:
                arg["default"] = f["default"]
            template = PATH_FLAGS.get(f"{cmd_id} {long}")
            if not template and f.get("helpValue"):
                hv = f["helpValue"].lower()
                template = "folders" if "dir" in hv else "filepaths" if re.search(r"file|path", hv) else None
            if template:
                arg["template"] = template
            opt["args"] = arg
            if f.get("multiple"):
                opt["isRepeatable"] = True
        if f.get("required"):
            opt["isRequired"] = True
        if f.get("deprecated"):
            opt["deprecated"] = True
        exclusive = [n for e in f.get("exclusive", []) for n in names_of(e)]
        if f.get("allowNo"):
            exclusive.append(f"--no-{name}")
        if exclusive:
            opt["exclusiveOn"] = exclusive
        depends = [n for d in f.get("dependsOn", []) for n in names_of(d)[:1]]
        if depends:
            opt["dependsOn"] = depends
        out.append(opt)
        if f.get("allowNo"):
            no_desc = flags.get(f"no-{name}", {}).get("description") or f"Negate {long}"
            no: dict = {"name": f"--no-{name}", "description": tidy(no_desc), "exclusiveOn": [long]}
            if f.get("deprecated"):
                no["deprecated"] = True
            out.append(no)
    return out


def positional_args(cmd_id: str, cmd: dict) -> list[dict]:
    args = [a for a in cmd["args"].values() if not a.get("hidden")]
    out = []
    for i, a in enumerate(args):
        arg: dict = {"name": a["name"]}
        if a.get("description"):
            arg["description"] = tidy(a["description"])
        if not a.get("required"):
            arg["isOptional"] = True
        if a.get("multiple") or (cmd.get("strict") is False and i == len(args) - 1):
            arg["isVariadic"] = True
        if a.get("options"):
            arg["suggestions"] = a["options"]
        if template := PATH_ARGS.get(f"{cmd_id} {a['name']}"):
            arg["template"] = template
        out.append(arg)
    return out


def leaf(cmd_id: str, cmd: dict) -> dict:
    name = cmd_id.split(":")[-1]
    # Aliases at the same level, eg "tokens add" for "tokens create", "start" for "preview"
    prefix = cmd_id.split(":")[:-1]
    extra = [
        a.split(":")[-1]
        for a in cmd.get("aliases", []) + cmd.get("hiddenAliases", [])
        if a.split(":")[:-1] == prefix and a.split(":")[-1] != name
    ]
    names = list(dict.fromkeys([name, *extra]))
    node: dict = {"name": names[0] if len(names) == 1 else names}
    node["description"] = tidy(cmd.get("summary") or cmd.get("description") or name)
    if cmd.get("state") == "deprecated" or cmd.get("deprecationOptions"):
        node["deprecated"] = True
    if options := flag_options(cmd_id, cmd["flags"]):
        node["options"] = options
    if args := positional_args(cmd_id, cmd):
        node["args"] = args[0] if len(args) == 1 else args
    if name in DANGEROUS:
        node["isDangerous"] = True
    return node


# Build the tree: topics become nodes, commands become leaves (a topic can also be a command)
root: dict = {"children": {}}
for cmd_id in sorted(commands):
    cmd = commands[cmd_id]
    if cmd.get("hidden") or cmd_id == "help":
        continue
    node = root
    for part in cmd_id.split(":"):
        node = node["children"].setdefault(part, {"children": {}})
    node["cmd"] = (cmd_id, cmd)


def to_spec(name: str, node: dict, path: list[str]) -> dict:
    topic_id = ":".join([*path, name])
    out = leaf(*node["cmd"]) if "cmd" in node else {"name": name}
    if not path and name in TOPIC_ALIASES:
        names = out["name"] if isinstance(out["name"], list) else [out["name"]]
        out["name"] = list(dict.fromkeys([*names, *TOPIC_ALIASES[name]]))
    subcommands = [to_spec(n, c, [*path, name]) for n, c in sorted(node["children"].items())]
    if "description" not in out:
        # oclif falls back to the only command's summary for a topic without a description
        only = subcommands[0]["description"] if len(subcommands) == 1 else name
        out["description"] = tidy(TOPICS.get(topic_id) or TOPIC_FALLBACKS.get(topic_id) or only)
    if subcommands:
        out["subcommands"] = subcommands
    return out


subcommands = [to_spec(n, c, []) for n, c in sorted(root["children"].items())]
subcommands.insert(
    0,
    {
        "name": "help",
        "description": "Display help for sanity",
        "args": {"name": "command", "isOptional": True, "isVariadic": True, "template": "help"},
    },
)

spec = {
    "name": "sanity",
    "description": tidy(cli_pkg["description"]),
    "subcommands": subcommands,
    "options": [
        {"name": ["--help", "-h"], "description": "Show help for sanity", "isPersistent": True},
        {"name": ["--version", "-v"], "description": "Show the installed version of the Sanity CLI"},
    ],
}


def emit(value) -> str:
    if isinstance(value, dict):
        return "{" + ", ".join(f"{k}: {emit(v)}" for k, v in value.items()) + "}"
    if isinstance(value, list):
        return "[" + ", ".join(emit(v) for v in value) + "]"
    return json.dumps(value, ensure_ascii=False)


plugins = ", ".join(f"{p['name']} {p['version']}" for p in (load(d)[2] for d in plugin_dirs))
print(
    f"// Generated by build-scripts/specs/gen_sanity.py from @sanity/cli {cli_pkg['version']}"
    f" ({plugins}); don't edit by hand"
)
print(f"const completionSpec: Fig.Spec = {emit(spec)};\n")
print("export default completionSpec;")
