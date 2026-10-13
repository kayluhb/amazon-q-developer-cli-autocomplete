import itertools
import os
from typing import List, Mapping, Sequence
from rust import rust_env
from util import run_cmd, get_variants, Variant
from const import DESKTOP_FUZZ_PACKAGE_NAME, DESKTOP_PACKAGE_NAME


def run_clippy(
    variants: List[Variant],
    features: Mapping[str, Sequence[str]] | None = None,
    target: str | None = None,
    fail_on_warn: bool = False,
):
    args = ["cargo", "clippy", "--locked", "--workspace"]

    if target:
        args.extend(["--target", target])

    if Variant.FULL not in variants:
        args.extend(["--exclude", DESKTOP_PACKAGE_NAME, "--exclude", DESKTOP_FUZZ_PACKAGE_NAME])

    if features:
        args.extend(
            [
                "--features",
                ",".join(set(itertools.chain.from_iterable(features.values()))),
            ]
        )

    if fail_on_warn:
        args.extend(["--", "-D", "warnings"])

    run_cmd(
        args,
        env={
            **os.environ,
            **rust_env(release=False),
        },
    )


def run_cargo_tests(
    variants: List[Variant],
    features: Mapping[str, Sequence[str]] | None = None,
    target: str | None = None,
):
    args = ["cargo"]

    args.extend(["build", "--tests", "--locked", "--workspace", "--exclude", DESKTOP_FUZZ_PACKAGE_NAME])

    if target:
        args.extend(["--target", target])

    if Variant.FULL not in variants:
        args.extend(["--exclude", DESKTOP_PACKAGE_NAME])

    if features:
        args.extend(
            [
                "--features",
                ",".join(set(itertools.chain.from_iterable(features.values()))),
            ]
        )

    run_cmd(
        args,
        env={
            **os.environ,
            **rust_env(release=False),
        },
    )

    args = ["cargo"]

    # Run all lib, bin, and integration tests. Required to exclude running doc tests.
    args.extend(
        ["test", "--locked", "--workspace", "--lib", "--bins", "--test", "*", "--exclude", DESKTOP_FUZZ_PACKAGE_NAME]
    )

    if target:
        args.extend(["--target", target])

    if features:
        args.extend(
            [
                "--features",
                ",".join(set(itertools.chain.from_iterable(features.values()))),
            ]
        )

    run_cmd(
        args,
        env={
            **os.environ,
            **rust_env(release=False),
        },
    )


def all_tests(clippy_fail_on_warn: bool):
    variants = get_variants()
    run_cargo_tests(variants=variants)
    run_clippy(variants=variants, fail_on_warn=clippy_fail_on_warn)
