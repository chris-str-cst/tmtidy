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
        r("go", &["go.mod"], &["bin"]),
        r("xcode", &["Package.swift"], &[".build"]),
        r("gradle", &["build.gradle", "build.gradle.kts"], &["build", ".gradle"]),
    ]
}
