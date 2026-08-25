use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

/// The pet's read-only window onto the process table.
///
/// Behind a trait so the liveness rule can be tested against fixtures rather than
/// against whatever happens to be running on the machine.
pub trait ProcessTable {
    /// Process start times, keyed by PID, rendered in UTC.
    ///
    /// PIDs that are not running are simply absent from the map. Rendering in UTC
    /// is not cosmetic: Claude Code writes `procStart` in UTC while `ps -o lstart`
    /// prints local time, so comparing the two unnormalised marks every live
    /// session dead.
    fn start_times_utc(&self, pids: &[u32]) -> HashMap<u32, String>;

    /// Profile directories belonging to `claude` processes running right now.
    ///
    /// A session started under a profile the pet has never seen becomes visible
    /// through this, with no configuration and no restart.
    fn claude_profile_dirs(&self) -> Vec<PathBuf>;

    /// PIDs of every running process whose command is `name`.
    ///
    /// Deliberately neutral: the caller names the command, so no agent's name
    /// enters this trait. Bearing the name is not by itself evidence of a
    /// session — ChatGPT.app runs a process called exactly `codex` — so what an
    /// adapter concludes from the answer stays the adapter's business.
    fn pids_of_command(&self, name: &str) -> Vec<u32>;

    /// The file paths each of these PIDs currently holds open.
    ///
    /// This is the strongest liveness evidence available for an agent that
    /// records no PID of its own: a held handle belongs to a process that exists
    /// right now, so nothing read through it can be stale.
    fn open_paths(&self, pids: &[u32]) -> HashMap<u32, Vec<PathBuf>>;
}

/// Talks to `ps`.
///
/// Caches each process's profile directory by PID. A running process cannot
/// change its own environment, so re-reading it every tick spends battery to
/// learn something that cannot have changed; newly appeared PIDs are still read
/// immediately, which is what keeps a session launched moments ago visible.
#[derive(Default)]
pub struct SystemProcessTable {
    dir_cache: Mutex<HashMap<u32, Option<PathBuf>>>,
}

impl SystemProcessTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read `CLAUDE_CONFIG_DIR` out of one process's environment.
    fn profile_dir_of(&self, pid: u32) -> Option<PathBuf> {
        // `ps e` prints the environment only when a PID is named explicitly.
        let out = Command::new("ps")
            .args(["eww", "-o", "command=", "-p", &pid.to_string()])
            .output()
            .ok()?;
        let env = String::from_utf8_lossy(&out.stdout);
        match env
            .split_whitespace()
            .find_map(|t| t.strip_prefix("CLAUDE_CONFIG_DIR="))
        {
            Some(dir) => Some(PathBuf::from(dir)),
            // No override means the process is using the default profile.
            None => std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".claude")),
        }
    }
}

impl ProcessTable for SystemProcessTable {
    fn start_times_utc(&self, pids: &[u32]) -> HashMap<u32, String> {
        let mut out = HashMap::new();
        if pids.is_empty() {
            return out;
        }
        let list = pids
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(",");
        // One call for every PID at once; dead PIDs are omitted from the output.
        let res = Command::new("ps")
            .env("TZ", "UTC")
            .args(["-o", "pid=,lstart=", "-p", &list])
            .output();
        let Ok(res) = res else { return out };
        for line in String::from_utf8_lossy(&res.stdout).lines() {
            let line = line.trim();
            let Some((pid, start)) = line.split_once(char::is_whitespace) else {
                continue;
            };
            if let Ok(pid) = pid.trim().parse::<u32>() {
                out.insert(pid, start.trim().to_string());
            }
        }
        out
    }

    fn pids_of_command(&self, name: &str) -> Vec<u32> {
        let Ok(listing) = Command::new("ps").args(["-Ao", "pid=,comm="]).output() else {
            return Vec::new();
        };
        // `comm` is sometimes a bare name and sometimes a full path, depending on
        // how the process was launched; both spellings name the same command.
        let suffix = format!("/{name}");
        let mut pids = Vec::new();
        for line in String::from_utf8_lossy(&listing.stdout).lines() {
            let Some((pid, comm)) = line.trim().split_once(char::is_whitespace) else {
                continue;
            };
            let comm = comm.trim();
            if comm == name || comm.ends_with(&suffix) {
                if let Ok(pid) = pid.trim().parse::<u32>() {
                    pids.push(pid);
                }
            }
        }
        pids
    }

    fn open_paths(&self, pids: &[u32]) -> HashMap<u32, Vec<PathBuf>> {
        let mut out = HashMap::new();
        if pids.is_empty() {
            return out;
        }
        let list = pids
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(",");
        // One call for every PID at once. `-F` emits a field-per-line stream:
        // `p<pid>` opens a process's record, `n<path>` names one open file.
        // `-w` suppresses the warnings an unreadable mount would otherwise print.
        let Ok(res) = Command::new("lsof")
            .args(["-w", "-Fpn", "-p", &list])
            .output()
        else {
            return out;
        };
        let mut current: Option<u32> = None;
        for line in String::from_utf8_lossy(&res.stdout).lines() {
            let Some((tag, rest)) = line.split_at_checked(1) else {
                continue;
            };
            match tag {
                "p" => current = rest.parse::<u32>().ok(),
                "n" if rest.starts_with('/') => {
                    if let Some(pid) = current {
                        out.entry(pid).or_insert_with(Vec::new).push(PathBuf::from(rest));
                    }
                }
                _ => {}
            }
        }
        out
    }

    fn claude_profile_dirs(&self) -> Vec<PathBuf> {
        let live_pids = self.pids_of_command("claude");

        let Ok(mut cache) = self.dir_cache.lock() else {
            return Vec::new();
        };
        // Processes that have exited stop being worth remembering.
        cache.retain(|pid, _| live_pids.contains(pid));

        let mut dirs = Vec::new();
        for pid in live_pids {
            let dir = match cache.get(&pid) {
                Some(known) => known.clone(),
                None => {
                    let found = self.profile_dir_of(pid);
                    cache.insert(pid, found.clone());
                    found
                }
            };
            if let Some(dir) = dir {
                if !dirs.contains(&dir) {
                    dirs.push(dir);
                }
            }
        }
        dirs
    }
}

/// A process table supplied by a test rather than by the machine.
///
/// `named` and `open` are what let Codex liveness be tested against fixtures:
/// the test states which PIDs bear a command name and which files each holds
/// open, so the join is exercised with no agent running anywhere.
#[cfg(test)]
#[derive(Default)]
pub struct FakeProcessTable {
    pub starts: HashMap<u32, String>,
    pub dirs: Vec<PathBuf>,
    pub named: HashMap<String, Vec<u32>>,
    pub open: HashMap<u32, Vec<PathBuf>>,
}

#[cfg(test)]
impl ProcessTable for FakeProcessTable {
    fn start_times_utc(&self, pids: &[u32]) -> HashMap<u32, String> {
        pids.iter()
            .filter_map(|p| self.starts.get(p).map(|s| (*p, s.clone())))
            .collect()
    }
    fn claude_profile_dirs(&self) -> Vec<PathBuf> {
        self.dirs.clone()
    }
    fn pids_of_command(&self, name: &str) -> Vec<u32> {
        self.named.get(name).cloned().unwrap_or_default()
    }
    fn open_paths(&self, pids: &[u32]) -> HashMap<u32, Vec<PathBuf>> {
        pids.iter()
            .filter_map(|p| self.open.get(p).map(|v| (*p, v.clone())))
            .collect()
    }
}
