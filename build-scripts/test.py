import itertools
import os
from typing import List, Mapping, Sequence
from rust import rust_env
from util import run_cmd, get_variants, Variant
from const import DESKTOP_FUZZ_PACKAGE_NAME, DESKTOP_PACKAGE_NAME


def cargo_args(
    command: List[str],
    variants: List[Variant],
    features: Mapping[str, Sequence[str]] | None,
    target: str | None,
    exclude_without_full: Sequence[str] = (DESKTOP_PACKAGE_NAME,),
) -> List[str]:
    """`cargo <command>` plus the target, the packages to exclude without the full variant, and the features."""
    args = ["cargo", *command]

    if target:
        args.extend(["--target", target])

    if Variant.FULL not in variants:
        for package in exclude_without_full:
            args.extend(["--exclude", package])

    if features:
        args.extend(
            [
                "--features",
                ",".join(set(itertools.chain.from_iterable(features.values()))),
            ]
        )

    return args


def run_cargo(args: List[str]):
    run_cmd(
        args,
        env={
            **os.environ,
            **rust_env(release=False),
        },
    )


def run_clippy(
    variants: List[Variant],
    features: Mapping[str, Sequence[str]] | None = None,
    target: str | None = None,
    fail_on_warn: bool = False,
):
    args = cargo_args(
        ["clippy", "--locked", "--workspace"],
        variants,
        features,
        target,
        exclude_without_full=(DESKTOP_PACKAGE_NAME, DESKTOP_FUZZ_PACKAGE_NAME),
    )

    if fail_on_warn:
        args.extend(["--", "-D", "warnings"])

    run_cargo(args)


def run_cargo_tests(
    variants: List[Variant],
    features: Mapping[str, Sequence[str]] | None = None,
    target: str | None = None,
):
    run_cargo(
        cargo_args(
            ["build", "--tests", "--locked", "--workspace", "--exclude", DESKTOP_FUZZ_PACKAGE_NAME],
            variants,
            features,
            target,
        )
    )

    # Run all lib, bin, and integration tests. Required to exclude running doc tests.
    run_cargo(
        cargo_args(
            [
                "test",
                "--locked",
                "--workspace",
                "--lib",
                "--bins",
                "--test",
                "*",
                "--exclude",
                DESKTOP_FUZZ_PACKAGE_NAME,
            ],
            variants,
            features,
            target,
        )
    )


def all_tests(clippy_fail_on_warn: bool):
    variants = get_variants()
    run_cargo_tests(variants=variants)
    run_clippy(variants=variants, fail_on_warn=clippy_fail_on_warn)
