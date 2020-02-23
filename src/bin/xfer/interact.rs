use cpclib::xfer::CpcXfer;
use rustyline;

use rustyline::error::ReadlineError;

use crate::parser::{parse_command, XferCommand};
use term_grid::{Grid, GridOptions, Direction, Filling};
use termize;

use rustyline::config::OutputStreamType;
use rustyline::completion::{Completer, FilenameCompleter, Pair, extract_word};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline_derive::{Helper, Validator, Highlighter, Hinter};
use rustyline::{Cmd, CompletionType, Config, Context, EditMode, Editor, KeyPress};

/// Help to add autocompletion.
/// Done currently with filname, will be done later with M4 file names
#[derive(Helper, Validator, Highlighter)]
struct XferInteractorHelper {
    completer: FilenameCompleter,
    hinter: HistoryHinter,
}


impl Completer for XferInteractorHelper {
    type Candidate = Pair;

    /// TODO add M4 completion
    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        //self.completer.complete(line, pos, ctx)
        self.complete_command_name(line, pos, ctx)
    }
}

// stolen to rustyline ccode as it is not public
const DOUBLE_QUOTES_ESCAPE_CHAR: Option<char> = Some('\\');
cfg_if::cfg_if! {
    if #[cfg(unix)] {
        // rl_basic_word_break_characters, rl_completer_word_break_characters
        const DEFAULT_BREAK_CHARS: [u8; 18] = [
            b' ', b'\t', b'\n', b'"', b'\\', b'\'', b'`', b'@', b'$', b'>', b'<', b'=', b';', b'|', b'&',
            b'{', b'(', b'\0',
        ];
        const ESCAPE_CHAR: Option<char> = Some('\\');
        // In double quotes, not all break_chars need to be escaped
        // https://www.gnu.org/software/bash/manual/html_node/Double-Quotes.html
        const DOUBLE_QUOTES_SPECIAL_CHARS: [u8; 4] = [b'"', b'$', b'\\', b'`'];
    } else if #[cfg(windows)] {
        // Remove \ to make file completion works on windows
        const DEFAULT_BREAK_CHARS: [u8; 17] = [
            b' ', b'\t', b'\n', b'"', b'\'', b'`', b'@', b'$', b'>', b'<', b'=', b';', b'|', b'&', b'{',
            b'(', b'\0',
        ];
        const ESCAPE_CHAR: Option<char> = None;
        const DOUBLE_QUOTES_SPECIAL_CHARS: [u8; 1] = [b'"']; // TODO Validate: only '"' ?
    } else if #[cfg(target_arch = "wasm32")] {
        const DEFAULT_BREAK_CHARS: [u8; 0] = [];
        const ESCAPE_CHAR: Option<char> = None;
        const DOUBLE_QUOTES_SPECIAL_CHARS: [u8; 0] = [];
    }
}


impl XferInteractorHelper {
    fn complete_command_name(&self, line: &str, pos: usize, ctx: &Context<'_> ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let commands = [
            "cd",
            "launch",
            "ls",
            "pwd",
            "reset",
            "reboot"
        ];

        let mut entries: Vec<Pair> = Vec::new();
        
        let (start, word) = extract_word(line, pos, ESCAPE_CHAR, &DEFAULT_BREAK_CHARS);
        // TODO check if it is the very first word
        for command in commands.iter() {
            if command.starts_with(word) {
                entries.push(Pair {
                    display: command.to_string(),
                    replacement: command.to_string(),
                })
            }
        }


        Ok((start, entries))
    }
}

impl Hinter for XferInteractorHelper {
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
    cwd: String
}

impl<'a> XferInteractor<'a> {

    pub fn treat_line(&mut self, line: &str) {
        let parse = parse_command(line);
        if let Ok((_, command)) = parse {
            println!("{:?}", command);

            match command {
                XferCommand::Help => {
                    println!("help       Displays the help.
cd <folder>         Goes to <folder> in the M4.
pwd                 Prints the current M4 directory.
reboot              Reboot.
reset               Reset.
<fname>             Launch <fname> from the M4.
launch <fname>      Launch <fname> from the host machine.
ls                  List the files in the current M4 directory.
                    ")

                },

                XferCommand::Pwd => match self.xfer.current_working_directory() {
                    Ok(pwd) => {
                        println!("{}", pwd);
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                    }
                },

                XferCommand::Ls(_path) => {
                    let content = self.xfer.current_folder_content();
                    if content.is_err() {
                        eprintln!("{}", content.err().unwrap());
                        return;
                    }

                    let mut grid = Grid::new(GridOptions {
                        filling:     Filling::Spaces(3),
                        direction:   Direction::LeftToRight,
                    });
                    for file in content.unwrap().files() {
                        grid.add(file.fname().into());
                    }

                    let grid_width = if let Some((w, h)) = termize::dimensions() {
                        w
                    }
                    else {
                        80
                    };

                    println!("{}", grid.fit_into_width(grid_width).unwrap());
                }

                XferCommand::Cd(path) => {
                    let path = match path {
                        None => "/",
                        Some(ref path) => &path,
                    };

                    let res = self.xfer.cd(path);
                    if res.is_err() {
                        eprintln!("{}", res.err().unwrap());
                        return;
                    }
                    else {
                        self.cwd = self.xfer.current_working_directory().unwrap()
                    }
                }

                XferCommand::LaunchHost(path) => {
                    if ! std::path::Path::new(&path).exists() {
                        eprintln!("{} not found.", path)
                    }
                    else {

                        let res = self.xfer.upload_and_run(path, None);
                        if res.is_err(){
                            eprintln!("{}", res.err().unwrap());
                            return;
                        }
                    }
                }

                XferCommand::LaunchM4(path) => {
                    /// Ensure the path is absolute (TODO check if this code is not also elswhere)
                    let path = if ! path.starts_with('/') {
                        self.cwd.clone() + &path 
                    }
                    else {
                        path
                    };

                    let res = self.xfer.run(&path);
                    if res.is_err(){
                        eprintln!("{}", res.err().unwrap());
                        return;
                    }
                }

                XferCommand::Reboot => {
                    self.xfer.reset_m4().unwrap();
                }

                XferCommand::Reset => {
                    self.xfer.reset_cpc().unwrap();
                }

                _ => unimplemented!(),
            }
        }
    }

    /// Start the interactive session on the current xfer session
    pub fn start(xfer: &'a CpcXfer) {


        let cwd = xfer.current_working_directory().unwrap();
        let mut interactor = XferInteractor {
            xfer: xfer,
            cwd
        };
        interactor.r#loop();

    }

    /// Manage the interactive loop
    fn r#loop(&mut self) {
        let history = "history.m4";

        let config = Config::builder()
            .history_ignore_space(true)
            .history_ignore_dups(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .output_stream(OutputStreamType::Stdout)
            .build();
        let h = XferInteractorHelper {
            completer: FilenameCompleter::new(),
            hinter: HistoryHinter {}
        };

        let mut rl = Editor::with_config(config);
        rl.set_helper(Some(h));

        if rl.load_history(history).is_err() {
            println!("No previous history to load.");
        }

        loop {
            // Build the prompt value
            let prompt = format!(
                "arnold@{}:{} $ ", 
                self.xfer.hostname(), 
                &self.cwd
            );

            // Treat the line
            let readline = rl.readline(&prompt);
            match readline {
                Ok(line) => {
                    self.treat_line(&line);
                    rl.add_history_entry(line);
                }
                Err(ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    break;
                }
                Err(ReadlineError::Eof) => break,
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }
        rl.save_history(history).unwrap();
    }
}