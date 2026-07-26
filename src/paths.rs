//! XDG base-directory resolution. We honor `$XDG_CONFIG_HOME` / `$XDG_STATE_HOME`
//! when set to an absolute path, else fall back to the Linux-style `~/.config`
//! and `~/.local/state` (deliberately NOT the `dirs` crate's macOS defaults,
//! which point at `~/Library/…` — this keeps paths familiar across platforms).

use std::ffi::OsString;
use std::path::PathBuf;

/// Base dir for config: `$XDG_CONFIG_HOME`, else `~/.config`.
pub fn config_home() -> PathBuf {
    resolve(std::env::var_os("XDG_CONFIG_HOME"), ".config")
}

/// Base dir for state (logs, run history): `$XDG_STATE_HOME`, else `~/.local/state`.
pub fn state_home() -> PathBuf {
    resolve(std::env::var_os("XDG_STATE_HOME"), ".local/state")
}

/// Use the env value only if it's an absolute path (the XDG spec mandates
/// ignoring relative — and empty — values); otherwise `~/<fallback>`.
fn resolve(env_value: Option<OsString>, fallback: &str) -> PathBuf {
    if let Some(v) = env_value {
        let p = PathBuf::from(v);
        if p.is_absolute() {
            return p;
        }
    }
    dirs::home_dir().unwrap_or_default().join(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn absolute_env_value_is_used_verbatim() {
        let got = resolve(Some(OsString::from("/custom/xdg")), ".config");
        assert_eq!(got, Path::new("/custom/xdg"));
    }

    #[test]
    fn relative_env_value_is_ignored_per_spec() {
        let got = resolve(Some(OsString::from("relative/path")), ".config");
        assert_eq!(got, dirs::home_dir().unwrap_or_default().join(".config"));
    }

    #[test]
    fn empty_env_value_falls_back() {
        let got = resolve(Some(OsString::from("")), ".local/state");
        assert_eq!(got, dirs::home_dir().unwrap_or_default().join(".local/state"));
    }

    #[test]
    fn unset_env_value_falls_back() {
        let got = resolve(None, ".local/state");
        assert_eq!(got, dirs::home_dir().unwrap_or_default().join(".local/state"));
    }
}
