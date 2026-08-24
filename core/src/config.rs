use serde::Deserialize;
use std::path::{Path, PathBuf};

/// The pet's own config file. No schema is shared with any agent.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub watch_directories: Vec<String>,
}

#[derive(Debug)]
pub enum ConfigError {
    Malformed { path: PathBuf, detail: String },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Malformed { path, detail } => {
                write!(f, "config file {} is not valid JSON: {}", path.display(), detail)
            }
        }
    }
}

/// `~/.config/agent-pet/config.json`, honouring `XDG_CONFIG_HOME` when set.
pub fn config_path() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(x) if !x.is_empty() => PathBuf::from(x),
        _ => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
    };
    Some(base.join("agent-pet").join("config.json"))
}

/// Load the config, or the defaults when there is no config file.
///
/// A missing file is the normal first-run case and yields defaults. A file that
/// exists but cannot be parsed is an error rather than a silent fallback: reading
/// past a broken config would show the user a directory list they did not ask for.
pub fn load(path: &Path) -> Result<Config, ConfigError> {
    let raw = match std::fs::read_to_string(path) {
        Ok(r) => r,
        Err(_) => return Ok(Config::default()),
    };
    if raw.trim().is_empty() {
        return Ok(Config::default());
    }
    serde_json::from_str(&raw).map_err(|e| ConfigError::Malformed {
        path: path.to_path_buf(),
        detail: e.to_string(),
    })
}

/// Watched by default when the pet starts with no configuration.
///
/// Codex is watched from the start even though this release cannot yet interpret
/// its sessions; they are ignored silently rather than surfaced as an error.
pub fn default_directories() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME") else {
        return Vec::new();
    };
    let home = PathBuf::from(home);
    vec![home.join(".claude"), home.join(".codex")]
}

/// Expand a leading `~` so the config file stays comfortable to hand-edit.
pub fn expand_tilde(raw: &str) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_defaults_not_an_error() {
        let cfg = load(Path::new("/nonexistent/agent-pet/config.json")).unwrap();
        assert!(cfg.watch_directories.is_empty());
    }

    #[test]
    fn malformed_file_is_an_error_not_a_silent_fallback() {
        let dir = std::env::temp_dir().join("agentpet-cfg-malformed");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("config.json");
        std::fs::write(&p, "{ this is not json").unwrap();
        assert!(load(&p).is_err());
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn watch_directories_are_read() {
        let dir = std::env::temp_dir().join("agentpet-cfg-ok");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("config.json");
        std::fs::write(&p, r#"{"watchDirectories":["~/work/.claude"]}"#).unwrap();
        let cfg = load(&p).unwrap();
        assert_eq!(cfg.watch_directories, vec!["~/work/.claude".to_string()]);
        std::fs::remove_file(&p).ok();
    }
}
