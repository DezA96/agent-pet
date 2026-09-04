use crate::config::{self, Config};
use std::path::PathBuf;

/// Every directory worth looking in this tick.
///
/// Rebuilt from scratch on each tick rather than resolved once at startup, which
/// is what makes startup an ordinary tick and lets a session launched later —
/// including one under a profile directory never seen before — appear within one
/// tick with no restart and no configuration.
///
/// Two sources, unioned: `learned` — every directory the adapters asked for, each
/// adapter's own default among them — and whatever the user put in the config
/// file. This function names no agent, reads no process and knows no default:
/// which directories an agent keeps sessions in is that agent's own fact, and the
/// pet only unions the answers and drops the duplicates.
pub fn candidate_directories(cfg: &Config, learned: &[PathBuf]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut push = |p: PathBuf| {
        // Resolve where possible so two spellings of one directory collapse.
        let p = std::fs::canonicalize(&p).unwrap_or(p);
        if !out.contains(&p) {
            out.push(p);
        }
    };

    for d in learned {
        push(d.clone());
    }
    for d in &cfg.watch_directories {
        push(config::expand_tilde(d));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_directory_learned_from_a_live_process_is_included() {
        let learned = std::env::temp_dir().join("agentpet-learned-profile");
        std::fs::create_dir_all(&learned).unwrap();
        let out = candidate_directories(&Config::default(), std::slice::from_ref(&learned));
        let learned = std::fs::canonicalize(&learned).unwrap();
        assert!(out.contains(&learned));
    }

    #[test]
    fn the_same_directory_from_two_sources_appears_once() {
        let shared = std::env::temp_dir().join("agentpet-shared-profile");
        std::fs::create_dir_all(&shared).unwrap();
        let cfg = Config {
            watch_directories: vec![shared.to_string_lossy().into_owned()],
        };
        let out = candidate_directories(&cfg, std::slice::from_ref(&shared));
        let shared = std::fs::canonicalize(&shared).unwrap();
        assert_eq!(out.iter().filter(|p| **p == shared).count(), 1);
    }
}
