from util import run_cmd


def run_doc():
    run_cmd(["cargo", "doc", "--no-deps", "--workspace"])
