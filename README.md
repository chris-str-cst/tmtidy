# tmtidy

Keep Time Machine backups lean. `tmtidy` scans your project roots, excludes
build/cache dirs (`target`, `node_modules`, `.venv`, …) from Time Machine using
sticky exclusions (no sudo), and can reclaim disk from long-dormant excluded
dirs by moving them to the Trash.

## Install

```bash
cargo install --path .
```

## Usage

```bash
tmtidy scan            # exclude build dirs under configured roots (default)
tmtidy status          # count current exclusions
tmtidy decay           # report stale excluded dirs (dry-run)
tmtidy decay --clean   # move stale excluded dirs to Trash
tmtidy decay --json    # machine-readable report
tmtidy config          # print the fully-resolved effective config as YAML
```

Config lives at `~/.config/tmtidy/config.yaml`. Baked-in rules cover
rust/node/python/go/swiftpm/gradle/maven/terraform/terragrunt, so a minimal
config with only `roots:` works out of the box. Use `defaults:` to pick which
baked rules apply, and `tmtidy config` to see what's active. Runs are logged to
`~/.local/state/tmtidy/tmtidy.log`.

**See [Configuration](docs/config.md)** for the full reference: every config
key, the baked-in rules table, the `defaults:` allowlist, custom rules, and
decay settings. A ready-to-edit `config.example.yaml` is included.

## Scheduling (macOS)

Run `scan` automatically so build dirs are excluded before Time Machine's next
hourly backup. Installs a per-user launchd LaunchAgent — no sudo, loads at login.

```bash
tmtidy schedule install          # hourly (default), runs once immediately too
tmtidy schedule install --every 30m
tmtidy schedule status           # installed? loaded? last run
tmtidy schedule disable          # stop, keep config
tmtidy schedule enable           # resume
tmtidy schedule uninstall        # remove entirely
```

Only `scan` is scheduled (safe, non-destructive). Run `tmtidy decay` yourself
when you want to reclaim space. `--every` takes a single unit: `s`, `m`, `h`, `d`
(minimum 1 minute).
The agent logs to `~/.local/state/tmtidy/scan.log`; logs are capped at 5 MiB —
oldest entries are dropped in place, with no archive kept.

## How it works

- Exclusions set the sticky `com.apple.metadata:com_apple_backup_excludeItem`
  xattr directly (the same one `tmutil addexclusion` writes) — no root required.
- Decay only trashes a dir that matches a build-target rule, currently carries
  the exclusion xattr, is older than `max_age_days`, is at least `min_size_mb`,
  and is not excluded via `exclude_rules`/`exclude_paths`/`ignore`.
- Deletions go to the macOS Trash — always recoverable.

## Special directories

Some build caches live in **global**, per-user locations, not inside a project,
so per-project rules can't reach them:

- **Go** — build cache is global (`~/Library/Caches/go-build`); the `go` rule
  excludes nothing per-project.
- **Xcode DerivedData** — global (`~/Library/Developer/Xcode/DerivedData`); the
  `swiftpm` rule targets SwiftPM's local `.build` instead.

Exclude a fixed global path once by hand: `tmutil addexclusion ~/Library/Caches/go-build`.

## Full Disk Access

Exclusions use sticky (xattr) exclusions — **no `sudo` or root**. But macOS
**TCC** still gates *reading* protected folders. If your roots include
`~/Desktop`, `~/Documents`, `~/Downloads`, or external/network volumes and scans
silently skip files, grant your terminal (or IDE) **Full Disk Access**:

> System Settings → Privacy & Security → Full Disk Access → enable your terminal

FDA is only about traversing protected folders; the exclusion mechanism itself
never needs elevated privileges. See [Configuration](docs/config.md) for details.

## Development

Needs a stable Rust toolchain (install via [rustup](https://rustup.rs)).

```bash
cargo build            # debug build
cargo build --release  # optimized binary -> target/release/tmtidy
cargo test             # run the full test suite
cargo run -- scan      # run a subcommand from source (args after `--`)
cargo clippy           # lints
```

## macOS only
