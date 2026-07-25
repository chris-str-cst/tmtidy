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
```

Config lives at `~/.config/tmtidy/config.yaml`. See `config.example.yaml`.
Baked-in rules cover rust/node/python/go/xcode/gradle, so a minimal config with
only `roots:` works out of the box. Runs are logged to
`~/.local/state/tmtidy/tmtidy.log`.

## How it works

- Exclusions use `tmutil addexclusion` (sticky/xattr) — no root required.
- Decay only trashes a dir that matches a build-target rule, currently carries
  the exclusion xattr, is older than `max_age_days`, is at least `min_size_mb`,
  and is not excluded via `exclude_rules`/`exclude_paths`/`ignore`.
- Deletions go to the macOS Trash — always recoverable.

## macOS only
