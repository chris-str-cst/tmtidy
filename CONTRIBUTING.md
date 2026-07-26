# Contributing to tmtidy

Thanks for helping keep Time Machine backups lean. This is a small macOS-only
Rust CLI — contributions of any size are welcome.

## Before you start

- **Bugs / features:** open an [issue](https://github.com/chris-str-cst/tmtidy/issues)
  first so we can agree on scope before you write code. For anything non-trivial,
  a quick issue saves a rejected PR.
- **Security issues:** please don't file a public issue — see
  [Reporting security issues](#reporting-security-issues).

## Development setup

macOS only (Apple Silicon or Intel). You need a stable Rust toolchain via
[rustup](https://rustup.rs) or `brew install rust`.

```bash
git clone https://github.com/chris-str-cst/tmtidy
cd tmtidy
cargo build
cargo test
cargo run -- scan --dry-run   # exercise a subcommand safely
```

## Before you open a PR

CI runs these on every push/PR and fails on any of them — run them locally first:

```bash
cargo fmt --check         # formatting
cargo clippy -- -D warnings   # lints, warnings are errors
cargo test                # full suite
```

Then:

- Keep the change focused — one concern per PR.
- Add or update tests for behavior changes.
- Update docs (`README.md`, `docs/config.md`, `config.example.yaml`) when you
  change flags, config keys, or behavior.
- Commits: concise, present-tense. Gitmoji prefixes are welcome (see the log),
  not required.
- Don't bump the version or touch `Formula/tmtidy.rb` — releases are tagged by a
  maintainer and the formula is auto-bumped by CI.

## Safety-sensitive areas

`tmtidy` writes exclusion xattrs and can move directories to the Trash. Changes
to `decay`, `exclude`, or the `walker` touch destructive or filesystem-mutating
paths — call this out in your PR and make sure the dry-run path stays the default.
Deletions must always go to the Trash, never `rm`.

## Reporting security issues

Do not open a public issue for vulnerabilities. Email the maintainer at the
address on their [GitHub profile](https://github.com/chris-str-cst), or use
GitHub's private [security advisory](https://github.com/chris-str-cst/tmtidy/security/advisories/new)
flow. You'll get a response as soon as possible.

## License

By contributing, you agree your contributions are licensed under the project's
[MIT License](LICENSE).
