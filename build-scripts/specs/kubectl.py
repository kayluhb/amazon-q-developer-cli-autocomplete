"""Builds specs/kubectl.ts from kubectl's help output. To regenerate after a kubectl release:

    git -C build/autocomplete-specs show HEAD:src/kubectl.ts > /tmp/kubectl.upstream.ts
    python3 build-scripts/specs/kubectl.py [path/to/kubectl] /tmp/kubectl.upstream.ts > specs/kubectl.ts
    cp specs/kubectl.ts build/autocomplete-specs/src/kubectl.ts
    (cd build/autocomplete-specs && npx prettier --write src/kubectl.ts)  # withfig's prettier config
    cp build/autocomplete-specs/src/kubectl.ts specs/kubectl.ts

It walks `kubectl <path> --help` for every command (plus `kubectl options` for the global flags).
Upstream's hand-written generators are copied over verbatim; positional args can't be parsed
reliably from kubectl's free-form usage lines, so they come from the ARGS table below."""

import json
import re
import subprocess
import sys

KUBECTL = sys.argv[1] if len(sys.argv) > 2 else "kubectl"
upstream = open(sys.argv[-1]).read()

# Upstream's helpers and shared args, everything before its shared options
PRELUDE = upstream[: upstream.index("const sharedOpts")]


class Raw(str):
    """Emitted as-is instead of as a string literal."""


def help_text(path: list[str]) -> str:
    result = subprocess.run(
        [KUBECTL, *path, "--help"], capture_output=True, text=True, stdin=subprocess.DEVNULL, timeout=20
    )
    return result.stdout


def sections(text: str) -> dict[str, list[str]]:
    """Split help output into {Header: lines}; text before the first header goes under ''."""
    out: dict[str, list[str]] = {"": []}
    current = ""
    for line in text.splitlines():
        if re.fullmatch(r"[A-Z][\w ()-]*:", line):
            current = line[:-1]
            out[current] = []
        else:
            out[current].append(line)
    return out


def parse_flags(lines: list[str]) -> list[dict]:
    """Flags look like `    -f, --filename=[]:` followed by tab-indented description lines."""
    flags: list[dict] = []
    for line in lines:
        m = re.match(r"^\s+(?:(-\w), )?(--[\w-]+)=(.*):$", line)
        if m:
            short, long, default = m.groups()
            flags.append({"short": short, "long": long, "default": default, "description": ""})
        elif line.startswith("\t") and flags:
            flags[-1]["description"] = (flags[-1]["description"] + " " + line.strip()).strip()
    return flags


def crawl(path: list[str], description: str) -> dict:
    s = sections(help_text(path))
    node = {
        "name": path[-1] if path else "kubectl",
        "description": description,
        "aliases": [a.strip() for a in ",".join(s.get("Aliases", [])).split(",") if a.strip()],
        "flags": parse_flags(s.get("Options", [])),
        "subcommands": [],
    }
    for header, lines in s.items():
        if "Commands" not in header or "plugins" in header:
            continue
        for line in lines:
            m = re.match(r"^  ([\w-]+)\s{2,}(.*)$", line)
            if m:
                print(" ".join([*path, m.group(1)]), file=sys.stderr)
                node["subcommands"].append(crawl([*path, m.group(1)], m.group(2).strip()))
    return node


def global_flags() -> list[dict]:
    out = subprocess.run(
        [KUBECTL, "options"], capture_output=True, text=True, stdin=subprocess.DEVNULL, timeout=20
    ).stdout
    return parse_flags(out.splitlines())


# Positional args by command path. Names refer to upstream's sharedArgs or are literal args.
TYPE_NAME = [Raw("sharedArgs.typeOrTypeSlashName"), Raw("sharedArgs.resourceSuggestionsFromResourceType")]
TYPE_NAME_OPT = [
    Raw("{ ...sharedArgs.typeOrTypeSlashName, isOptional: true }"),
    Raw("sharedArgs.resourceSuggestionsFromResourceType"),
]
DEPLOYMENTS = Raw("sharedArgs.listDeployments")
NODES = Raw("sharedArgs.listNodes")
CONTEXT = Raw("sharedArgs.listKubeConfContexts")
CLUSTER = Raw("sharedArgs.listClusters")
NAME = {"name": "NAME"}
VERBS = ["*", "get", "list", "watch", "delete", "create", "update", "patch"]
ARGS: dict[str, object] = {
    "annotate": [*TYPE_NAME, {"name": "KEY=VAL", "isVariadic": True}],
    "apply edit-last-applied": TYPE_NAME,
    "apply view-last-applied": TYPE_NAME,
    "attach": Raw("sharedArgs.runningPodsArg"),
    "auth can-i": [{"name": "VERB", "suggestions": VERBS}, *TYPE_NAME],
    "autoscale": TYPE_NAME,
    "certificate approve": {"name": "NAME", "isVariadic": True},
    "certificate deny": {"name": "NAME", "isVariadic": True},
    "completion": {"name": "SHELL", "suggestions": ["bash", "zsh", "fish", "powershell"]},
    "config delete-cluster": CLUSTER,
    "config delete-context": CONTEXT,
    "config delete-user": {"name": "NAME"},
    "config get-contexts": Raw("{ ...sharedArgs.listKubeConfContexts, isOptional: true }"),
    "config rename-context": [CONTEXT, {"name": "New Context Name"}],
    "config set": [{"name": "PROPERTY_NAME"}, {"name": "PROPERTY_VALUE"}],
    "config set-cluster": Raw("sharedArgs.listClusters"),
    "config set-context": Raw("{ ...sharedArgs.listKubeConfContexts, isOptional: true }"),
    "config set-credentials": NAME,
    "config unset": {"name": "PROPERTY_NAME"},
    "config use-context": CONTEXT,
    "cordon": NODES,
    "cp": [{"name": "SRC", "template": "filepaths"}, {"name": "DEST", "template": "filepaths"}],
    "create clusterrole": NAME,
    "create clusterrolebinding": NAME,
    "create configmap": NAME,
    "create cronjob": NAME,
    "create deployment": NAME,
    "create ingress": NAME,
    "create job": [NAME, {"name": "COMMAND", "isOptional": True, "isVariadic": True}],
    "create namespace": NAME,
    "create poddisruptionbudget": NAME,
    "create priorityclass": NAME,
    "create quota": NAME,
    "create role": NAME,
    "create rolebinding": NAME,
    "create secret docker-registry": NAME,
    "create secret generic": NAME,
    "create secret tls": NAME,
    "create service clusterip": NAME,
    "create service externalname": NAME,
    "create service loadbalancer": NAME,
    "create service nodeport": NAME,
    "create serviceaccount": NAME,
    "create token": {"name": "SERVICE_ACCOUNT_NAME"},
    "debug": TYPE_NAME_OPT,
    "delete": TYPE_NAME_OPT,
    "describe": TYPE_NAME_OPT,
    "drain": NODES,
    "edit": TYPE_NAME_OPT,
    "events": {"name": "TYPE/NAME", "isOptional": True},
    "exec": [Raw("sharedArgs.runningPodsArg"), {"name": "COMMAND", "isOptional": True, "isVariadic": True}],
    "explain": Raw("sharedArgs.resourcesArg"),
    "expose": TYPE_NAME_OPT,
    "get": TYPE_NAME_OPT,
    "kustomize": {"name": "DIR", "template": "folders", "isOptional": True},
    "label": [*TYPE_NAME, {"name": "KEY=VAL", "isVariadic": True}],
    "logs": TYPE_NAME,
    "patch": TYPE_NAME_OPT,
    "port-forward": [*TYPE_NAME, {"name": "[LOCAL_PORT:]REMOTE_PORT", "isVariadic": True}],
    "rollout history": DEPLOYMENTS,
    "rollout pause": DEPLOYMENTS,
    "rollout restart": DEPLOYMENTS,
    "rollout resume": DEPLOYMENTS,
    "rollout status": DEPLOYMENTS,
    "rollout undo": DEPLOYMENTS,
    "run": [NAME, {"name": "COMMAND", "isOptional": True, "isVariadic": True}],
    "scale": TYPE_NAME_OPT,
    "set env": [*TYPE_NAME_OPT, {"name": "KEY=VALUE", "isOptional": True, "isVariadic": True}],
    "set image": [*TYPE_NAME_OPT, {"name": "CONTAINER_NAME=IMAGE_NAME", "isVariadic": True}],
    "set resources": TYPE_NAME_OPT,
    "set selector": [*TYPE_NAME_OPT, {"name": "EXPRESSIONS"}],
    "set serviceaccount": [*TYPE_NAME_OPT, {"name": "SERVICE_ACCOUNT"}],
    "set subject": TYPE_NAME_OPT,
    "taint": [NODES, {"name": "KEY_1=VAL_1:TAINT_EFFECT_1", "isVariadic": True}],
    "top node": Raw("{ ...sharedArgs.listNodes, isOptional: true }"),
    "top pod": {"name": "NAME", "isOptional": True},
    "uncordon": NODES,
    "wait": TYPE_NAME_OPT,
}

CREATE_JOB_FROM = """{
  name: "Cronjob",
  generators: {
    script: () => scripts.typeWithoutName("cronjob"),
    postProcess: (out, tokens) => {
      return sharedPostProcess(out, tokens).map((item) => ({
        ...item,
        name: `cronjob/${item.name}`,
      })) as Fig.Suggestion[];
    },
  },
}"""
ROLES = """{
  name: "Role",
  generators: {
    script: () => scripts.typeWithoutName("roles"),
    postProcess: sharedPostProcess,
  },
}"""
NAMESPACES = """{
  script: scripts.typeWithoutName("namespaces"),
  postProcess: sharedPostProcess,
  cache: { strategy: "stale-while-revalidate", ttl: 1000 * 60 * 60 },
}"""

# Option args the help text can't tell us about, keyed by "<command path> <--flag>"; a bare
# "--flag" key applies to every command.
OPTION_ARGS: dict[str, dict] = {
    "--namespace": {"generators": Raw(NAMESPACES)},
    "--context": {"generators": Raw("sharedArgs.listKubeConfContexts.generators")},
    "--cluster": {"generators": Raw("sharedArgs.listClusters.generators")},
    "--container": {"generators": Raw("sharedArgs.listContainersFromPod.generators")},
    "--kubeconfig": {"template": "filepaths"},
    "--kuberc": {"template": "filepaths"},
    "--certificate-authority": {"template": "filepaths"},
    "--client-certificate": {"template": "filepaths"},
    "--client-key": {"template": "filepaths"},
    "--cache-dir": {"template": "folders"},
    "--filename": {"template": "filepaths"},
    "--kustomize": {"template": "folders"},
    "--from-file": {"template": "filepaths"},
    "--from-env-file": {"template": "filepaths"},
    "--env-file": {"template": "filepaths"},
    "--custom": {"template": "filepaths"},
    "--cert": {"template": "filepaths"},
    "--key": {"template": "filepaths"},
    "--output-directory": {"template": "folders"},
    "--patch-file": {"template": "filepaths"},
    "--cascade": {"suggestions": ["background", "orphan", "foreground"]},
    "--image-pull-policy": {"suggestions": ["Always", "IfNotPresent", "Never"]},
    "--resource": {"generators": Raw("sharedArgs.resourcesArg.generators")},
    "--clusterrole": {"generators": Raw("sharedArgs.listClusterRoles.generators")},
    "create job --from": Raw(CREATE_JOB_FROM),
    "create rolebinding --role": Raw(ROLES),
    "create role --verb": {"suggestions": VERBS},
    "create clusterrole --verb": {"suggestions": VERBS},
    "create cronjob --restart": {"suggestions": ["OnFailure", "Never"]},
    "create service clusterip --clusterip": {"suggestions": ["None"]},
    "run --restart": {"suggestions": ["Always", "OnFailure", "Never"]},
    "expose --protocol": {"suggestions": ["TCP", "UDP", "SCTP"]},
    "expose --type": {"suggestions": ["ClusterIP", "NodePort", "LoadBalancer", "ExternalName"]},
    "patch --type": {"suggestions": ["json", "merge", "strategic"]},
    "debug --profile": {"suggestions": ["legacy", "general", "baseline", "netadmin", "restricted", "sysadmin"]},
    "create token --bound-object-kind": {"suggestions": ["Node", "Pod", "Secret"]},
    "kuberc set --section": {"suggestions": ["defaults", "aliases", "credentialplugin"]},
    "completion --shell": {"suggestions": ["bash", "zsh", "fish", "powershell"]},
}


def tidy(description: str) -> str:
    """Match the fig linter's description style: capitalised, no trailing dot."""
    description = description.strip().rstrip(".")
    return description[:1].upper() + description[1:]


def choices(description: str) -> list[str] | None:
    """Values listed in a flag's description, e.g. `One of: (a, b, c)`, `either 'a' or 'b'`,
    `Must be "a", "b", or "c"`."""
    if m := re.search(r"[Oo]ne of:? \(([^)]+)\)", description):
        return [c.strip() for c in re.split(r"[,|]", m.group(1)) if c.strip()]
    if m := re.search(r"(?:either|[Oo]ne of) ('[\w-]+'(?:, '[\w-]+')*,? or '[\w-]+')", description):
        return re.findall(r"'([\w-]+)'", m.group(1))
    if m := re.search(r"One of: ([\w-]+(?:\|[\w-]+)+)", description):
        return m.group(1).split("|")
    if m := re.search(r'Must be (?:one of )?((?:"[\w-]+",? )+or "[\w-]+")', description):
        return re.findall(r'"([\w-]+)"', m.group(1))
    if m := re.search(r"Must be one of: ((?:[\w-]+(?: \([^)]*\))?, )+[\w-]+)", description):
        return [re.sub(r"\s*\(.*\)", "", c).strip() for c in m.group(1).split(",")]
    return None


def is_bool(flag: dict) -> bool:
    return flag["default"] in {"true", "false"}


def option(flag: dict, path: list[str], persistent: bool = False) -> dict:
    names = [n for n in (flag["short"], flag["long"]) if n]
    out: dict = {"name": names[0] if len(names) == 1 else names, "description": tidy(flag["description"])}
    if persistent:
        out["isPersistent"] = True
    if is_bool(flag):
        return out
    override = OPTION_ARGS.get(" ".join([*path, flag["long"]])) or OPTION_ARGS.get(flag["long"])
    if isinstance(override, Raw):
        out["args"] = override
    else:
        name = flag["long"].lstrip("-")
        args: dict = {"name": name}
        if override:
            args.update(override)
        elif values := choices(flag["description"]):
            args["suggestions"] = values
        default = flag["default"].strip("'")
        if default and default != "[]" and flag["long"] != "--cache-dir":
            args["default"] = default
        out["args"] = args
    if flag["default"].startswith("["):
        out["isRepeatable"] = True
    return out


def command(node: dict, path: list[str]) -> dict:
    names = list(dict.fromkeys([node["name"], *node["aliases"]]))
    out: dict = {"name": names[0] if len(names) == 1 else names, "description": tidy(node["description"])}
    if node["subcommands"]:
        out["subcommands"] = [command(c, [*path, c["name"]]) for c in node["subcommands"]]
    if node["flags"]:
        out["options"] = [option(f, path) for f in node["flags"]]
    if (args := ARGS.pop(" ".join(path), None)) is not None:
        out["args"] = args
    return out


def emit(value) -> str:
    if isinstance(value, Raw):
        return str(value)
    if isinstance(value, dict):
        return "{" + ", ".join(f"{k}: {emit(v)}" for k, v in value.items()) + "}"
    if isinstance(value, list):
        return "[" + ", ".join(emit(v) for v in value) + "]"
    return json.dumps(value, ensure_ascii=False)


tree = crawl([], "")
spec = command(tree, [])
spec["description"] = "Kubernetes cluster manager"
if ARGS:
    sys.exit(f"ARGS entries for commands kubectl no longer has: {sorted(ARGS)}")
# Hidden commands, not listed in `kubectl --help`
spec["subcommands"] += [
    {"name": "options", "description": "Print the list of flags inherited by all commands"},
    {"name": "help", "description": "Help about any command", "args": {"name": "command", "isOptional": True}},
]
spec["options"] = [option(f, [], persistent=True) for f in global_flags()]
# Every command and the root accept --help
spec["options"].insert(0, {"name": ["-h", "--help"], "description": "Show help for command", "isPersistent": True})

version = subprocess.run(
    [KUBECTL, "version", "--client"], capture_output=True, text=True, stdin=subprocess.DEVNULL, timeout=20
).stdout.split("\n")[0]
print(f"// Generated by build-scripts/specs/kubectl.py from kubectl {version.split()[-1]}; don't edit by hand")
print(PRELUDE)
print(f"const completionSpec: Fig.Spec = {emit(spec)};\n")
print("export default completionSpec;")
