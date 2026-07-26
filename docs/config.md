# Configuration

Config lives at `~/.config/tmtidy/config.yaml`. Every key is optional except
`roots:` (commands that walk the disk need at least one). A minimal config:

```yaml
roots:
  - path: ~/code
```

Run `tmtidy config` to print the **fully-resolved effective config** — your
settings merged with the baked-in defaults. Useful to see exactly which rules
are active before a scan.

## Baked-in default rules

`tmtidy` ships rules for common ecosystems. They apply automatically; you don't
need to declare them.

| Rule      | Markers (any present)               | Excluded target dirs         |
|-----------|-------------------------------------|------------------------------|
| `rust`    | `Cargo.toml`                        | `target`                     |
| `node`    | `package.json`                      | `node_modules`, `.next`, `dist` |
| `python`  | `pyproject.toml`, `setup.py`        | `.venv`, `__pycache__`, `build` |
| `go`      | `go.mod`                            | *(none — see below)*         |
| `swiftpm` | `Package.swift`                     | `.build`                     |
| `gradle`  | `build.gradle`, `build.gradle.kts`  | `build`, `.gradle`           |
| `maven`   | `pom.xml`                           | `target`                     |
| `terraform` | `.terraform.lock.hcl`             | `.terraform`                 |
| `terragrunt` | `terragrunt.hcl`, `root.hcl`     | `.terragrunt-cache`          |

A rule fires when a directory contains one of its markers **and** a matching
target subdirectory; the target dir is what gets excluded from Time Machine.
The `terraform` rule also covers **OpenTofu** (same `.terraform` dir and
lockfile).

### Selecting which defaults apply

Use `defaults:` as an allowlist. Omit it for all nine. List names to keep only
those. An empty list disables all baked defaults.

```yaml
defaults: [rust, node]   # only these; the other seven baked rules off
```

Your own `rules:` always apply regardless of `defaults:` — the allowlist filters
baked rules only.

### Custom rules

`rules:` merge over the baked defaults. A user rule with the same `name` as a
baked one **replaces** it entirely (targets are not unioned):

```yaml
rules:
  - name: node                       # overrides the baked node rule
    markers: [package.json]
    targets: [node_modules, .next, dist]
  - name: unity                      # brand-new rule
    markers: [Assembly-CSharp.csproj]
    targets: [Library, Temp]
```

## Special / out-of-scope directories

Some build artifacts live in **global**, per-user locations rather than inside a
project, so per-project rules can't reach them:

- **Go** (`~/Library/Caches/go-build`) — Go's build cache is global. The `go`
  rule matches `go.mod` but excludes nothing per-project; exclude the cache
  path yourself if wanted (see below).
- **Xcode DerivedData** (`~/Library/Developer/Xcode/DerivedData`) — global, not
  per-project. The `swiftpm` rule instead targets SwiftPM's local `.build`.
- **Homebrew, pip/npm/cargo global caches** — likewise global.

To exclude a fixed global path, add it as a single-directory root, or exclude it
once by hand:

```bash
tmutil addexclusion ~/Library/Caches/go-build
```

## Full Disk Access

`tmtidy` uses **sticky (xattr) exclusions**: it writes the
`com.apple.metadata:com_apple_backup_excludeItem` xattr directly — the same one
`tmutil addexclusion` sets, but without shelling out to `tmutil` (which blocks
~10s per call). This needs **no `sudo` and no root**, unlike fixed-path
(`tmutil addexclusion -p`) exclusions.

However, macOS **TCC** still gates *reading* certain folders. If your roots
include protected locations — `~/Desktop`, `~/Documents`, `~/Downloads`, or
external/network volumes — the scan **prints an error** naming the denied paths
(it doesn't silently skip). Grant **Full Disk Access** to whatever runs tmtidy:

> System Settings → Privacy & Security → Full Disk Access

For an interactive `tmtidy scan`, grant your terminal (Terminal.app, iTerm, or
your IDE) — the run borrows its grant. **But TCC grants attach to the binary and
do *not* inherit**, so a scheduled run (via launchd, with no terminal parent) is
judged on the `tmtidy` binary itself. If you schedule scans over protected
folders, add the **tmtidy binary** to Full Disk Access — `schedule install`
prints its resolved path as `binary:`. Then re-run.

Full Disk Access is only about *traversing* protected folders; the exclusion
mechanism itself never requires elevated privileges.

## Decay settings

Decay reclaims disk from long-dormant excluded dirs by moving them to the Trash.
All keys live under `decay:`:

| Key            | Default | Meaning                                                        |
|----------------|---------|----------------------------------------------------------------|
| `max_age_days` | `30`    | Only dirs whose mtime is older than this are candidates.       |
| `min_size_mb`  | `100`   | Size floor — smaller dirs are ignored (avoids churn).          |
| `auto_clean`   | `false` | Trash candidates without needing `--clean`.                    |
| `exclude_rules`| `[]`    | Rule names whose matches are never decayed (e.g. `swiftpm`).   |
| `exclude_paths`| `[]`    | Specific paths never decayed.                                  |
| `json_output`  | *(off)* | Also append each decay report to this file.                    |

A directory is trashed **only if all five hold**: (1) it matches a rule target,
(2) currently carries the exclusion xattr, (3) is older than `max_age_days`,
(4) is at least `min_size_mb`, and (5) is not listed in
`exclude_rules` / `exclude_paths` / top-level `ignore`. Deletions go to the
macOS Trash and are always recoverable.

### Flag precedence

CLI flags override config:

- `--dry-run` always wins — never trashes, even with `auto_clean: true`.
- `--clean` trashes even if `auto_clean: false`.
- `--json` prints the report to stdout regardless of `json_output`.

## Top-level `ignore`

Paths to skip entirely during the walk (neither excluded nor decayed):

```yaml
ignore:
  - ~/code/keepme
```

## Full example

See [`config.example.yaml`](../config.example.yaml).
