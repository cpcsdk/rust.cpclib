//! Shell-style `<file`/`>file`/`>>file`/`|` parsing for a bndbuild command
//! line, and the execution of the resulting `InnerTask::Pipe`.
//!
//! Quote-aware, mirroring `shlex`'s own tokenizing rules exactly (single
//! quotes: fully literal, no escapes; double quotes: only `\$`/`` \` ``/`\"`/
//! `\\`/a swallowed `\<newline>` are meaningful, anything else keeps its
//! backslash; unquoted: a backslash escapes the immediate next character) -
//! `get_all_args` tokenizes with `shlex::split` downstream of this, on
//! whatever text this module leaves behind, so any disagreement here about
//! what counts as "inside quotes" would silently misparse a real build file.
//!
//! One bndbuild-specific wrinkle this must not break: `$<`/`$@` are the
//! Makefile-style automatic-variable markers (first dependency/target,
//! substituted later by `StandardTaskArguments::replace_automatic_variables`,
//! well after this module runs) - a `<` immediately preceded by `$` is never
//! stdin redirection, only a bare `<` is.

use std::io::Write;
use std::sync::{Arc, Mutex};

use camino::Utf8PathBuf;
use cpclib_common::event::EventObserver;
use cpclib_runner::runner::{TaskStdin, TaskStdout};

use crate::event::{BndBuilderEvent, BndBuilderObserver};
use crate::task::{PipeTaskArguments, TaskKind};

/// One redirection target, already dequoted (via `shlex::split` on just its
/// own token, reusing the same tokenizer everything downstream trusts,
/// rather than re-implementing dequoting a second time).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RedirSpec {
    Stdin(Utf8PathBuf),
    StdoutTruncate(Utf8PathBuf),
    StdoutAppend(Utf8PathBuf)
}

#[derive(Clone, Copy, PartialEq)]
enum Quote {
    None,
    Single,
    Double
}

/// Every byte position of an unquoted, unescaped `|`, `<`, or `>` in `line`,
/// alongside which character it is. Shared notion of "outside quotes" for
/// both pipe-splitting and redirection-stripping, so the two never disagree
/// about the same input.
fn top_level_operators(line: &str) -> Vec<(usize, char)> {
    let mut quote = Quote::None;
    let mut escaped = false;
    let mut found = Vec::new();

    for (i, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match quote {
            Quote::Single => {
                if ch == '\'' {
                    quote = Quote::None;
                }
            },
            Quote::Double => {
                match ch {
                    '\\' => escaped = true,
                    '"' => quote = Quote::None,
                    _ => {}
                }
            },
            Quote::None => {
                match ch {
                    '\\' => escaped = true,
                    '\'' => quote = Quote::Single,
                    '"' => quote = Quote::Double,
                    '|' | '<' | '>' => found.push((i, ch)),
                    _ => {}
                }
            }
        }
    }
    found
}

/// Split `line` on top-level (outside quotes) unquoted `|`. A line with no
/// such `|` returns a single-element vec borrowing the whole input - the
/// overwhelmingly common case allocates nothing beyond that one element.
pub fn split_top_level_pipes(line: &str) -> Vec<&str> {
    let pipe_positions: Vec<usize> = top_level_operators(line)
        .into_iter()
        .filter(|(_, ch)| *ch == '|')
        .map(|(i, _)| i)
        .collect();
    if pipe_positions.is_empty() {
        return vec![line];
    }
    let mut segments = Vec::with_capacity(pipe_positions.len() + 1);
    let mut start = 0;
    for pos in pipe_positions {
        segments.push(&line[start..pos]);
        start = pos + 1; // '|' is one byte
    }
    segments.push(&line[start..]);
    segments
}

/// How many bytes, starting at the beginning of `s`, make up one shlex-style
/// word (respecting quotes/escapes, same rules as `top_level_operators`),
/// stopping at the first unquoted whitespace/operator or the end of `s`.
/// Errors if `s` ends while still inside a quote - matches `shlex::split`
/// itself refusing an unterminated quote.
fn word_len(s: &str) -> Result<usize, String> {
    let mut quote = Quote::None;
    let mut escaped = false;
    let mut end = s.len();

    for (i, ch) in s.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match quote {
            Quote::Single => {
                if ch == '\'' {
                    quote = Quote::None;
                }
            },
            Quote::Double => {
                match ch {
                    '\\' => escaped = true,
                    '"' => quote = Quote::None,
                    _ => {}
                }
            },
            Quote::None => {
                match ch {
                    '\\' => escaped = true,
                    '\'' => quote = Quote::Single,
                    '"' => quote = Quote::Double,
                    ' ' | '\t' | '\n' | '|' | '<' | '>' => {
                        end = i;
                        break;
                    },
                    _ => {}
                }
            }
        }
    }
    if quote != Quote::None {
        return Err(format!("unterminated quote in redirection target: {s}"));
    }
    Ok(end)
}

/// The redirection target starting right after a `<`/`>`/`>>` operator that
/// ended at byte offset `operator_end` within `text`. Returns the dequoted
/// filename and the byte offset (within `text`) where the target token ends.
fn extract_target(text: &str, operator_end: usize) -> Result<(Utf8PathBuf, usize), String> {
    let after = &text[operator_end..];
    let skipped = after.len() - after.trim_start_matches([' ', '\t', '\n']).len();
    let token_start = operator_end + skipped;
    let token_str = &text[token_start..];
    let token_len = word_len(token_str)?;
    if token_len == 0 {
        return Err("expected a filename after a redirection operator".to_string());
    }
    let raw_token = &token_str[..token_len];
    let words = shlex::split(raw_token)
        .ok_or_else(|| format!("invalid quoting in redirection target: {raw_token}"))?;
    let filename = words
        .into_iter()
        .next()
        .ok_or_else(|| "empty redirection target".to_string())?;
    Ok((Utf8PathBuf::from(filename), token_start + token_len))
}

/// Strip every `<file`/`>file`/`>>file` from `segment`, left to right,
/// returning the remaining text (still exactly what the existing per-keyword
/// parser expects) plus whichever redirections were found. A `<` immediately
/// preceded by `$` (the `$<` automatic-variable marker) is never treated as
/// redirection. More than one stdin or stdout redirection on the same
/// segment is a hard error, not a silent "last one wins."
pub fn strip_redirections(
    segment: &str
) -> Result<(String, Option<Utf8PathBuf>, Option<RedirSpec>), String> {
    let mut stdin: Option<Utf8PathBuf> = None;
    let mut stdout: Option<RedirSpec> = None;
    let mut remaining = String::new();
    let mut pos = 0usize;

    loop {
        let tail = &segment[pos..];
        let ops = top_level_operators(tail);

        let mut chosen: Option<(usize, char, usize)> = None;
        for (i, ch) in &ops {
            match ch {
                '<' => {
                    if *i > 0 && tail[..*i].ends_with('$') {
                        // `$<` - the first-dependency automatic variable, not redirection.
                        continue;
                    }
                    chosen = Some((*i, '<', 1));
                    break;
                },
                '>' => {
                    let append = tail[i + 1..].starts_with('>');
                    chosen = Some((*i, '>', if append { 2 } else { 1 }));
                    break;
                },
                _ => {}
            }
        }

        let Some((op_pos, op_char, op_len)) = chosen
        else {
            remaining.push_str(tail);
            break;
        };

        remaining.push_str(&tail[..op_pos]);
        let operator_end = op_pos + op_len;
        let (target, consumed_end) = extract_target(tail, operator_end)?;

        match op_char {
            '<' => {
                if stdin.is_some() {
                    return Err("only one stdin redirection (`<`) is allowed per command".into());
                }
                stdin = Some(target);
            },
            '>' => {
                if stdout.is_some() {
                    return Err(
                        "only one stdout redirection (`>`/`>>`) is allowed per command".into()
                    );
                }
                stdout = Some(if op_len == 2 {
                    RedirSpec::StdoutAppend(target)
                }
                else {
                    RedirSpec::StdoutTruncate(target)
                });
            },
            _ => unreachable!("top_level_operators only ever finds '|', '<', '>'")
        }

        pos += consumed_end;
    }

    Ok((remaining, stdin, stdout))
}

/// Where a `RedirectedObserver`'s stdout ends up: a `>`/`>>` target file, or
/// the write end of an inter-stage `std::io::pipe()`. `None` (in
/// `RedirectedObserver::sink`) means "pass through to the wrapped observer
/// unchanged" - the last stage of a pipeline with no `>`/`>>`.
enum StdoutSink {
    File(Mutex<fs_err::File>),
    Pipe(Mutex<std::io::PipeWriter>)
}

/// An [`EventObserver`] decorator that redirects `emit_stdout` to a file or
/// a pipe instead of forwarding it to the wrapped observer - `emit_stderr`
/// always forwards unchanged, matching bash (stderr is never redirected or
/// piped by this feature). Reused for both plain `>`/`>>` redirection and
/// `|` piping since, per the investigation behind this module, every task's
/// output - embedded or delegated - already funnels through `emit_stdout`.
///
/// `I` is the wrapped observer; also implements [`BndBuilderObserver`] when
/// `I` does, so a stage can be run through the ordinary [`crate::execute`]
/// unchanged, just wrapped in a fresh `RedirectedObserver`.
pub struct RedirectedObserver<I> {
    sink: Option<StdoutSink>,
    inner: I,
    /// `emit_stdout` returns `()` and can't fail loudly - a write error found
    /// there is recorded here instead, and folded into that stage's result
    /// by `execute_pipe` once the stage's own execution has returned.
    write_error: Mutex<Option<std::io::Error>>
}

impl<I> std::fmt::Debug for RedirectedObserver<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RedirectedObserver").finish_non_exhaustive()
    }
}

impl<I> RedirectedObserver<I> {
    fn new(sink: Option<StdoutSink>, inner: I) -> Self {
        Self {
            sink,
            inner,
            write_error: Mutex::new(None)
        }
    }

    /// Takes (leaving `None` behind) whichever non-broken-pipe write error
    /// was recorded while this observer was in use, if any.
    fn take_write_error(&self) -> Option<std::io::Error> {
        self.write_error.lock().unwrap().take()
    }
}

impl<I: EventObserver> EventObserver for RedirectedObserver<I> {
    fn emit_stdout(&self, s: &str) {
        match &self.sink {
            None => self.inner.emit_stdout(s),
            Some(StdoutSink::File(f)) => {
                if let Err(e) = f.lock().unwrap().write_all(s.as_bytes()) {
                    *self.write_error.lock().unwrap() = Some(e);
                }
            },
            Some(StdoutSink::Pipe(w)) => {
                if let Err(e) = w.lock().unwrap().write_all(s.as_bytes())
                    && e.kind() != std::io::ErrorKind::BrokenPipe
                {
                    // BrokenPipe means the downstream stage finished (or
                    // failed) early and stopped reading - not our error to
                    // report.
                    *self.write_error.lock().unwrap() = Some(e);
                }
            }
        }
    }

    fn emit_stderr(&self, s: &str) {
        self.inner.emit_stderr(s)
    }
}

impl<I: BndBuilderObserver> BndBuilderObserver for RedirectedObserver<I> {
    fn update(&self, event: BndBuilderEvent) {
        self.inner.update(event)
    }

    fn emit_ignored_error(&self, err: &str) {
        self.inner.emit_ignored_error(err)
    }
}

/// Runs a `|`-pipeline (or a single redirected task - a degenerate one-stage
/// pipeline), built by [`super::task::InnerTask`]'s deserializer out of
/// `split_top_level_pipes`/`strip_redirections`.
///
/// All stages run concurrently (`std::thread::scope`), not sequentially:
/// a producer emitting more than the OS pipe buffer (64KiB on Linux) before
/// its consumer starts reading would deadlock otherwise. Pipefail-style
/// semantics apply - any stage failing fails the whole pipeline - which is a
/// deliberate deviation from bash's default (only the last stage's exit
/// status matters): in a build tool, an earlier stage failing silently while
/// a later stage succeeds is far more likely to be a masked bug than
/// intended behaviour.
///
/// Deliberately **not** generic over the observer type: it and
/// [`RedirectedObserver`]'s `emit_stdout`/`emit_stderr` are the recursion
/// base case that keeps `InnerTask::Pipe` dispatch (see `executor.rs`) from
/// growing an unbounded `RedirectedObserver<Arc<RedirectedObserver<...>>>`
/// type chain that the compiler would have to monomorphize forever - see
/// the erasure comment at that call site.
pub fn execute_pipe(
    p: &PipeTaskArguments,
    observer: Arc<dyn BndBuilderObserver + Send + Sync>
) -> Result<(), String> {
    let n = p.stages.len();
    assert!(n >= 1, "a Pipe always has at least one stage");

    // readers[i] / writers[i]: the pipe feeding stage i's stdin / draining
    // stage i's stdout, for the inter-stage boundaries only (stage 0 has no
    // reader here - it uses `p.stdin` instead; the last stage has no writer
    // here - it uses `p.stdout` instead).
    let mut readers: Vec<Option<std::io::PipeReader>> = Vec::with_capacity(n);
    let mut writers: Vec<Option<std::io::PipeWriter>> = Vec::with_capacity(n);
    readers.push(None);
    for _ in 0..n - 1 {
        let (r, w) = std::io::pipe().map_err(|e| format!("failed to create a pipe: {e}"))?;
        readers.push(Some(r));
        writers.push(Some(w));
    }
    writers.push(None);

    // Build each stage's stdin source, stdout destination, and
    // output-redirecting observer up front (fallible - opening a `>`/`>>`
    // target can fail), before spawning anything.
    let mut stage_stdins: Vec<Option<TaskStdin>> = Vec::with_capacity(n);
    let mut stage_stdouts: Vec<Option<TaskStdout>> = Vec::with_capacity(n);
    type ErasedObserver = Arc<dyn BndBuilderObserver + Send + Sync>;
    let mut stage_observers: Vec<Option<Arc<RedirectedObserver<ErasedObserver>>>> =
        Vec::with_capacity(n);
    for i in 0..n {
        // Always `Some`, even for a first stage with no `<file`: a delegated
        // runner still needs the signal to force its non-PTY code path here
        // (a PTY would merge stdout and stderr into one stream, defeating
        // `>`/`|`), even though there is nothing real to read -
        // `TaskStdin::Empty` carries exactly that "no real input" distinction
        // through to runners (like `Echo`/`Rm`) that only special-case
        // genuine input.
        let stdin = if i == 0 {
            Some(p.stdin.clone().map(TaskStdin::File).unwrap_or(TaskStdin::Empty))
        }
        else {
            Some(TaskStdin::Reader(readers[i].take().unwrap()))
        };
        stage_stdins.push(stdin);

        let is_last = i == n - 1;

        // A Delegated stage (rasm, extern, gource, ffmpeg, ...) spawns a
        // real OS process whose stdout can be arbitrary binary data (not
        // necessarily text) - wire its destination directly as a `TaskStdout`
        // so `ExternRunner` can hand it to the child as a raw `Stdio`,
        // bypassing the EventObserver/String machinery entirely. Without
        // this, a byte stream like `gource ... -o - | ffmpeg ...` (raw PPM
        // frames) gets silently mangled: `emit_stdout` decodes it as UTF-8
        // text one character at a time, dropping every invalid sequence.
        // An embedded task (echo, rm, ...) has no real OS stdout to wire
        // this way and always emits text it constructed itself, so it
        // keeps going through `RedirectedObserver` as before - correct
        // there since there is no binary data to lose.
        if p.stages[i].kind() == TaskKind::Delegated {
            let stdout = if is_last {
                match &p.stdout {
                    None => None,
                    Some((path, append)) => Some(TaskStdout::File(path.clone(), *append))
                }
            }
            else {
                Some(TaskStdout::Writer(writers[i].take().unwrap()))
            };
            stage_stdouts.push(stdout);
            stage_observers.push(Some(Arc::new(RedirectedObserver::new(None, observer.clone()))));
        }
        else {
            stage_stdouts.push(None);
            let sink = if is_last {
                match &p.stdout {
                    None => None,
                    Some((path, append)) => {
                        let file = if *append {
                            fs_err::OpenOptions::new().create(true).append(true).open(path)
                        }
                        else {
                            fs_err::File::create(path)
                        }
                        .map_err(|e| format!("unable to open {path} for writing: {e}"))?;
                        Some(StdoutSink::File(Mutex::new(file)))
                    }
                }
            }
            else {
                Some(StdoutSink::Pipe(Mutex::new(writers[i].take().unwrap())))
            };
            stage_observers.push(Some(Arc::new(RedirectedObserver::new(sink, observer.clone()))));
        }
    }

    // Spawn every stage concurrently. Each thread OWNS (moves) its
    // `RedirectedObserver`, so it drops - closing its pipe writer, if any,
    // and signalling EOF downstream - as soon as that stage's own execution
    // returns, not after every stage has joined.
    let results: Vec<Result<(), String>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..n)
            .map(|i| {
                let stage = &p.stages[i];
                let stage_observer = stage_observers[i].take().unwrap();
                let stdin = stage_stdins[i].take();
                let stdout = stage_stdouts[i].take();
                scope.spawn(move || {
                    let result = crate::executor::execute_redirected(
                        stage,
                        &stage_observer,
                        stdin,
                        stdout
                    );
                    let write_error = stage_observer.take_write_error();
                    match (result, write_error) {
                        (Ok(()), None) => Ok(()),
                        (Ok(()), Some(e)) => {
                            Err(format!("write error in pipeline stage {i}: {e}"))
                        },
                        (Err(e), _) => Err(e)
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or_else(|_| Err("a pipeline stage's thread panicked".to_string()))
            })
            .collect()
    });

    results.into_iter().find(|r| r.is_err()).unwrap_or(Ok(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_with_no_pipe_is_a_single_segment_borrowing_the_input() {
        let segments = split_top_level_pipes("basm main.asm -o main.bin");
        assert_eq!(segments, vec!["basm main.asm -o main.bin"]);
    }

    #[test]
    fn pipes_split_into_segments() {
        let segments = split_top_level_pipes("extern sort input.txt | extern uniq");
        assert_eq!(segments, vec!["extern sort input.txt ", " extern uniq"]);
    }

    #[test]
    fn a_pipe_inside_quotes_is_not_a_split_point() {
        let segments = split_top_level_pipes("echo 'x | y'");
        assert_eq!(segments, vec!["echo 'x | y'"]);
        let segments = split_top_level_pipes("echo \"x | y\"");
        assert_eq!(segments, vec!["echo \"x | y\""]);
    }

    #[test]
    fn plain_redirection_is_stripped_and_reported() {
        let (remaining, stdin, stdout) = strip_redirections("echo hello > out.txt").unwrap();
        assert_eq!(remaining, "echo hello ");
        assert_eq!(stdin, None);
        assert_eq!(stdout, Some(RedirSpec::StdoutTruncate("out.txt".into())));
    }

    #[test]
    fn append_redirection_is_recognised() {
        let (remaining, _, stdout) = strip_redirections("echo hello >> out.txt").unwrap();
        assert_eq!(remaining, "echo hello ");
        assert_eq!(stdout, Some(RedirSpec::StdoutAppend("out.txt".into())));
    }

    #[test]
    fn stdin_redirection_is_recognised() {
        let (remaining, stdin, _) = strip_redirections("rm < filelist.txt").unwrap();
        assert_eq!(remaining, "rm ");
        assert_eq!(stdin, Some("filelist.txt".into()));
    }

    #[test]
    fn a_redirection_target_with_a_space_can_be_quoted() {
        let (_, _, stdout) = strip_redirections("echo hi > \"MODULE A/out.txt\"").unwrap();
        assert_eq!(
            stdout,
            Some(RedirSpec::StdoutTruncate("MODULE A/out.txt".into()))
        );
    }

    /// The one bndbuild-specific wrinkle: `$<`/`$@` (automatic variables,
    /// substituted later) must never be mistaken for redirection.
    #[test]
    fn dollar_lt_is_not_redirection() {
        let (remaining, stdin, stdout) = strip_redirections("basm $< -o $@").unwrap();
        assert_eq!(remaining, "basm $< -o $@");
        assert_eq!(stdin, None);
        assert_eq!(stdout, None);
    }

    #[test]
    fn a_redirection_character_inside_quotes_is_not_redirection() {
        let (remaining, stdin, stdout) = strip_redirections("echo \"a > b\"").unwrap();
        assert_eq!(remaining, "echo \"a > b\"");
        assert_eq!(stdin, None);
        assert_eq!(stdout, None);

        let (remaining, ..) = strip_redirections("echo 'a > b'").unwrap();
        assert_eq!(remaining, "echo 'a > b'");
    }

    #[test]
    fn a_second_stdout_redirection_is_a_hard_error() {
        assert!(strip_redirections("echo hi > a.txt > b.txt").is_err());
    }

    #[test]
    fn a_second_stdin_redirection_is_a_hard_error() {
        assert!(strip_redirections("rm < a.txt < b.txt").is_err());
    }

    #[test]
    fn an_unterminated_quote_in_a_target_is_an_error() {
        assert!(strip_redirections("echo hi > \"unterminated").is_err());
    }

    #[test]
    fn a_dangling_operator_with_no_target_is_an_error() {
        assert!(strip_redirections("echo hi >").is_err());
    }
}
