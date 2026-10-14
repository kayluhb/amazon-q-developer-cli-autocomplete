"""Builds specs/shopify/4.0.0.ts from the oclif manifest that @shopify/cli publishes, which is
what `shopify <command> --help` is rendered from. To regenerate after a Shopify CLI release:

    cd "$(mktemp -d)" && npm pack @shopify/cli@latest && tar xzf shopify-cli-*.tgz && cd -
    python3 build-scripts/specs/gen_shopify.py <that dir>/package > specs/shopify/4.0.0.ts
    npx prettier --write specs/shopify/4.0.0.ts --parser typescript

Hidden commands, topics and flags are left out, as `--help` leaves them out. Icons, priorities,
templates and suggestion descriptions that the manifest can't tell us come from upstream's
src/shopify/3.0.0.ts and are kept in the tables below."""

import json
import re
import sys

package = sys.argv[1]
manifest = json.load(open(f"{package}/oclif.manifest.json"))
pkg = json.load(open(f"{package}/package.json"))
TOPICS = pkg["oclif"]["topics"]
VERSION = pkg["version"]

# Flags every command repeats; kept, but sorted below the command's own flags
GLOBAL_FLAGS = {"json-schema", "auth-alias", "no-input", "no-color", "verbose"}
OPTION_ICON = "fig://icon?type=option"

# Keyed by command path, e.g. "theme dev"
COMMAND_EXTRAS = {
    "theme": {"priority": 100},
    "theme dev": {"priority": 100},
    "theme init": {"priority": 75},
    "theme check": {"icon": "fig://icon?type=alert"},
    "theme package": {"icon": "fig://icon?type=box"},
    "app": {"priority": 90},
    "app dev": {"priority": 100},
    "app deploy": {"priority": 80},
    "version": {"priority": 30, "icon": OPTION_ICON},
    "upgrade": {"icon": OPTION_ICON},
}
# Keyed by "<command path> --flag"; merged into the option
OPTION_EXTRAS = {
    "theme init --clone-url": {"icon": "fig://icon?type=github"},
    "theme init --latest": {"icon": "fig://icon?type=github"},
}
# Keyed by "<command path> --flag" or "<command path> <arg>"; merged into its args
ARG_EXTRAS = {
    "theme check --config": {"template": "filepaths"},
}
SUGGESTION_DESCRIPTIONS = {
    "theme dev --live-reload": {
        "hot-reload": "Hot reloads local changes to CSS and sections (default)",
        "full-page": "Refreshes the entire page when a file is modified",
        "off": "Deactivate live reload",
    },
}
FOLDER_FLAGS = {"path", "assets-dir", "worker-dir", "include-dir"}
FILE_FLAGS = {"query-file", "variable-file", "output-file", "env-file", "codegen-config-path", "input", "export"}


def tidy(text: str) -> str:
    """First paragraph before any list, links reduced to their text, in the fig linter's style:
    capitalised, no trailing dot."""
    text = text.replace("<%= config.bin %>", "shopify").strip()
    text = re.split(r"\n\s*\n|\n\s*- ", text)[0]
    text = re.sub(r'"([^"]+)" \((https?://[^)]+)\)', r"\1", text)
    text = " ".join(text.split()).removesuffix(".").removesuffix(":")
    return text[:1].upper() + text[1:]


def visible_commands() -> dict[str, dict]:
    hidden_topics = {t for t, v in TOPICS.items() if v.get("hidden")}
    return {
        cid: c
        for cid, c in manifest["commands"].items()
        if not c.get("hidden") and not any(cid == t or cid.startswith(t + ":") for t in hidden_topics)
    }


def option(flag: dict, path: str) -> list[dict]:
    names = [f"--{flag['name']}"] + ([f"-{flag['char']}"] if flag.get("char") else [])
    out: dict = {"name": names[0] if len(names) == 1 else names}
    if desc := tidy(flag.get("description") or flag.get("summary") or ""):
        out["description"] = desc
    key = f"{path} --{flag['name']}"
    if flag["type"] == "option":
        args: dict = {"name": flag["name"]}
        if values := flag.get("options"):
            descs = SUGGESTION_DESCRIPTIONS.get(key, {})
            args["suggestions"] = [{"name": v, "description": descs[v]} if v in descs else v for v in values]
        elif flag["name"] in FOLDER_FLAGS:
            args["template"] = "folders"
        elif flag["name"] in FILE_FLAGS:
            args["template"] = "filepaths"
        args.update(ARG_EXTRAS.get(key, {}))
        out["args"] = args
        if flag.get("multiple"):
            out["isRepeatable"] = True
    if flag.get("required"):
        out["isRequired"] = True
    if flag.get("exclusive"):
        out["exclusiveOn"] = [f"--{n}" for n in flag["exclusive"]]
    if flag.get("dependsOn"):
        out["dependsOn"] = [f"--{n}" for n in flag["dependsOn"]]
    if flag.get("deprecated"):
        out["deprecated"] = True
    if flag["name"] in GLOBAL_FLAGS:
        out["priority"] = 1
    out.update(OPTION_EXTRAS.get(key, {}))
    options = [out]
    if flag.get("allowNo"):
        options.append({"name": f"--no-{flag['name']}", "description": f"Negates --{flag['name']}"})
    return options


def positional(name: str, arg: dict, path: str) -> dict:
    out: dict = {"name": name}
    if desc := tidy(arg.get("description") or ""):
        out["description"] = desc
    if not arg.get("required"):
        out["isOptional"] = True
    if values := arg.get("options"):
        out["suggestions"] = values
    out.update(ARG_EXTRAS.get(f"{path} {name}", {}))
    return out


def command_node(cmd: dict, path: str) -> dict:
    out: dict = {}
    if desc := tidy(cmd.get("summary") or cmd.get("description") or ""):
        out["description"] = desc
    options = [o for f in cmd["flags"].values() if not f.get("hidden") and f["name"] != "help" for o in option(f, path)]
    # Command-specific flags first, then the ones every command shares
    options.sort(key=lambda o: o.get("priority") == 1)
    if options:
        out["options"] = options
    args = [positional(n, a, path) for n, a in (cmd.get("args") or {}).items()]
    if args:
        if not cmd.get("strict", True):
            args[-1]["isVariadic"] = True
        out["args"] = args[0] if len(args) == 1 else args
    return out


def build_tree(commands: dict[str, dict]) -> dict:
    root: dict = {"children": {}}
    for cid in commands:
        node = root
        for part in cid.split(":"):
            node = node["children"].setdefault(part, {"children": {}})
        node["command"] = commands[cid]
    return root


def first_description(node: dict) -> str:
    if "command" in node:
        return tidy(node["command"].get("summary") or node["command"].get("description") or "")
    return next((d for c in node["children"].values() if (d := first_description(c))), "")


def subcommand(name: str, node: dict, path: list[str]) -> dict:
    key = " ".join(path)
    out: dict = {"name": name}
    body = command_node(node["command"], key) if "command" in node else {}
    topic = TOPICS.get(":".join(path), {})
    # Like oclif's help: a topic's own description, else its command's, else its first child's
    out["description"] = tidy(topic["description"]) if topic.get("description") else first_description(node)
    body.pop("description", None)
    if node["children"]:
        out["subcommands"] = [subcommand(n, c, [*path, n]) for n, c in sorted(node["children"].items())]
        if "command" not in node:
            out["requiresSubcommand"] = True
    out.update(body)
    out.update(COMMAND_EXTRAS.get(key, {}))
    if not out["description"]:
        del out["description"]
    return out


def emit(value) -> str:
    if isinstance(value, dict):
        return "{" + ", ".join(f"{k}: {emit(v)}" for k, v in value.items()) + "}"
    if isinstance(value, list):
        return "[" + ", ".join(emit(v) for v in value) + "]"
    return json.dumps(value, ensure_ascii=False)


tree = build_tree(visible_commands())
spec = {
    "name": "shopify",
    "description": tidy(pkg["description"]),
    "icon": "https://avatars.githubusercontent.com/u/8085?s=16&v=4",
    "options": [
        {
            "name": ["--help", "-h"],
            "icon": OPTION_ICON,
            "description": "Show help for this command",
            "priority": 1,
            "isPersistent": True,
        },
    ],
    "subcommands": [subcommand(n, c, [n]) for n, c in sorted(tree["children"].items())],
}

print(f"// Generated by build-scripts/specs/gen_shopify.py from @shopify/cli {VERSION}; don't edit by hand")
print(f"const completion: Fig.Subcommand = {emit(spec)};\n")
print("const versions: Fig.VersionDiffMap = {};\n")
print('versions["4.0.0"] = {};\n')
print("export { versions };")
print("export default completion;")
