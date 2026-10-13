export default {
  "*.{rs,toml}": () => [
    "cargo +nightly-2025-11-01 fmt --check -- --color always",
    "cargo clippy --locked --color always -- -D warnings",
  ],
  "*.proto": () => [
    "cd proto && buf lint && buf format --exit-code > /dev/null",
  ],
  "*.py": ["ruff format --check", "ruff check"],
  "*.{ts,js,tsx,jsx,mjs}": "prettier --check",
  "!(*test*)*": "typos --config .typos.toml --force-exclude",
};
