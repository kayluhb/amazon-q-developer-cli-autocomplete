from enum import Enum
from functools import cache
import json
import os
import shlex
import stat
import subprocess
import pathlib
import platform
from typing import List, Mapping, Sequence
from const import DESKTOP_PACKAGE_NAME, TAURI_PRODUCT_NAME

DEBUG = "\033[94;1m"
INFO = "\033[92;1m"
WARN = "\033[93;1m"
FAIL = "\033[91;1m"
ENDC = "\033[0m"


@cache
def isCi() -> bool:
    return os.environ.get("CI") is not None


@cache
def isDarwin() -> bool:
    return platform.system() == "Darwin"


@cache
def version() -> str:
    output = run_cmd_output(
        [
            "cargo",
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
        ]
    )
    data = json.loads(output)
    for pkg in data["packages"]:
        if pkg["name"] == DESKTOP_PACKAGE_NAME:
            return pkg["version"]
    raise ValueError("Version not found")


@cache
def tauri_product_name() -> str:
    """
        Derived from the `package.productName` configured in the tauri.conf.json file.

    Tauri build output paths replace underscores with dashes.
    """
    return TAURI_PRODUCT_NAME.replace("_", "-")


def log(*value: object, title: str, color: str | None):
    if isCi() or color is None:
        print(f"{title}:", *value, flush=True)
    else:
        print(f"{color}{title}:{ENDC}", *value, flush=True)


def debug(*value: object):
    log(*value, title="DEBUG", color=DEBUG)


def info(*value: object):
    log(*value, title="INFO", color=INFO)


def warn(*value: object):
    log(*value, title="WARN", color=WARN)


def fail(*value: object):
    log(*value, title="FAIL", color=FAIL)


Args = Sequence[str | os.PathLike]
Env = Mapping[str, str | os.PathLike]
Cwd = str | os.PathLike


class Secret(str):
    """An argv value that run_cmd prints as *** instead of its value.

    repr() is also redacted, so a CalledProcessError traceback (which shows
    repr(args)) does not leak the value. str() still returns the real value,
    so subprocess passes it through to argv.
    """

    def __repr__(self) -> str:
        return "'***'"


def format_cmd(args: Args) -> str:
    return shlex.join("***" if isinstance(arg, Secret) else str(arg) for arg in args)


def run_cmd(args: Args, env: Env | None = None, cwd: Cwd | None = None, check: bool = True):
    print(f"+ {format_cmd(args)}")
    subprocess.run(args, env=env, cwd=cwd, check=check)


def run_cmd_output(
    args: Args,
    env: Env | None = None,
    cwd: Cwd | None = None,
) -> str:
    print(f"+ {format_cmd(args)}")
    res = subprocess.run(args, env=env, cwd=cwd, check=True, stdout=subprocess.PIPE)
    return res.stdout.decode("utf-8")


def run_cmd_status(
    args: Args,
    env: Env | None = None,
    cwd: Cwd | None = None,
) -> int:
    res = subprocess.run(args, env=env, cwd=cwd)
    return res.returncode


def set_executable(path: pathlib.Path):
    st = os.stat(path)
    os.chmod(path, st.st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


class Variant(Enum):
    FULL = 1
    MINIMAL = 2


@cache
def get_variants() -> List[Variant]:
    match platform.system():
        case "Darwin":
            return [Variant.FULL]
        case other:
            raise ValueError(f"Unsupported platform {other}")


def enum_encoder(obj):
    if isinstance(obj, Enum):
        return obj.value
    return obj.__dict__
