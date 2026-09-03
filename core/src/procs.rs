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

    /// Profile directories that running processes named `command` record in `var`.
    ///
    /// Only what a process actually recorded: one that sets nothing contributes
    /// nothing here, because the caller already knows the default it is running
    /// under and has no use for being told it back.
    ///
    /// Neutral in the same way `pids_of_command` is, and for the same reason: the
    /// caller names the command and the variable, so no agent's name enters this
    /// trait. Which two words to pass is the one agent-specific fact here, and it
    /// belongs to the adapter that owns the agent — the same conclusion story 003
    /// reached about liveness.
    ///
    /// A session started under a profile the pet has never seen becomes visible
    /// through this, with no configuration and no restart.
    fn profile_dirs_of_command(&self, command: &str, var: &str) -> Vec<PathBuf>;

    /// One poll is starting; forget anything cached only for the length of one.
    ///
    /// A table that answers several questions from one snapshot needs a moment to
    /// take a fresh one, and that moment is the tick, not a stopwatch: a listing
    /// held past the end of a poll could keep an exited session on the surface,
    /// which is the one thing every liveness rule here exists to prevent. Default
    /// empty, because a table that caches nothing has nothing to drop.
    fn begin_poll(&self) {}

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
    /// Keyed by command as well as PID: two adapters ask about two different
    /// commands, and one asking must not evict what the other learned.
    dir_cache: Mutex<HashMap<(String, u32), Option<PathBuf>>>,
    /// This poll's process listing, dropped when the next poll begins.
    ///
    /// `ps -Ao pid=,comm=` costs ~35 ms here and three callers want it within one
    /// tick: each adapter asking which of its own processes are running, and the
    /// Codex adapter asking again on its way to their open files. Paying three
    /// times over, on the thread drawing the surface, buys an answer that cannot
    /// change inside one tick's snapshot. Cleared by `begin_poll` rather than by
    /// any elapsed time, so a listing can never outlive the poll that took it,
    /// however the poll interval is later tuned.
    listing: Mutex<Option<String>>,
}

impl SystemProcessTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read one process's profile directory out of its own environment.
    ///
    /// `None` where the environment could not be read, and equally where the
    /// process set nothing: both mean this process has no directory of its own to
    /// contribute, and the caller supplies the default either way.
    fn profile_dir_of(&self, pid: u32, var: &str) -> Option<PathBuf> {
        // `ps e` prints the environment only when a PID is named explicitly.
        let out = Command::new("ps")
            .args(["eww", "-o", "command=", "-p", &pid.to_string()])
            .output()
            .ok()?;
        dir_in_environment(&String::from_utf8_lossy(&out.stdout), var)
    }

    /// One `ps -Ao pid=,comm=` shared by everything that asks inside a poll.
    fn listing(&self) -> String {
        let mut held = self.listing.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(text) = held.as_ref() {
            return text.clone();
        }
        let Ok(out) = Command::new("ps").args(["-Ao", "pid=,comm="]).output() else {
            // Leave the stale listing in place rather than caching a failure: the
            // next caller retries instead of inheriting an empty machine.
            return String::new();
        };
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        *held = Some(text.clone());
        text
    }
}

impl ProcessTable for SystemProcessTable {
    fn begin_poll(&self) {
        *self.listing.lock().unwrap_or_else(|p| p.into_inner()) = None;
    }

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
        pids_in_listing(&self.listing(), name)
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

    fn profile_dirs_of_command(&self, command: &str, var: &str) -> Vec<PathBuf> {
        let live_pids = self.pids_of_command(command);
        // Recovered rather than surrendered to: a panic under this lock once cost
        // every learned directory for the life of the process, which is every
        // custom-profile session silently absent with the poll still reporting ok.
        let mut cache = self.dir_cache.lock().unwrap_or_else(|p| p.into_inner());
        cached_profile_dirs(&mut cache, command, &live_pids, |pid| {
            self.profile_dir_of(pid, var)
        })
    }
}

/// The value of `var` in one `ps eww -o command=` line, or `None` where the line
/// does not carry it.
///
/// That output is a process's argv followed by its environment, all separated by
/// spaces, and two things follow from that. The *last* match wins: a launcher's
/// own arguments can carry `VAR=...` and the real environment is printed after
/// them. And a value runs to the next `KEY=` token rather than to the next space,
/// because a profile directory can legitimately contain one — anything under
/// `~/Library/Application Support` does — and cutting at the space yields a
/// directory that does not exist, which the pet shows as a session that never
/// appears rather than as an error.
///
/// Known limit: a process that never sets the variable, but carries the text in
/// its own arguments — `claude -p "... CODEX_HOME=/tmp/x ..."` — is read as
/// setting it, because `ps` gives no marker for where argv ends. The cost is one
/// extra directory scanned: a row still needs that agent's own liveness proof and
/// its own file shapes, so a directory nobody's session lives in produces
/// nothing. Ruling it out needs `KERN_PROCARGS2`, which hands back argc.
fn dir_in_environment(text: &str, var: &str) -> Option<PathBuf> {
    let prefix = format!("{var}=");
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let at = tokens.iter().rposition(|t| t.starts_with(&prefix))?;
    let mut value = tokens[at][prefix.len()..].to_string();
    for token in &tokens[at + 1..] {
        if is_assignment(token) {
            break;
        }
        value.push(' ');
        value.push_str(token);
    }
    (!value.is_empty()).then(|| PathBuf::from(value))
}

/// Whether a token opens a new environment entry: a variable name, then `=`.
///
/// Deliberately not upper-case-only. A real macOS environment carries
/// `__CFBundleIdentifier`, `MallocNanoZone`, `npm_config_cache` and friends, and
/// treating those as part of the previous value is how a profile directory with a
/// space in it swallows the rest of the environment.
fn is_assignment(token: &str) -> bool {
    let Some((key, _)) = token.split_once('=') else {
        return false;
    };
    let mut chars = key.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// PIDs whose command is `name`, out of a `ps -Ao pid=,comm=` listing.
///
/// `comm` is sometimes a bare name and sometimes a full path, depending on how
/// the process was launched; both spellings name the same command.
fn pids_in_listing(text: &str, name: &str) -> Vec<u32> {
    let suffix = format!("/{name}");
    let mut pids = Vec::new();
    for line in text.lines() {
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

/// The learned directories of one command's live processes, reading each
/// process's environment at most once for as long as it runs.
///
/// A running process cannot change its own environment, so re-reading it every
/// tick spends battery to learn something that cannot have changed. Split out
/// from the process table so both halves of that — reading once, and one
/// command's ask never evicting another's — are testable without a machine.
///
/// Keyed by PID and nothing else, so a PID recycled onto a new process of the
/// same command within one tick inherits the dead process's directory until it
/// exits. The window is one tick and the cost is one directory watched that need
/// not be; ruling it out would mean carrying each process's start time here too,
/// which is a second `ps` per tick for a case macOS makes rare.
fn cached_profile_dirs(
    cache: &mut HashMap<(String, u32), Option<PathBuf>>,
    command: &str,
    live_pids: &[u32],
    read: impl Fn(u32) -> Option<PathBuf>,
) -> Vec<PathBuf> {
    // Processes that have exited stop being worth remembering, and only this
    // command's entries are this call's to forget.
    cache.retain(|(c, pid), _| c != command || live_pids.contains(pid));

    let mut dirs = Vec::new();
    for pid in live_pids {
        let key = (command.to_string(), *pid);
        let dir = match cache.get(&key) {
            Some(known) => known.clone(),
            None => {
                let found = read(*pid);
                cache.insert(key, found.clone());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Real `ps eww -o command=` output, reduced: argv first, environment after.
    const PS_LINE: &str =
        "node /opt/bin/claude --resume SHLVL=1 CLAUDE_CONFIG_DIR=/Users/x/.dev TERM=xterm-256color";

    #[test]
    fn a_variable_is_read_out_of_the_environment() {
        assert_eq!(
            dir_in_environment(PS_LINE, "CLAUDE_CONFIG_DIR"),
            Some(PathBuf::from("/Users/x/.dev"))
        );
    }

    #[test]
    fn a_variable_the_process_does_not_set_is_absent_rather_than_empty() {
        assert_eq!(dir_in_environment(PS_LINE, "CODEX_HOME"), None);
    }

    #[test]
    fn a_profile_directory_containing_a_space_is_read_whole() {
        // Cut at the space, this reads `/Users/x/Library/Application`, which is a
        // directory that does not exist and a session that never appears.
        let line = "codex --sandbox SHLVL=1 CODEX_HOME=/Users/x/Library/Application Support/codex LANG=en_US.UTF-8";
        assert_eq!(
            dir_in_environment(line, "CODEX_HOME"),
            Some(PathBuf::from("/Users/x/Library/Application Support/codex"))
        );
    }

    #[test]
    fn a_spaced_directory_stops_at_the_next_variable_whatever_its_case() {
        // A real macOS environment carries `__CFBundleIdentifier`, `MallocNanoZone`
        // and `npm_config_*`; stopping only at upper-case names swallowed them into
        // the path, which is the very failure the spacing rule exists to prevent.
        let line = "codex CODEX_HOME=/Users/x/My Profile __CFBundleIdentifier=com.apple.Terminal LANG=C";
        assert_eq!(
            dir_in_environment(line, "CODEX_HOME"),
            Some(PathBuf::from("/Users/x/My Profile"))
        );
    }

    #[test]
    fn a_variable_set_to_nothing_reports_nothing() {
        assert_eq!(dir_in_environment("codex CODEX_HOME= LANG=C", "CODEX_HOME"), None);
    }

    #[test]
    fn the_environment_wins_over_the_same_text_in_the_arguments() {
        // A launcher that prints the assignment as one of its own arguments; the
        // environment is what the process actually runs under, and comes second.
        let line = "sh -c CODEX_HOME=/decoy codex SHLVL=1 CODEX_HOME=/Users/x/.codex-work";
        assert_eq!(
            dir_in_environment(line, "CODEX_HOME"),
            Some(PathBuf::from("/Users/x/.codex-work"))
        );
    }

    #[test]
    fn both_spellings_of_a_command_name_are_the_same_command() {
        let listing = "  401 claude\n  402 /opt/homebrew/bin/claude\n  403 claude-desktop\n  404 node\n";
        assert_eq!(pids_in_listing(listing, "claude"), vec![401, 402]);
    }

    #[test]
    fn a_listing_that_makes_no_sense_yields_no_pids() {
        assert_eq!(pids_in_listing("", "codex"), Vec::<u32>::new());
        assert_eq!(pids_in_listing("garbage\n\n  x claude\n", "claude"), Vec::<u32>::new());
    }

    /// Counts how many times a PID's environment was actually read.
    fn counting_reader(log: &RefCell<Vec<u32>>) -> impl Fn(u32) -> Option<PathBuf> + '_ {
        move |pid| {
            log.borrow_mut().push(pid);
            Some(PathBuf::from(format!("/profiles/{pid}")))
        }
    }

    #[test]
    fn a_running_process_is_read_once_however_many_ticks_pass() {
        let mut cache = HashMap::new();
        let log = RefCell::new(Vec::new());
        for _ in 0..5 {
            let out = cached_profile_dirs(&mut cache, "claude", &[7], counting_reader(&log));
            assert_eq!(out, vec![PathBuf::from("/profiles/7")]);
        }
        assert_eq!(log.borrow().len(), 1, "the environment was re-read: {:?}", log.borrow());
    }

    #[test]
    fn one_commands_ask_does_not_forget_what_another_command_learned() {
        // Both adapters ask on every tick. Keyed by PID alone, each ask evicted
        // the other's entries and every process was re-read every tick.
        let mut cache = HashMap::new();
        let log = RefCell::new(Vec::new());
        cached_profile_dirs(&mut cache, "claude", &[7], counting_reader(&log));
        cached_profile_dirs(&mut cache, "codex", &[9], counting_reader(&log));
        cached_profile_dirs(&mut cache, "claude", &[7], counting_reader(&log));
        cached_profile_dirs(&mut cache, "codex", &[9], counting_reader(&log));
        assert_eq!(*log.borrow(), vec![7, 9], "an entry was evicted and re-read");
    }

    #[test]
    fn a_process_that_has_exited_stops_being_remembered() {
        let mut cache = HashMap::new();
        let log = RefCell::new(Vec::new());
        cached_profile_dirs(&mut cache, "claude", &[7], counting_reader(&log));
        cached_profile_dirs(&mut cache, "claude", &[], counting_reader(&log));
        assert!(cache.is_empty(), "a dead process is still cached: {cache:?}");
    }

    #[test]
    fn a_process_whose_environment_cannot_be_read_yields_no_directory() {
        let mut cache = HashMap::new();
        let out = cached_profile_dirs(&mut cache, "claude", &[7], |_| None);
        assert!(out.is_empty());
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
    /// How many polls this table has been told are starting.
    pub polls: std::cell::Cell<u32>,
    pub starts: HashMap<u32, String>,
    /// What a command's running processes report for one variable, keyed by
    /// `(command, variable)`. Keyed by both so a test can tell an adapter asking
    /// under the wrong variable from one asking under its own: the wrong
    /// variable learns nothing, exactly as it would on the machine.
    pub dirs: HashMap<(String, String), Vec<PathBuf>>,
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
    fn begin_poll(&self) {
        self.polls.set(self.polls.get() + 1);
    }

    fn profile_dirs_of_command(&self, command: &str, var: &str) -> Vec<PathBuf> {
        // Nothing running under that command learns nothing, as on the machine:
        // a profile is reported by a process, not by a directory existing.
        match self.named.get(command) {
            Some(pids) if !pids.is_empty() => self
                .dirs
                .get(&(command.to_string(), var.to_string()))
                .cloned()
                .unwrap_or_default(),
            _ => Vec::new(),
        }
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
