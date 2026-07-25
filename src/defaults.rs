use crate::config::Rule;

pub fn default_rules() -> Vec<Rule> {
    fn r(name: &str, markers: &[&str], targets: &[&str]) -> Rule {
        Rule {
            name: name.into(),
            markers: markers.iter().map(|s| s.to_string()).collect(),
            targets: targets.iter().map(|s| s.to_string()).collect(),
        }
    }
    vec![
        r("rust", &["Cargo.toml"], &["target"]),
        r("node", &["package.json"], &["node_modules", ".next", "dist"]),
        r("python", &["pyproject.toml", "setup.py"], &[".venv", "__pycache__", "build"]),
        // Go's build cache is global (~/Library/Caches/go-build), so there's
        // nothing to exclude per-project.
        r("go", &["go.mod"], &[]),
        // Real Xcode DerivedData lives at ~/Library/Developer/Xcode/DerivedData
        // (global, out of scope for per-project rules). This rule matches
        // SwiftPM's local .build dir instead.
        r("swiftpm", &["Package.swift"], &[".build"]),
        r("gradle", &["build.gradle", "build.gradle.kts"], &["build", ".gradle"]),
    ]
}
