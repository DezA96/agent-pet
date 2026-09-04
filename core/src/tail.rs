//! Reading the tail of a file an agent keeps appending to.
//!
//! Neutral on purpose: what each agent writes is that adapter's business, and
//! this module knows only that the file grows at the end and that the poll
//! reading it runs on the thread drawing the surface.

/// How far back from the end of an unseen file to read.
///
/// Reading a whole file on first sight runs on the thread drawing the surface,
/// and at startup that is every live session at once: a Codex rollout on this
/// disk is 74 MB and a single-turn session reached 1.3 MB, and nothing bounds a
/// Claude transcript either. Starting at bare EOF was rejected as worse: a
/// session already mid-turn would read `unknown` until its next event, breaking
/// the release's within-a-few-seconds spot check.
///
/// Everything the pet derives sits at the tail. For Claude that is the newest
/// tool call and the newest substantive entry; for a settled Codex session,
/// `task_complete` lands 214 to 4,799 bytes from EOF across every rollout on
/// this disk, because it is written as the turn ends. A Codex session still
/// *working* is another case entirely, and `codex::rollout` scans back for its
/// boundary. The cost accepted everywhere is an entry older than this much of
/// the file going unseen on first sight; the next one written is read as it
/// lands.
pub const FIRST_READ_WINDOW: u64 = 256 * 1024;

/// Where reading a file first seen at `len` bytes begins.
pub fn first_sight_start(len: u64) -> u64 {
    len.saturating_sub(FIRST_READ_WINDOW)
}

/// Drop the partial line a mid-file landing produces, and say how many bytes
/// went, so the caller can move its offset to the first whole line.
///
/// That fragment is not parseable JSON and is dropped rather than guessed at.
/// `None` is a buffer holding no line break at all: one line longer than the
/// window, still being written, from which nothing is readable this tick.
pub fn drop_leading_fragment(buf: &mut Vec<u8>) -> Option<u64> {
    let i = buf.iter().position(|b| *b == b'\n')?;
    buf.drain(..=i);
    Some(i as u64 + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_file_is_read_from_its_start_and_a_large_one_from_its_tail() {
        assert_eq!(first_sight_start(0), 0);
        assert_eq!(first_sight_start(FIRST_READ_WINDOW), 0);
        assert_eq!(first_sight_start(FIRST_READ_WINDOW + 1), 1);
    }

    #[test]
    fn a_mid_line_landing_skips_to_the_first_whole_line() {
        let mut buf = b"tail of a line}\n{\"whole\":1}\n".to_vec();
        assert_eq!(drop_leading_fragment(&mut buf), Some(16));
        assert_eq!(buf, b"{\"whole\":1}\n");
    }

    #[test]
    fn a_buffer_with_no_line_break_yields_nothing_and_is_left_alone() {
        let mut buf = b"one line longer than the window".to_vec();
        assert_eq!(drop_leading_fragment(&mut buf), None);
        assert_eq!(buf, b"one line longer than the window");
    }
}
