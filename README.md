# tmtidy

[![CI](https://github.com/chris-str-cst/tmtidy/actions/workflows/ci.yml/badge.svg)](https://github.com/chris-str-cst/tmtidy/actions/workflows/ci.yml)

Keep Time Machine backups lean. `tmtidy` scans your project roots, excludes
build/cache dirs (`target`, `node_modules`, `.venv`, …) from Time Machine using
sticky exclusions (no sudo), and can reclaim disk from long-dormant excluded
dirs by moving them to the Trash.

## Install

macOS only. Apple Silicon and Intel are both supported.

### Homebrew (recommended)

```bash
brew tap chris-str-cst/tmtidy https://github.com/chris-str-cst/tmtidy
brew trust chris-str-cst/tmtidy   # recent Homebrew gates third-party taps
brew install tmtidy
```

Upgrade later with `brew upgrade tmtidy`. The `brew trust` step is only needed on
Homebrew versions that block untrusted taps — if `brew install` doesn't complain,
you can skip it.

### Manual download

Grab the tarball for your arch from the
[Releases page](https://github.com/chris-str-cst/tmtidy/releases), extract, and
put `tmtidy` on your `PATH`:

```bash
tar -xzf tmtidy-*-aarch64-apple-darwin.tar.gz   # or x86_64- on Intel
sudo mv tmtidy /usr/local/bin/
```

A double-clicked download is quarantined by macOS. Clear it once (Homebrew
installs are never quarantined, so the tap skips this):

```bash
xattr -d com.apple.quarantine /usr/local/bin/tmtidy
```

Binaries are unsigned; the `xattr` clear (or right-click → Open) is all Gatekeeper
needs.

### From source

See [Development](#development).

## Configure

`tmtidy` needs at least one **root** to scan. Baked-in rules already cover
rust/node/python/go/swiftpm/gradle/maven/terraform/terragrunt, so a config with
only `roots:` works out of the box. Scaffold it with:

```bash
tmtidy init          # writes ~/.config/tmtidy/config.yaml + creates the state dir
```

`init` drops the fully-commented [`config.example.yaml`](config.example.yaml)
template in place and never overwrites an existing config (pass `--force` to
replace it). Then edit its `roots:` list. Prefer to do it by hand?

```bash
mkdir -p ~/.config/tmtidy
cat > ~/.config/tmtidy/config.yaml <<'EOF'
roots:
  - path: ~/code      # scan everything under here (add more entries as needed)
    max_depth: 8      # optional, default 8
EOF
```

Verify what's active — your roots plus every baked-in rule, decay settings, and
ignores:

```bash
tmtidy config
```

Without a config (or with no `roots:`), `scan`/`status`/`decay` exit with
`no roots configured …`. Want a fuller starting template with every option
commented? Grab [`config.example.yaml`](config.example.yaml). Full reference —
all keys, the baked-rule table, the `defaults:` allowlist, custom rules, and
decay — in [Configuration](docs/config.md).

## Usage

```bash
tmtidy init            # scaffold config + state dir (--force to overwrite config)
tmtidy scan            # exclude build dirs under configured roots (default)
tmtidy scan --dry-run  # report what would be excluded, write nothing
tmtidy status          # count current exclusions
tmtidy decay           # report stale excluded dirs (dry-run)
tmtidy decay --clean   # move stale excluded dirs to Trash
tmtidy decay --json    # machine-readable report
tmtidy config          # print the fully-resolved effective config as YAML
```

Config lives at `~/.config/tmtidy/config.yaml` (see [Configure](#configure) to
create it). Runs are logged to `~/.local/state/tmtidy/tmtidy.log`.

### Verbose output

By default `scan` prints only a summary line. Pass the global `--verbose` flag
(before the subcommand) to see every excluded dir. `+` = excluded (or *would* on
`--dry-run`), `=` = already excluded:

```console
$ tmtidy --verbose scan --dry-run
+ would exclude ~/code/web/node_modules
+ would exclude ~/code/web/dist
+ would exclude ~/code/ios/.build
+ would exclude ~/code/ml/.venv
+ would exclude ~/code/ml/__pycache__
+ would exclude ~/code/api/target
scan: 6 would exclude, 0 already excluded, 0 errors [dry-run]
```

On a re-run, dirs already carrying the exclusion xattr show as `=`:

```console
$ tmtidy --verbose scan
= already ~/code/web/node_modules
= already ~/code/api/target
scan: 0 newly excluded, 2 already excluded, 0 errors
```

`--verbose` works the same way with `decay` and `status`.

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

The agent runs as you (per-user domain), never root. If your roots include
macOS-protected folders, grant the **tmtidy binary** Full Disk Access — a
scheduled run does *not* inherit your terminal's grant. See
[Full Disk Access](#full-disk-access).

## How it works

- Exclusions set the sticky `com.apple.metadata:com_apple_backup_excludeItem`
  xattr directly (the same one `tmutil addexclusion` writes) — no root required.
- Decay is opt-in and manual: it only trashes on `tmtidy decay --clean` (never
  during `scan`, never scheduled), and is dry-run by default. Skip it entirely and
  nothing is ever trashed. When run, it only trashes a dir that matches a
  build-target rule, currently carries the exclusion xattr, is older than
  `max_age_days`, is at least `min_size_mb`, and is not excluded via
  `exclude_rules`/`exclude_paths`/`ignore`.
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
`~/Desktop`, `~/Documents`, `~/Downloads`, or external/network volumes without
Full Disk Access, `scan`/`status` **print an error** naming the denied paths and
the exact binary to grant (they don't silently skip). Run `tmtidy scan` and read
the message — it tells you what to do.

> System Settings → Privacy & Security → Full Disk Access → enable your terminal

**TCC grants attach to the binary, and do *not* inherit.** When you run
`tmtidy scan` in Terminal it borrows Terminal's grant; a scheduled run has no
such parent, so launchd judges the `tmtidy` binary on its own. Result: an
interactive scan can reach protected folders while the scheduled one gets
permission-denied on the same paths (visible in `~/.local/state/tmtidy/scan.log`).
If you schedule scans over protected folders, add the **tmtidy binary itself** to
Full Disk Access — `schedule install` prints its resolved path as `binary:`. If
your roots are only project/dev trees outside the protected set, no FDA is needed.

FDA is only about traversing protected folders; the exclusion mechanism itself
never needs elevated privileges. See [Configuration](docs/config.md) for details.

## Development

Needs a stable Rust toolchain — via [rustup](https://rustup.rs) or `brew install rust`.

```bash
cargo build            # debug build
cargo build --release  # optimized binary -> target/release/tmtidy
cargo test             # run the full test suite
cargo run -- scan      # run a subcommand from source (args after `--`)
cargo clippy           # lints
cargo install --path . # build + install to ~/.cargo/bin/tmtidy
```

CI (`.github/workflows/ci.yml`) runs `fmt --check`, `clippy -D warnings`, tests,
and a release build for both macOS arches on every push/PR. Pushing a `v*` tag
triggers `release.yml`: it builds per-arch tarballs, publishes a GitHub Release,
and auto-bumps `Formula/tmtidy.rb`. Keep the git tag and `Cargo.toml` version in
sync — the release job fails on mismatch.

## macOS only
