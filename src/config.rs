use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::defaults::default_rules;

fn default_depth() -> usize { 8 }
fn default_max_age_days() -> u64 { 30 }
fn default_min_size_mb() -> u64 { 100 }

#[derive(Debug, Clone, Deserialize)]
pub struct Root {
    #[serde(deserialize_with = "de_path")]
    pub path: PathBuf,
    #[serde(default = "default_depth")]
    pub max_depth: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub name: String,
    #[serde(default)]
    pub markers: Vec<String>,
    #[serde(default)]
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DecayConfig {
    #[serde(default = "default_max_age_days")]
    pub max_age_days: u64,
    #[serde(default = "default_min_size_mb")]
    pub min_size_mb: u64,
    #[serde(default)]
    pub auto_clean: bool,
    #[serde(default)]
    pub exclude_rules: Vec<String>,
    #[serde(default, deserialize_with = "de_paths")]
    pub exclude_paths: Vec<PathBuf>,
    #[serde(default, deserialize_with = "de_opt_path")]
    pub json_output: Option<PathBuf>,
}

impl Default for DecayConfig {
    fn default() -> Self {
        DecayConfig {
            max_age_days: default_max_age_days(),
            min_size_mb: default_min_size_mb(),
            auto_clean: false,
            exclude_rules: Vec::new(),
            exclude_paths: Vec::new(),
            json_output: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub roots: Vec<Root>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub decay: DecayConfig,
    #[serde(default, deserialize_with = "de_paths")]
    pub ignore: Vec<PathBuf>,
}

fn expand(s: &str) -> PathBuf {
    PathBuf::from(shellexpand::tilde(s).into_owned())
}

fn de_path<'de, D>(d: D) -> std::result::Result<PathBuf, D::Error>
where D: serde::Deserializer<'de> {
    let s = String::deserialize(d)?;
    Ok(expand(&s))
}
fn de_paths<'de, D>(d: D) -> std::result::Result<Vec<PathBuf>, D::Error>
where D: serde::Deserializer<'de> {
    let v = Vec::<String>::deserialize(d)?;
    Ok(v.iter().map(|s| expand(s)).collect())
}
fn de_opt_path<'de, D>(d: D) -> std::result::Result<Option<PathBuf>, D::Error>
where D: serde::Deserializer<'de> {
    let o = Option::<String>::deserialize(d)?;
    Ok(o.map(|s| expand(&s)))
}

impl Config {
    pub fn from_yaml_str(s: &str) -> Result<Config> {
        let mut cfg: Config = serde_yaml::from_str(s).context("parsing config YAML")?;
        cfg.merge_default_rules();
        Ok(cfg)
    }

    pub fn load(path: Option<&Path>) -> Result<Config> {
        let path = path.map(PathBuf::from).unwrap_or_else(default_config_path);
        if path.exists() {
            let s = std::fs::read_to_string(&path)
                .with_context(|| format!("reading config {}", path.display()))?;
            Config::from_yaml_str(&s)
        } else {
            // No config file: defaults only, empty roots (caller validates).
            let mut cfg = Config { roots: vec![], rules: vec![], decay: DecayConfig::default(), ignore: vec![] };
            cfg.merge_default_rules();
            Ok(cfg)
        }
    }

    /// Add default rules for any name the user did not define. User rules win.
    fn merge_default_rules(&mut self) {
        for d in default_rules() {
            if !self.rules.iter().any(|r| r.name == d.name) {
                self.rules.push(d);
            }
        }
    }

    /// Union of every target dir name across all rules — used to prune the walk.
    pub fn target_names(&self) -> std::collections::HashSet<String> {
        self.rules.iter().flat_map(|r| r.targets.iter().cloned()).collect()
    }
}

pub fn default_config_path() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".config/tmtidy/config.yaml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_yaml_and_applies_decay_defaults() {
        let yaml = "roots:\n  - path: /tmp/a\n";
        let cfg: Config = Config::from_yaml_str(yaml).unwrap();
        assert_eq!(cfg.roots.len(), 1);
        assert_eq!(cfg.roots[0].max_depth, 8); // default depth
        assert_eq!(cfg.decay.max_age_days, 30);
        assert_eq!(cfg.decay.min_size_mb, 100);
        assert!(!cfg.decay.auto_clean);
    }

    #[test]
    fn user_rules_are_merged_with_defaults_and_override_by_name() {
        // user redefines "node", adds "custom"; keeps default "rust"
        let yaml = "roots: []\nrules:\n  - name: node\n    markers: [package.json]\n    targets: [node_modules]\n  - name: custom\n    markers: [Foo]\n    targets: [bar]\n";
        let cfg: Config = Config::from_yaml_str(yaml).unwrap();
        let names: Vec<&str> = cfg.rules.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"rust"));   // default preserved
        assert!(names.contains(&"custom")); // user rule added
        let node = cfg.rules.iter().find(|r| r.name == "node").unwrap();
        assert_eq!(node.targets, vec!["node_modules"]); // user override wins, single entry
    }

    #[test]
    fn expands_tilde_in_paths() {
        let home = dirs::home_dir().unwrap();
        let yaml = "roots:\n  - path: ~/proj\nignore:\n  - ~/skip\n";
        let cfg: Config = Config::from_yaml_str(yaml).unwrap();
        assert_eq!(cfg.roots[0].path, home.join("proj"));
        assert_eq!(cfg.ignore[0], home.join("skip"));
    }
}
