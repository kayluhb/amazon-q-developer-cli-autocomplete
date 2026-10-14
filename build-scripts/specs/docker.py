"""Builds specs/docker.ts from the installed docker CLI's help output, including the CLI plugins it
lists (buildx, scout, ...). To regenerate after a docker upgrade:

    git -C build/autocomplete-specs show HEAD:src/docker.ts > /tmp/docker.upstream.ts
    python3 build-scripts/specs/docker.py crawl > /tmp/docker-tree.json
    python3 build-scripts/specs/docker.py gen /tmp/docker-tree.json /tmp/docker.upstream.ts > specs/docker.ts
    npx prettier --write specs/docker.ts --parser typescript

The crawl only runs `docker <command> --help` and `docker --version`. Plugins differ between
installs (most come with Docker Desktop), so check the plugin list in the generated header.
Dynamic suggestions reuse upstream's hand-written generators, which are copied over verbatim;
`compose` keeps loading withfig's separate docker-compose spec."""

import json
import os
import re
import subprocess
import sys

from _common import Raw, emit
from _common import tidy as base_tidy

# Plugins that get their own spec, or whose help isn't about docker commands
NOT_CRAWLED = {"compose"}
SKIP_COMMANDS = {"help", "completion", "docker-cli-plugin-hooks", "__complete", "__completeNoDesc"}
HEADERS = {
    "Usage",
    "Aliases",
    "Options",
    "Global Options",
    "Flags",
    "Global Flags",
    "Examples",
    "Description",
    "Commands",
    "Management Commands",
    "Common Commands",
    "Swarm Commands",
    "Available Commands",
}
FLAG_RE = re.compile(r"^(\s+)(?:(-\w), )?(--[\w.-]+)(?: (\S+))?(?:\s{2,}(.*))?$")
COMMAND_RE = re.compile(r"^\s+([\w-]+)(\*?)(?:\s{2,}(.*))?$")


# ---------------------------------------------------------------------------------------- crawl


def help_text(path: list[str], via_help: bool = False) -> str:
    cmd = ["docker", "help", *path] if via_help else ["docker", *path, "--help"]
    out = subprocess.run(cmd, capture_output=True, text=True, stdin=subprocess.DEVNULL, timeout=30)
    return out.stdout + out.stderr


def usage_of(sec: dict[str, list[str]], path: list[str]) -> str:
    return next((u.strip() for u in sec.get("Usage", []) if u.strip()), "docker " + " ".join(path))


def sections(text: str) -> dict[str, list[str]]:
    """Split help output into {Header: lines}; text before the first header goes under ''."""
    out: dict[str, list[str]] = {"": []}
    current = ""
    for line in text.splitlines():
        head, _, rest = line.partition(":")
        if line.rstrip() in HEADERS or (head in HEADERS and not line.startswith(" ")):
            current = line.rstrip().rstrip(":") if line.rstrip().rstrip(":") in HEADERS else head
            out.setdefault(current, [])
            if head in HEADERS and rest.strip():
                out[current].append(rest.strip())
        else:
            out[current].append(line)
    return out


def parse_flags(lines: list[str]) -> list[dict]:
    flags: list[dict] = []
    indent = 0
    for line in lines:
        m = FLAG_RE.match(line)
        if m and (not flags or len(m.group(1)) <= indent):
            pad, short, long, arg, desc = m.groups()
            indent = len(pad) + 4
            flags.append({"short": short, "long": long, "arg": arg, "description": (desc or "").strip()})
        elif flags and line.strip() and line[0].isspace():
            flags[-1]["description"] += " " + line.strip()
    return flags


def crawl(path: list[str], description: str) -> dict:
    sec = sections(help_text(path))
    usage = usage_of(sec, path)
    if path and path[-1] not in usage.split() and usage.split()[1:2] in (["[OPTIONS]"], ["COMMAND"]):
        # `docker swarm init --help` shows the init plugin's help instead; `docker help` doesn't
        sec = sections(help_text(path, via_help=True))
        usage = usage_of(sec, path)
    aliases = []
    for line in sec.get("Aliases", []):
        for alias in line.split(","):
            words = alias.split()
            # `docker container ls, docker container list, docker ps`: only siblings are aliases here
            if words and words[:-1] == ["docker", *path[:-1]] and words[-1] != path[-1]:
                aliases.append(words[-1])
    node = {
        "name": path[-1] if path else "docker",
        "aliases": aliases,
        "description": description,
        "usage": usage,
        "flags": parse_flags(sec.get("Options", []) + sec.get("Flags", []) + sec.get("Global Options", [])),
        "subcommands": [],
    }
    if not path:
        node["description"] = next(line.strip() for line in sec["Usage"][1:] if line.strip())
    if path and path[-1] not in usage.split() and usage.split()[1:2] in (["[OPTIONS]"], ["COMMAND"]):
        # still the root's help; `docker builder` showing `docker buildx`'s help is fine, it runs buildx
        print(f"skipping docker {' '.join(path)}: help is for {usage!r}", file=sys.stderr)
        node["usage"] = "docker " + " ".join(path)
        node["flags"] = []
        return node
    seen = set()
    for header, lines in sec.items():
        if not header.endswith("Commands"):
            continue
        for line in lines:
            m = COMMAND_RE.match(line)
            if not m or m.group(1) in SKIP_COMMANDS or m.group(1) in seen:
                continue
            name, plugin, desc = m.groups()
            seen.add(name)
            if name in NOT_CRAWLED:
                child = {"name": name, "description": desc or "", "notCrawled": True}
            else:
                child = crawl([*path, name], (desc or "").strip())
            child["plugin"] = bool(plugin)
            node["subcommands"].append(child)
    node["subcommands"].sort(key=lambda c: c["name"])
    return node


# ------------------------------------------------------------------------------------------ gen


def balanced(src: str, start: int) -> str:
    """The {...} or [...] literal starting at src[start]."""
    depth = 0
    for i in range(start, len(src)):
        if src[i] in "{[":
            depth += 1
        elif src[i] in "}]":
            depth -= 1
            if depth == 0:
                return src[start : i + 1]
    raise ValueError("unbalanced literal")


def upstream_generator(upstream: str, marker: str) -> str:
    """Upstream's inline `generators: ...` literal for the arg identified by `marker`."""
    at = upstream.index("generators:", upstream.index(marker)) + len("generators:")
    return balanced(upstream, at + len(upstream[at:]) - len(upstream[at:].lstrip()))


G = "dockerGenerators."
# Positional args by their usage name (after splitting alternatives on `|`), as upstream had them
PLACEHOLDERS = {
    "CONTAINER": {"generators": Raw(G + "runningDockerContainers")},
    "IMAGE": {"generators": Raw(G + "allLocalImages")},
    "NETWORK": {"generators": Raw(G + "listDockerNetworks")},
    "NODE": {"generators": Raw(G + "listDockerSwarmNodes")},
    "PLUGIN": {"generators": Raw(G + "listDockerPlugins")},
    "SECRET": {"generators": Raw(G + "listDockerSecrets")},
    "SERVICE": {"generators": Raw(G + "listDockerServices")},
    "SERVICE=REPLICAS": {"generators": Raw(G + "listDockerServicesReplicas")},
    "STACK": {"generators": Raw(G + "listDockerStacks")},
    "VOLUME": {"generators": Raw(G + "listDockerVolumes")},
    "CONTEXT": {"generators": Raw(G + "allDockerContexts")},
    "COMMAND": {"isCommand": True},
    "FILE": {"template": "filepaths"},
    "PATH": {"template": "folders"},
    "DIRECTORY": {"template": "folders"},
    "FOLDER": {"template": "folders"},
    "PLUGIN-DATA-DIR": {"template": "folders"},
}
NEW_NAMES = {"NETWORK", "VOLUME", "CONTEXT", "PLUGIN", "SECRET", "CONFIG"}
# Completions keyed by "<command path> <--flag or positional usage name>"
OVERRIDES = {
    "run IMAGE": {"generators": Raw("runImages")},
    "create IMAGE": {"generators": Raw("runImages")},
    "pull NAME[:TAG|@DIGEST]": {"generators": Raw(G + "dockerHubSearch")},
    "inspect NAME|ID": {"generators": Raw("dockerObjects")},
    "sbom IMAGE": {"generators": Raw(G + "allLocalImagesWithRepository")},
    "start CONTAINER": {"generators": Raw(G + "allDockerContainers")},
    "rm CONTAINER": {"generators": Raw(G + "allDockerContainers")},
    "unpause CONTAINER": {"generators": Raw(G + "pausedDockerContainers")},
    "build --file": {"template": "filepaths"},
    "build --target": {"generators": Raw("dockerfileTargets")},
    "run --log-driver": {
        "suggestions": [
            "json-file",
            "syslog",
            "journald",
            "gelf",
            "fluentd",
            "awslogs",
            "splunk",
            "etwlogs",
            "gcplogs",
            "none",
        ]
    },
    "version --format": {"suggestions": ["pretty", "json"]},
    "context ls --format": {"suggestions": ["pretty", "json"]},
    "service create --mode": {"suggestions": ["replicated", "global", "replicated-job", "global-job"]},
    # global options
    "--context": {"generators": Raw(G + "allDockerContexts")},
    "--config": {"template": "folders"},
    "--tlscacert": {"template": "filepaths"},
    "--tlscert": {"template": "filepaths"},
    "--tlskey": {"template": "filepaths"},
}
OVERRIDES["create --log-driver"] = OVERRIDES["run --log-driver"]
# Commands reachable under several paths share overrides: `docker run` is `docker container run`
SAME_AS = {
    ("container",): (),
    ("image",): (),
    ("builder",): (),
    ("buildx",): (),
    ("container", "ls"): ("ps",),
    ("image", "ls"): ("images",),
    ("image", "rm"): ("rmi",),
}
FILE_FLAGS = {
    "--env-file",
    "--label-file",
    "--cidfile",
    "--iidfile",
    "--compose-file",
    "--ca-cert",
    "--ca-key",
    "--cert",
    "--key",
    "--output",
    "--metadata-file",
    "--input",
    "--secret-file",
    "--config-file",
}
# Args missing from a plugin's usage line
EXTRA_ARGS = {
    "sbom": {"name": "IMAGE", "generators": Raw(G + "allLocalImagesWithRepository")},
    "debug": {"name": "CONTAINER|IMAGE", "generators": [Raw(G + "allDockerContainers"), Raw(G + "allLocalImages")]},
}
# Upstream's shorthand for the common `-i -t` pair
INTERACTIVE = {"name": "-it", "description": "Launch an interactive session", "icon": "fig://icon?type=commandkey"}


def lookup_paths(path: list[str]) -> list[str]:
    """`container run` also matches overrides for `run`, `image ls` for `images`, ..."""
    out = [" ".join(path)]
    for prefix, repl in SAME_AS.items():
        if tuple(path[: len(prefix)]) == prefix and len(path) > len(prefix):
            out.append(" ".join([*repl, *path[len(prefix) :]]))
        if tuple(path) == prefix and repl:
            out.append(" ".join(repl))
    return out


def override(path: list[str], key: str) -> dict | None:
    for p in lookup_paths(path):
        if (o := OVERRIDES.get(f"{p} {key}".strip())) is not None:
            return o
    return None


def tidy(description: str) -> str:
    """The shared tidy, also without a trailing colon."""
    return base_tidy(description).removesuffix(":")


def choices(description: str) -> list[str] | None:
    # ("auto", "quiet", "plain") / (worker|manager) / options=[a b c]
    if m := re.search(r'\(((?:"[\w.-]+"(?:, | or |\|)?){2,})\)', description):
        return re.findall(r'"([\w.-]+)"', m.group(1))
    if m := re.search(r"options=\[([\w. -]+)\]", description):
        return m.group(1).split()
    return None


def option(flag: dict, path: list[str]) -> dict:
    names = [n for n in (flag["short"], flag["long"]) if n]
    desc = flag["description"]
    if ":" in desc and ("'table'" in desc or "'json'" in desc):
        desc = desc.split(":")[0]  # --format's multi-line template list
    desc = re.sub(r"\s*\(default (?:map\[\]|\[\])\)", "", desc)
    # defaults that describe this machine: ~/.docker paths, the active builder
    if os.path.expanduser("~") in desc or flag["long"] == "--builder":
        desc = re.sub(r"\s*\(default [^)]*\)", "", desc)
    out: dict = {"name": names[0] if len(names) == 1 else names, "description": tidy(desc)}
    if arg := flag["arg"]:
        args: dict = {"name": arg}
        if (o := override(path, flag["long"])) is not None:
            args.update(o)
        elif values := choices(flag["description"]):
            args["suggestions"] = values
        elif flag["long"] in FILE_FLAGS or flag["long"].endswith("-file"):
            args["template"] = "filepaths"
        out["args"] = args
        if arg in {
            "list",
            "strings",
            "stringArray",
            "filter",
            "map",
            "ulimit",
            "mount",
            "network",
            "pref",
            "external-ca",
            "gpu-request",
            "weight-device",
            "throttle-device",
            "command",
        }:
            out["isRepeatable"] = True
    return out


def tokens(usage: str) -> list[str]:
    """Split usage into top-level groups, keeping [...] / (...) together and `A | B` as one."""
    usage = re.sub(r"\s*\|\s*", "|", usage)
    out, depth, cur = [], 0, ""
    for ch in usage:
        if ch in "[(":
            depth += 1
        elif ch in "])":
            depth -= 1
        if ch == " " and depth == 0:
            out.append(cur)
            cur = ""
        else:
            cur += ch
    return [t for t in out + [cur] if t]


def positional_args(node: dict, path: list[str]) -> list[dict]:
    toks = tokens(node["usage"].split(". ")[0])
    # skip `docker <path>`; plugins' usage may abbreviate their own path
    while toks and not re.search(r"[A-Z\[(]", toks[0]):
        toks.pop(0)
    args: list[dict] = []
    skip_next = False
    for tok in toks:
        if skip_next:  # a flag's value in the usage line, e.g. `--to IMAGE`
            skip_next = False
            continue
        if tok.startswith("--"):
            skip_next = "=" not in tok
            continue
        if tok in {"[OPTIONS]", "[OPTIONS...]", "[flags]", "[command]", "COMMAND"} and not args and node["subcommands"]:
            continue
        if tok in {"[OPTIONS]", "OPTIONS", "[flags]", "-"} or tok.startswith("-"):
            continue
        inner = tok[1:-1] if tok.startswith("[") and tok.endswith("]") else tok
        variadic = inner.endswith("...")
        inner = inner.removesuffix("...")
        if args and inner.split("[")[0] == args[-1]["name"].split("[")[0] and variadic:
            args[-1]["isVariadic"] = True  # CONTAINER [CONTAINER...]
            continue
        inner = inner.strip("<>")
        if re.fullmatch(r"\(([a-z-]+\|)+[a-z-]+\)", inner):  # (worker|manager)
            arg: dict = {"name": inner, "suggestions": inner.strip("()").split("|")}
        else:
            arg = {"name": inner}
        if tok.startswith("["):
            arg["isOptional"] = True
        if variadic:
            arg["isVariadic"] = True
        if "suggestions" not in arg:
            arg.update(positional_completion(inner, path))
        args.append(arg)
    # `IMAGE [COMMAND] [ARG...]`: everything after the command belongs to it
    command_at = next((i for i, a in enumerate(args) if a.get("isCommand")), len(args))
    return args[: command_at + 1]


def positional_completion(name: str, path: list[str]) -> dict:
    if (o := override(path, name)) is not None:
        return o
    alternatives = [re.sub(r"\[.*", "", a).upper() for a in name.split("|")]
    for alt in alternatives:
        if path[-1:] == ["create"] and alt in NEW_NAMES:
            continue  # the name of the thing being created
        if (p := PLACEHOLDERS.get(alt)) is not None or (p := PLACEHOLDERS.get(alt.rstrip("S"))) is not None:
            return p
    return {}


def command(node: dict, path: list[str]) -> dict:
    names = list(dict.fromkeys([node["name"], *node.get("aliases", [])]))
    out: dict = {"name": names[0] if len(names) == 1 else names, "description": tidy(node["description"])}
    if node.get("notCrawled"):
        out["loadSpec"] = "docker-compose"
        return out
    if node["subcommands"]:
        out["subcommands"] = [command(c, [*path, c["name"]]) for c in node["subcommands"]]
    flags = [f for f in node["flags"] if f["long"] != "--help"]
    options = [option(f, path) for f in flags]
    if any(lp.split()[-1:] in (["run"], ["exec"]) for lp in lookup_paths(path)):
        options.insert(0, dict(INTERACTIVE))
    if options:
        out["options"] = options
    if (extra := EXTRA_ARGS.get(" ".join(path))) is not None:
        out["args"] = extra
    elif args := positional_args(node, path):
        out["args"] = args[0] if len(args) == 1 else args
    return out


def gen(tree: dict, upstream: str) -> str:
    # Upstream's helpers and dockerGenerators, everything before its command definitions
    prelude = upstream[: upstream.index("const containersArg")]
    extra = {
        "runImages": upstream_generator(upstream, 'description: "The Docker image to use"'),
        "dockerObjects": upstream_generator(upstream, 'name: "Name or ID"'),
        "dockerfileTargets": upstream_generator(upstream, 'name: "target build stage"'),
    }
    spec = command(tree, [])
    version = subprocess.run(
        ["docker", "--version"], capture_output=True, text=True, stdin=subprocess.DEVNULL, timeout=30
    ).stdout.strip()
    plugins = ", ".join(c["name"] for c in tree["subcommands"] if c.get("plugin"))
    lines = [
        f"// Generated by build-scripts/specs/docker.py from {version} (plugins: {plugins}); don't edit by hand",
        prelude,
    ]
    lines += [
        f"const {name}: Fig.Generator{'[]' if text.startswith('[') else ''} = {text};\n" for name, text in extra.items()
    ]
    lines += [f"const completionSpec: Fig.Spec = {emit(spec)};\n", "export default completionSpec;"]
    return "\n".join(lines)


if __name__ == "__main__":
    if sys.argv[1] == "crawl":
        print(json.dumps(crawl([], ""), indent=1))
    else:
        print(gen(json.load(open(sys.argv[2])), open(sys.argv[3]).read()))
