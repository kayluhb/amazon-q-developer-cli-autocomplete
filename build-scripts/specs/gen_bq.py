"""Builds specs/bq.ts from the installed bq's help output. To regenerate after a Google Cloud SDK
upgrade:

    python3 build-scripts/specs/gen_bq.py > specs/bq.ts
    npx prettier --write specs/bq.ts --parser typescript

bq's help is absl-style (`bq --help` for global flags, `bq help <command>` for each command), so
the whole tree is crawled from it. The Cloud SDK wrapper refuses `bq help init`, so the crawl runs
the SDK's platform/bq/bq.py directly. Positional arguments aren't machine-readable in the help,
so they live in ARGS below; check them against each command's "Usage:" text after an upgrade.

The output depends on the machine: it crawls whichever `bq` is first on PATH (last generated from
Google Cloud SDK 583.0.0, bq 2.1.38). absl prints defaults with ~ expanded to $HOME, so the home
directory is written back as ~ to keep the spec the same for every user."""

import json
import os
import re
import shutil
import subprocess
import sys

from _common import Raw

BQ_PY = os.path.join(os.path.dirname(os.path.dirname(os.path.realpath(shutil.which("bq")))), "platform", "bq", "bq.py")


def bq(*args):
    return subprocess.run(
        [sys.executable, BQ_PY, *args],
        stdin=subprocess.DEVNULL,
        capture_output=True,
        text=True,
        timeout=60,
    ).stdout


FILE = {"template": "filepaths"}
TABLE = {"name": "table", "description": "[project:]dataset.table"}
IDENTIFIER = {"name": "identifier", "description": "Resource identifier, e.g. [project:]dataset.table"}

# Positional arguments by command, from each command's Usage/Arguments/Examples help text
ARGS = {
    "add-iam-policy-binding": IDENTIFIER,
    "cancel": {"name": "job_id"},
    "cp": [
        {"name": "source_table", "description": "One or more comma-separated source tables"},
        {"name": "destination_table"},
    ],
    "extract": [
        {"name": "source", "description": "Source table, or model with -m"},
        {"name": "destination_uris", "description": "Comma-separated Cloud Storage URIs"},
    ],
    "get-iam-policy": IDENTIFIER,
    "head": {**IDENTIFIER, "isOptional": True},
    "help": {"name": "command", "isOptional": True, "suggestions": Raw("COMMANDS")},
    "insert": [TABLE, {"name": "file", "isOptional": True, **FILE}],
    "load": [
        {"name": "destination_table"},
        {"name": "source", "description": "Local file or comma-separated URIs", **FILE},
        {"name": "schema", "isOptional": True, "description": "JSON schema file or inline schema", **FILE},
    ],
    "ls": {**IDENTIFIER, "isOptional": True},
    # Optional: mk --reservation_assignment and --transfer_config take only flags
    "mk": [
        {**IDENTIFIER, "isOptional": True},
        {"name": "schema", "isOptional": True, "description": "JSON schema file or inline schema", **FILE},
    ],
    "mkdef": [
        {"name": "source_uris", "description": "Comma-separated list of URIs"},
        {"name": "schema", "isOptional": True, "description": "JSON schema file or inline schema", **FILE},
    ],
    "partition": [{"name": "source_table_prefix"}, {"name": "destination_partitioned_table"}],
    "query": {"name": "sql_query", "isOptional": True, "description": "Reads stdin if omitted"},
    "remove-iam-policy-binding": IDENTIFIER,
    "rm": IDENTIFIER,
    "set-iam-policy": [IDENTIFIER, {"name": "filename", "description": "JSON policy file", **FILE}],
    "show": {**IDENTIFIER, "isOptional": True},
    "truncate": IDENTIFIER,
    "undelete": {"name": "dataset"},
    "update": [
        IDENTIFIER,
        {"name": "schema", "isOptional": True, "description": "JSON schema file or inline schema", **FILE},
    ],
    "wait": [
        {"name": "job_id", "isOptional": True},
        {"name": "secs", "isOptional": True, "description": "Seconds to wait (>= 0)"},
    ],
}

# Flags whose value is a local file or directory
FILE_FLAGS = {
    "--apilog",
    "--bigqueryrc",
    "--ca_certificates_file",
    "--discovery_file",
    "--flagfile",
    "--udf_resource",
    "--view_udf_resource",
}

# Typos in bq's help text, which the repo's typos check would reject. Written as regexes so
# the misspellings don't appear literally here either.
TYPOS = {r"res[p]resents": "represents", r"ses[s]on\b": "session"}

# Hand-written descriptions where bq's first sentence says nothing useful
DESCRIPTIONS = {
    "--use_legacy_sql": "Use Legacy SQL; if omitted, the server picks the dialect from the query",
}

FLAG_RE = re.compile(r"(?:(-\w+),)?--(\[no\])?(\w+):\s?(.*)")


def first_sentence(text):
    text = re.sub(r"\s+", " ", text).strip()
    for typo, fix in TYPOS.items():
        text = re.sub(typo, fix, text)
    text = re.sub(r"^usage:-\s*\S+\s+", "", text)  # --message_type leads with an example value
    text = text[:1].upper() + text[1:]
    # A one-word lead such as "Optional." or "DEPRECATED." keeps the sentence after it too
    m = re.match(r"((?:\S+\.\s)?.+?(?<!e\.g)(?<!i\.e))(?:[.;](?:\s|$)|$)", text)
    return (m.group(1) if m else text).rstrip(".;").rstrip()


def parse_flags(lines, indent, absl=False):
    """Parses absl flag help lines whose flags start at `indent` spaces, from the absl.flags
    section only if `absl`, otherwise from every other section."""
    flags, cur, skipping = [], None, False
    for line in lines:
        if not line.strip():
            continue
        lead = len(line) - len(line.lstrip())
        if lead == indent - 2 and line.strip().endswith(":"):
            skipping = (line.strip() == "absl.flags:") != absl
            cur = None
            continue
        if skipping:
            continue
        m = FLAG_RE.fullmatch(line[indent:]) if lead == indent else None
        if m:
            cur = {"short": m.group(1), "bool": bool(m.group(2)), "name": m.group(3), "body": [m.group(4)]}
            flags.append(cur)
        elif cur and lead > indent:
            cur["body"].append(line.strip())
        elif lead < indent:
            cur = None
    return [flag_to_option(f) for f in flags]


def flag_to_option(f):
    body = " ".join(b for b in f["body"] if b)
    long = "--" + f["name"]
    opt = {"name": [f["short"], long] if f["short"] else long}
    default = None
    # absl appends "(default: '...')" and type notes such as "(an integer)" after the description
    while True:
        m = re.search(r"\s*\((?:default: '(.*?)'|an? [^()]*|the [^()]*)\)\s*$", body)
        if not m:
            break
        if m.group(1) is not None:
            default = m.group(1).replace(os.path.expanduser("~"), "~")
        body = body[: m.start()]
    repeatable = "repeat this option to specify a list of values" in body
    body = re.sub(r";?\s*repeat this option to specify a list of values", "", body)
    choices = None
    # Enum values such as "<CSV|NEWLINE_DELIM\nITED_JSON>:" are wrapped mid-word, so join without spaces
    m = re.match(r"<([^>]*)>", "".join(f["body"]))
    if m:
        choices = [c for c in m.group(1).split("|") if c]
        body = re.sub(r"^<[^>]*>\s*:\s*", "", body)
    opt["description"] = DESCRIPTIONS.get(long) or first_sentence(body) or f["name"]
    if repeatable:
        opt["isRepeatable"] = True
    if not f["bool"]:
        arg = {"name": f["name"]}
        if choices:
            arg["suggestions"] = choices
        if default:
            arg["default"] = default
        if long in FILE_FLAGS:
            arg.update(FILE)
        opt["args"] = arg
    return opt, f["bool"]


def options(parsed, persistent=False):
    out = []
    for opt, is_bool in parsed:
        if persistent:
            opt["isPersistent"] = True
        out.append(opt)
        if is_bool:
            long = opt["name"][-1] if isinstance(opt["name"], list) else opt["name"]
            neg = {"name": "--no" + long[2:], "description": "Negates " + long, "hidden": True}
            if persistent:
                neg["isPersistent"] = True
            out.append(neg)
    return out


def command_names(top):
    block = top.split("Any of the following commands:")[1].strip().split("\n\n")[0]
    joined = ""
    for line in block.splitlines():
        line = line.strip()
        joined += line if joined.endswith("-") else " " + line
    return [c.strip() for c in joined.split(",") if c.strip()]


def command(name):
    text = bq("help", name)
    lines = text.splitlines()
    start = next(i for i, line in enumerate(lines) if line.startswith(name + " "))
    desc = [lines[start][len(name) :].strip()]
    for line in lines[start + 1 :]:
        if not line.strip():
            break
        desc.append(line.strip())
    cmd = {"name": name, "description": first_sentence(" ".join(desc))}
    if "Flags for %s:" % name in text:
        flag_lines = text.split("Flags for %s:" % name, 1)[1].split("\n\n\n")[0].splitlines()
        opts = options(parse_flags(flag_lines, 31))
        if opts:
            cmd["options"] = opts
    if name in ARGS:
        cmd["args"] = ARGS[name]
    return cmd


def to_ts(value):
    if isinstance(value, Raw):
        return str(value)
    if isinstance(value, dict):
        items = ["%s: %s" % (json.dumps(k), to_ts(v)) for k, v in value.items()]
        return "{" + ", ".join(items) + "}"
    if isinstance(value, list):
        return "[" + ", ".join(to_ts(v) for v in value) + "]"
    return json.dumps(value)


def main():
    version = re.search(r"BigQuery CLI (\S+)", bq("version")).group(1)
    top = bq("help")
    names = command_names(top)
    glob = bq("--help").split("Global flags:", 1)[1]
    glob_lines = glob.split("\n\nRun ")[0].splitlines()
    global_opts = options(parse_flags(glob_lines, 2), persistent=True)
    # absl's own flags (--flagfile, --undefok) are listed under every command; keep one copy
    absl_lines = bq("help", "cp").split("Flags for cp:", 1)[1].splitlines()
    global_opts += options(parse_flags(absl_lines, 31, absl=True), persistent=True)
    global_opts += [
        {"name": ["--help", "-h"], "description": "Show help for global flags"},
        {"name": "--helpshort", "description": "Show help for flags of the main module only"},
    ]
    spec = {
        "name": "bq",
        "description": "BigQuery command-line tool",
        "subcommands": [command(n) for n in names],
        "options": global_opts,
        "parserDirectives": {"flagsArePosixNoncompliant": True},
    }
    print("// Generated by build-scripts/specs/gen_bq.py from bq version %s; don't edit by hand" % version)
    print("const COMMANDS: string[] = %s;\n" % to_ts(names))
    print("const completionSpec: Fig.Spec = %s;\n" % to_ts(spec))
    print("export default completionSpec;")


if __name__ == "__main__":
    main()
