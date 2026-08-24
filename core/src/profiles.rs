use crate::config::{self, Config};
use crate::procs::ProcessTable;
use std::path::PathBuf;

/// Every directory worth looking in this tick.
///
/// Rebuilt from scratch on each tick rather than resolved once at startup, which
/// is what makes startup an ordinary tick and lets a session launched later —
/// including one under a profile directory never seen before — appear within one
/// tick with no restart and no configuration.
///
/// Three sources, unioned: the defaults, whatever the user put in the config file,
/// and the profile directory of every `claude` process running right now.
pub fn candidate_directories(cfg: &Config, procs: &dyn ProcessTable) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut push = |p: PathBuf| {
        // Resolve where possible so two spellings of one directory collapse.
        let p = std::fs::canonicalize(&p).unwrap_or(p);
        if !out.contains(&p) {
            out.push(p);
        }
    };

    for d in config::default_directories() {
        push(d);
    }
    for d in &cfg.watch_directories {
        push(config::expand_tilde(d));
    }
    for d in procs.claude_profile_dirs() {
        push(d);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::procs::FakeProcessTable;

    fn procs(dirs: Vec<PathBuf>) -> FakeProcessTable {
        FakeProcessTable {
            starts: Default::default(),
            dirs,
        }
    }

    #[test]
    fn defaults_are_watched_with_no_configuration() {
        let home = PathBuf::from(std::env::var("HOME").unwrap());
        let out = candidate_directories(&Config::default(), &procs(vec![]));
        let has = |p: PathBuf| {
            let p = std::fs::canonicalize(&p).unwrap_or(p);
            out.contains(&p)
        };
        assert!(has(home.join(".claude")), "default Claude directory missing");
        assert!(has(home.join(".codex")), "default Codex directory missing");
    }

    #[test]
    fn a_directory_learned_from_a_live_process_is_included() {
        let learned = std::env::temp_dir().join("agentpet-learned-profile");
        std::fs::create_dir_all(&learned).unwrap();
        let out = candidate_directories(&Config::default(), &procs(vec![learned.clone()]));
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
        let out = candidate_directories(&cfg, &procs(vec![shared.clone()]));
        let shared = std::fs::canonicalize(&shared).unwrap();
        assert_eq!(out.iter().filter(|p| **p == shared).count(), 1);
    }
}
