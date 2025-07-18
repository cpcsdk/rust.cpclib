use cpclib_common::camino::Utf8Path;
use cpclib_common::winnow::Parser;
use cpclib_xfer::CpcXfer;
use rustyline::completion::{Completer, FilenameCompleter, Pair, extract_word};
use rustyline::error::ReadlineError;
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::{CompletionType, Config, Context, EditMode, Editor};
use rustyline_derive::{Helper, Highlighter, Validator};
use subprocess::Exec;
use term_grid::{Direction, Filling, Grid, GridOptions};
use {rustyline, termize};

use crate::parser::{XferCommand, parse_command};

/// Help to add autocompletion.
/// Done currently with filname, will be done later with M4 file names
#[derive(Helper, Validator, Highlighter)]
struct XferInteractorHelper<'a> {
    commands: Vec<&'static str>,
    completer: FilenameCompleter,
    hinter: HistoryHinter,
    xfer: &'a CpcXfer /* TODO find a way to not share xfer there in order to not lost time to do too much calls to M4 */
}

impl Completer for XferInteractorHelper<'_> {
    type Candidate = Pair;

    /// TODO add M4 completion
    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let local = self.completer.complete(line, pos, ctx)?;
        let commands = self.complete_command_name(line, pos, ctx)?;
        let m4 = self.complete_m4_path_name(line, pos, ctx)?;

        assert_eq!(local.0, commands.0);
        assert_eq!(local.0, m4.0);

        let mut complete = Vec::with_capacity(local.1.len() + commands.1.len() + m4.1.len());

        // Retreive the command in order to filter the autocompletion
        let command = line.trim().split(' ').next().map(str::to_lowercase);
        let command = command.as_deref();
        // and get its position
        let start = {
            let mut idx = 0;
            for c in line.chars() {
                if c == ' ' || c == '\t' {
                    idx += 1;
                }
                else {
                    break;
                }
            }
            idx
        };

        // Ensure command completion is only done for the first word
        if start == local.0 {
            complete.extend(commands.1);
        }

        // Ensure local completion is only done for launch (at the moment)
        if let Some("launch" | "put") = command {
            complete.extend(local.1)
        }

        // Ensure M4 completion is not used for launch
        match command {
            Some("launch") => {},
            _ => complete.extend(m4.1)
        }

        Ok((local.0, complete))
    }
}

// stolen to rustyline ccode as it is not public
const DOUBLE_QUOTES_ESCAPE_CHAR: Option<char> = Some('\\');
cfg_if::cfg_if! {
    if #[cfg(unix)] {
        // rl_basic_word_break_characters, rl_completer_word_break_characters
        const DEFAULT_BREAK_CHARS: [char; 18] = [
            ' ', '\t', '\n', '"', '\\', '\'', '`', '@', '$', '>', '<', '=', ';', '|', '&',
            '{', '(', '\0',
        ];
        const ESCAPE_CHAR: Option<char> = Some('\\');
        // In double quotes, not all break_chars need to be escaped
        // https://www.gnu.org/software/bash/manual/html_node/Double-Quotes.html
        const DOUBLE_QUOTES_SPECIAL_CHARS: [u8; 4] = [b'"', b'$', b'\\', b'`'];
    } else if #[cfg(windows)] {
        // Remove \ to make file completion works on windows
        const DEFAULT_BREAK_CHARS: [char; 17] = [
            ' ', '\t', '\n', '"', '\'', '`', '@', '$', '>', '<', '=', ';', '|', '&', '{',
            '(', '\0',
        ];
        const ESCAPE_CHAR: Option<char> = None;
        const DOUBLE_QUOTES_SPECIAL_CHARS: [u8; 1] = [b'"']; // TODO Validate: only '"' ?
    } else if #[cfg(target_arch = "wasm32")] {
        const DEFAULT_BREAK_CHARS: [char; 0] = [];
        const ESCAPE_CHAR: Option<char> = None;
        const DOUBLE_QUOTES_SPECIAL_CHARS: [u8; 0] = [];
    }
}

impl<'a> XferInteractorHelper<'a> {
    pub fn new(xfer: &'a CpcXfer) -> Self {
        XferInteractorHelper {
            completer: FilenameCompleter::new(),
            hinter: HistoryHinter {},
            xfer,
            commands: vec![
                "rm", "del", "delete", "era", "cd", "exit", "launch", "ls", "put", "pwd", "reset",
                "reboot",
            ]
        }
    }

    /// Make the completion on the M4 filename
    fn complete_m4_path_name(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let mut entries: Vec<Pair> = Vec::new();

        let (start, word) =
            extract_word(line, pos, ESCAPE_CHAR, |c| DEFAULT_BREAK_CHARS.contains(&c));
        for file in self.xfer.current_folder_content().unwrap().files() {
            let fname1 = file.fname();
            let fname2 = "./".to_owned() + fname1;

            for &fname in &[fname1, &fname2] {
                if fname.starts_with(word) {
                    entries.push(Pair {
                        display: fname.into(),
                        replacement: fname.into()
                    });
                }
            }
        }

        Ok((start, entries))
    }

    /// Search the possible command names for completion
    /// TODO do it only when first word
    fn complete_command_name(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let mut entries: Vec<Pair> = Vec::new();

        let (start, word) =
            extract_word(line, pos, ESCAPE_CHAR, |c| DEFAULT_BREAK_CHARS.contains(&c));
        // TODO check if it is the very first word
        for command in &self.commands {
            if command.starts_with(word) {
                entries.push(Pair {
                    display: (*command).to_string(),
                    replacement: (*command).to_string()
                })
            }
        }

        Ok((start, entries))
    }
}

impl Hinter for XferInteractorHelper<'_> {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

/// The object htat manages the interaction.
/// There are some state to store over commands.
pub struct XferInteractor<'a> {
    /// Reference to the cpc xfer
    xfer: &'a CpcXfer,
    /// Current Working Directory
    cwd: String,
    exit: bool
}

impl<'a> XferInteractor<'a> {
    pub fn treat_line(&mut self, line: &str) {
        let parse = parse_command.parse(line);
        if let Ok(command) = parse {
            println!("{command:?}");

            match command {
                XferCommand::Help => {
                    println!(
                        "help       Displays the help.
cd <folder>         Goes to <folder> in the M4.
exit                Leaves the program.
rm <file>           Remove the file for the M4. Synonyms: era, del, delete. 
pwd                 Prints the current M4 directory.
reboot              Reboot.
reset               Reset.
<fname>             Launch <fname> from the M4.
launch <fname>      Launch <fname> from the host machine.
ls                  List the files in the current M4 directory.
!<command>          Launch <command> on the host machine.
                    "
                    )
                },

                XferCommand::Exit => {
                    self.exit = true;
                },

                XferCommand::Pwd => {
                    match self.xfer.current_working_directory() {
                        Ok(pwd) => {
                            println!("{pwd}");
                        },
                        Err(e) => {
                            eprintln!("{e}");
                        }
                    }
                },

                XferCommand::Ls(_path) => {
                    let content = self.xfer.current_folder_content();
                    if content.is_err() {
                        eprintln!("{}", content.err().unwrap());
                        return;
                    }

                    let mut grid = Grid::new(GridOptions {
                        filling: Filling::Spaces(3),
                        direction: Direction::LeftToRight
                    });
                    for file in content.unwrap().files() {
                        grid.add(file.fname().into());
                    }

                    let grid_width = if let Some((w, _h)) = termize::dimensions() {
                        w
                    }
                    else {
                        80
                    };

                    println!("{}", grid.fit_into_width(grid_width).unwrap());
                },

                XferCommand::Cd(path) => {
                    let path = match path {
                        None => "/",
                        Some(ref path) => path
                    };

                    let res = self.xfer.cd(path);
                    if res.is_err() {
                        eprintln!("{}", res.err().unwrap());
                    }
                    else {
                        self.cwd = self.xfer.current_working_directory().unwrap()
                    }
                },

                XferCommand::Era(path) => {
                    let res = self.xfer.rm(path);
                    if res.is_err() {
                        eprintln!("{}", res.err().unwrap());
                    }
                },

                XferCommand::Put(arg1) => {
                    let path = Utf8Path::new(&arg1);
                    if !path.exists() {
                        eprintln!("{arg1} does not exists");
                        return;
                    }

                    let destination = self.cwd.clone();

                    // Put the file
                    let res = self.xfer.upload(path, &destination, None);
                    if res.is_err() {
                        eprintln!("{}", res.err().unwrap());
                    }
                },

                XferCommand::LaunchHost(path) => {
                    if !std::path::Path::new(&path).exists() {
                        eprintln!("{path} not found.")
                    }
                    else {
                        let res = self.xfer.upload_and_run(path, None);
                        if res.is_err() {
                            eprintln!("{}", res.err().unwrap());
                        }
                    }
                },

                XferCommand::LaunchM4(path) => {
                    // Ensure the path is absolute (TODO check if this code is not also elswhere)
                    let path = if !path.starts_with('/') {
                        self.cwd.clone() + &path
                    }
                    else {
                        path
                    };

                    let res = self.xfer.run(&path);
                    if res.is_err() {
                        eprintln!("{}", res.err().unwrap());
                    }
                },

                XferCommand::LocalCommand(command) => {
                    Exec::shell(command).join(); // ignore failure
                },

                XferCommand::Reboot => {
                    self.xfer.reset_m4().unwrap();
                },

                XferCommand::Reset => {
                    self.xfer.reset_cpc().unwrap();
                },

                _ => unimplemented!()
            }
        }
    }

    /// Start the interactive session on the current xfer session
    pub fn start(xfer: &'a CpcXfer) {
        let cwd = xfer.current_working_directory().unwrap();
        let mut interactor = XferInteractor {
            xfer,
            cwd,
            exit: false
        };
        interactor.r#loop();
    }

    /// Manage the interactive loop
    fn r#loop(&mut self) {
        let history = "history.m4";

        let config = Config::builder()
            .history_ignore_space(true)
            .history_ignore_dups(true)
            .unwrap()
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            //.output_stream(OutputStreamType::Stdout)
            .build();
        let h = XferInteractorHelper::new(self.xfer);

        let mut rl = Editor::with_config(config).unwrap();
        rl.set_helper(Some(h));

        if rl.load_history(history).is_err() {
            println!("No previous history to load.");
        }

        while !self.exit {
            // Build the prompt value
            let prompt = format!("arnold@{}:{} $ ", self.xfer.hostname(), &self.cwd);

            // Treat the line
            let readline = rl.readline(&prompt);
            match readline {
                Ok(line) => {
                    self.treat_line(&line);
                    rl.add_history_entry(line);
                },
                Err(ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    break;
                },
                Err(ReadlineError::Eof) => break,
                Err(err) => {
                    println!("Error: {err:?}");
                    break;
                }
            }
        }
        rl.save_history(history).unwrap();
    }
}
