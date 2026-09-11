//! Talking to native 1984's own `--monitor-pty` debug monitor: a plain-text,
//! minicom-compatible line protocol over a PTY, printed once to the
//! process's own stderr at startup (`1984: monitor PTY: /dev/pts/N`).
//!
//! Read-only: confirmed live against a real 2.1.1-era build that its
//! command set (`D`/`M`/`B`/`BC`/`N`/`G`/`GA`/`CRTC`/`X`/`Q`) has no memory
//! *write* command at all - only `M <addr> [<end>]` (a hex+ASCII dump) is
//! usable here, so this module offers `read_memory` and nothing else.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};

use cpclib_common::camino::Utf8Path;

pub struct Monitor {
    reader: BufReader<File>,
    writer: File
}

/// Open the PTY device the process printed at startup.
pub fn connect(pty_path: &Utf8Path) -> Result<Monitor, String> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(pty_path)
        .map_err(|e| format!("cannot open the monitor PTY at {pty_path}: {e}"))?;
    let mut writer =
        file.try_clone().map_err(|e| format!("cannot clone the monitor PTY handle: {e}"))?;
    // Confirmed live: the monitor's own read loop does not appear to be
    // listening the instant the PTY is opened - a real command sent as the
    // very first write was never answered at all. One priming CRLF (no
    // read-back needed here) wakes it; whatever prompt echo it produces is
    // just more noise `read_memory`'s own line-skipping already tolerates.
    let _ = writer.write_all(b"\r\n");
    Ok(Monitor {
        reader: BufReader::new(file),
        writer
    })
}

impl Monitor {
    /// `M <addr> <end>` (both plain hex, no `0x` prefix - confirmed live
    /// that the monitor reads bare digits as hex, matching an old-school
    /// Z80 monitor convention). Every line of the reply that looks like
    /// `>NNNNN xx xx ...: ....` contributes its bytes; every other line
    /// (the command echo, the trailing prompt) is silently skipped, so
    /// there is no need to separately recognise where the reply starts or
    /// ends - only where the hex-dump lines are.
    pub fn read_memory(&mut self, address: u16, count: u16) -> Result<Vec<u8>, String> {
        if count == 0 {
            return Ok(Vec::new());
        }
        let end = address.saturating_add(count.saturating_sub(1));
        let command = format!("M {address:X} {end:X}\r\n");
        self.writer
            .write_all(command.as_bytes())
            .map_err(|e| format!("cannot send to the monitor: {e}"))?;

        let mut bytes = Vec::with_capacity(count as usize);
        // A generous but bounded number of *real* lines - the echo and the
        // trailing prompt each take a handful, plus one hex-dump line per 8
        // bytes. Separately bounded retries for `read_line` returning 0:
        // confirmed live, repeatedly, that 0 here is *not* end-of-stream on
        // this PTY - a real reply reliably arrives a short retry later, and
        // treating the very first 0 as a hard close (as a plain file or a
        // socket would warrant) made every read fail after only the
        // greeting, despite the process being very much alive underneath.
        // Whatever non-canonical/timed-read termios mode this PTY inherits
        // is producing timeouts shaped exactly like EOF, not real hangups.
        //
        // Bounded by wall-clock time, not by a retry count: a fixed count *
        // fixed sleep (previously 50 * 20ms = 1s total) turned out to be
        // flaky live - on a loaded machine (this box also builds/runs other
        // things concurrently) the emulator can take longer than that just
        // to finish booting before its monitor loop is truly pumping, which
        // a retry-count budget has no way to account for.
        let max_lines = 32 + (count as usize / 8);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let mut lines_seen = 0;
        while lines_seen < max_lines {
            let mut line = String::new();
            let n = self
                .reader
                .read_line(&mut line)
                .map_err(|e| format!("cannot read from the monitor: {e}"))?;
            if n == 0 {
                if std::time::Instant::now() >= deadline {
                    return Err("the monitor PTY closed while reading a reply".to_owned());
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
                continue;
            }
            lines_seen += 1;
            if let Some(mut line_bytes) = parse_hex_dump_line(line.trim_end()) {
                bytes.append(&mut line_bytes);
                if bytes.len() >= count as usize {
                    bytes.truncate(count as usize);
                    return Ok(bytes);
                }
            }
        }
        Err(format!(
            "the monitor never sent {count} bytes for address {address:#06x} (got {})",
            bytes.len()
        ))
    }
}

/// `>NNNNN xx xx xx xx xx xx xx xx: <ascii, possibly ANSI-coloured>` -> the
/// hex bytes, or `None` for any other line (the command echo, the prompt).
fn parse_hex_dump_line(line: &str) -> Option<Vec<u8>> {
    let line = line.strip_prefix('>')?;
    let (addr, rest) = line.split_once(' ')?;
    if addr.len() != 5 || !addr.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let (hex_part, _ascii_part) = rest.split_once(':')?;
    let mut bytes = Vec::new();
    for token in hex_part.split_whitespace() {
        if token.len() != 2 {
            return None;
        }
        bytes.push(u8::from_str_radix(token, 16).ok()?);
    }
    if bytes.is_empty() { None } else { Some(bytes) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_real_hex_dump_line() {
        assert_eq!(
            parse_hex_dump_line(">00000 01 89 7F ED 49 C3 91 05: ....I..."),
            Some(vec![0x01, 0x89, 0x7F, 0xED, 0x49, 0xC3, 0x91, 0x05])
        );
    }

    #[test]
    fn parses_a_short_final_line() {
        assert_eq!(parse_hex_dump_line(">00010 C3                     : ."), Some(vec![0xC3]));
    }

    #[test]
    fn ignores_the_command_echo_and_prompt_lines() {
        assert_eq!(parse_hex_dump_line("M 0 10"), None);
        assert_eq!(parse_hex_dump_line("> M 0 10"), None);
        assert_eq!(parse_hex_dump_line(">_ "), None);
        assert_eq!(parse_hex_dump_line(">"), None);
    }
}
