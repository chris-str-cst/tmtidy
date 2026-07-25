use crate::config::Config;
use crate::decay::{find_candidates, trash_path};
use crate::exclude::{add_exclusion, is_excluded};
use crate::rules::match_dir;
use crate::stats::{DecayStats, ScanStats};
use crate::walker::walk_root;
use crate::logging::append_run;
use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Parser)]
#[command(name = "tmtidy", version, about = "Keep Time Machine lean by excluding build dirs")]
struct Cli {
    /// Config file (default ~/.config/tmtidy/config.yaml)
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[arg(long, global = true)]
    verbose: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Walk roots and exclude build dirs (default)
    Scan,
    /// Find (and optionally trash) stale excluded build dirs
    Decay {
        #[arg(long)]
        clean: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
    /// Show current exclusion count
    Status,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let cfg = Config::load(cli.config.as_deref())?;
    match cli.command.unwrap_or(Command::Scan) {
        Command::Scan => cmd_scan(&cfg, cli.verbose),
        Command::Decay { clean, dry_run, json } => cmd_decay(&cfg, clean, dry_run, json, cli.verbose),
        Command::Status => cmd_status(&cfg, cli.verbose),
    }
}

fn ensure_roots(cfg: &Config) -> Result<()> {
    if cfg.roots.is_empty() {
        anyhow::bail!("no roots configured. Create ~/.config/tmtidy/config.yaml with a `roots:` list.");
    }
    Ok(())
}

fn cmd_scan(cfg: &Config, verbose: bool) -> Result<()> {
    ensure_roots(cfg)?;
    let prune = cfg.target_names();
    let ignore: HashSet<PathBuf> = cfg.ignore.iter().cloned().collect();
    let mut stats = ScanStats::default();

    for root in &cfg.roots {
        for dir in walk_root(root, &prune, &ignore) {
            for m in match_dir(&dir, &cfg.rules) {
                if is_excluded(&m.path) {
                    stats.skipped_existing += 1;
                    if verbose {
                        println!("= already {}", m.path.display());
                    }
                    continue;
                }
                match add_exclusion(&m.path) {
                    Ok(()) => {
                        stats.excluded_new += 1;
                        if verbose {
                            println!("+ excluded {}", m.path.display());
                        }
                    }
                    Err(e) => {
                        stats.errors += 1;
                        eprintln!("warn: {}", e);
                    }
                }
            }
        }
    }
    println!(
        "scan: {} newly excluded, {} already excluded, {} errors",
        stats.excluded_new, stats.skipped_existing, stats.errors
    );
    append_run(&json!({"ts": now_iso(), "command": "scan", "stats": stats}))?;
    Ok(())
}

/// Precedence: --dry-run always wins; else clean if --clean or auto_clean.
fn should_clean(dry_run: bool, clean: bool, auto_clean: bool) -> bool {
    !dry_run && (clean || auto_clean)
}

fn cmd_decay(cfg: &Config, clean: bool, dry_run: bool, json_flag: bool, verbose: bool) -> Result<()> {
    ensure_roots(cfg)?;
    let do_clean = should_clean(dry_run, clean, cfg.decay.auto_clean);

    let mut candidates = find_candidates(cfg, SystemTime::now());
    let mut stats = DecayStats { candidates: candidates.len() as u64, ..Default::default() };

    for c in &mut candidates {
        if do_clean {
            match trash_path(&c.path) {
                Ok(()) => {
                    c.trashed = true;
                    stats.trashed += 1;
                    stats.reclaimed_bytes += c.size_bytes;
                    if verbose {
                        println!("{} ({}, {}d) trashed", c.path.display(), human(c.size_bytes), c.age_days);
                    }
                }
                Err(e) => eprintln!("warn: {}", e),
            }
        } else if verbose {
            println!("{} ({}, {}d) would trash", c.path.display(), human(c.size_bytes), c.age_days);
        }
    }

    let total_reclaimed: u64 = candidates.iter().map(|c| c.size_bytes).sum();
    let result = json!({
        "generated_at": now_iso(),
        "command": "decay",
        "max_age_days": cfg.decay.max_age_days,
        "cleaned": do_clean,
        "candidates": candidates,
        "total_candidate_bytes": total_reclaimed,
        "total_reclaimed_bytes": stats.reclaimed_bytes,
    });

    if json_flag {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "decay: {} candidates (>{}d){}, {} → {}",
            stats.candidates, cfg.decay.max_age_days,
            if do_clean { "" } else { " [dry-run]" },
            human(if do_clean { stats.reclaimed_bytes } else { total_reclaimed }),
            if do_clean { "Trash" } else { "would trash" },
        );
    }
    if let Some(p) = &cfg.decay.json_output {
        crate::logging::append_run_to(p, &result).ok();
    }
    append_run(&result)?;
    Ok(())
}

fn cmd_status(cfg: &Config, verbose: bool) -> Result<()> {
    ensure_roots(cfg)?;
    let prune = cfg.target_names();
    let ignore: HashSet<PathBuf> = cfg.ignore.iter().cloned().collect();
    let mut count = 0u64;
    for root in &cfg.roots {
        for dir in walk_root(root, &prune, &ignore) {
            for m in match_dir(&dir, &cfg.rules) {
                if is_excluded(&m.path) {
                    count += 1;
                    if verbose {
                        println!("{}", m.path.display());
                    }
                }
            }
        }
    }
    println!("status: {} excluded build dirs", count);
    Ok(())
}

fn human(bytes: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut b = bytes as f64;
    let mut i = 0;
    while b >= 1024.0 && i < U.len() - 1 { b /= 1024.0; i += 1; }
    format!("{:.1} {}", b, U[i])
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::should_clean;

    #[test]
    fn dry_run_forces_false_regardless_of_clean() {
        assert!(!should_clean(true, true, false));
    }

    #[test]
    fn dry_run_forces_false_regardless_of_auto_clean() {
        assert!(!should_clean(true, false, true));
    }

    #[test]
    fn clean_flag_true_and_not_dry_run_cleans() {
        assert!(should_clean(false, true, false));
    }

    #[test]
    fn auto_clean_true_and_not_dry_run_cleans() {
        assert!(should_clean(false, false, true));
    }

    #[test]
    fn all_false_does_not_clean() {
        assert!(!should_clean(false, false, false));
    }
}
