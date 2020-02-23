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
struct XferInteractorHelper<'a> {
    commands: [&'a str;6],
    completer: FilenameCompleter,
    hinter: HistoryHinter,
    xfer: &'a CpcXfer // TODO find a way to not share xfer there in order to not lost time to do too much calls to M4
}


impl<'a> Completer for XferInteractorHelper<'a> {
    type Candidate = Pair;

    /// TODO add M4 completion
    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let local = self.completer.complete(line, pos, ctx)?;
        let commands = self.complete_command_name(line, pos, ctx)?;
        let m4 = self.complete_m4_path_name(line, pos, ctx)?;

        assert_eq!( local.0, commands.0);
        assert_eq!( local.0, m4.0);

        let mut complete = Vec::with_capacity(local.1.len() + commands.1.len() + m4.1.len());

        // Retreive the command in order to filter the autocompletion
        let command = line.trim().split(' ').next().map(str::to_lowercase);
        let command = command.as_ref().map(String::as_str);
        // and get its position
        let start = {
            let mut idx = 0;
            for c in line.chars() {
                if c == ' ' || c == '\t' {
                    idx +=1;
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
        match command {
            Some("launch") => {complete.extend(local.1)},
            _ => {}
        }
        
        // Ensure M4 completion is not used for launch
        match command {
            Some("launch") => {},
            _ => {complete.extend(m4.1)}
        }

        Ok((local.0, complete))
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


impl<'a> XferInteractorHelper<'a> {


    /// Make the completion on the M4 filename
    fn complete_m4_path_name(&self, line: &str, pos:usize, ctx: &Context<'_> ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let mut entries: Vec<Pair> = Vec::new();

        let (start, word) = extract_word(line, pos, ESCAPE_CHAR, &DEFAULT_BREAK_CHARS);
        for file in self.xfer.current_folder_content().unwrap().files() {
            let fname1 = file.fname();
            let fname2 = ("./".to_owned() + &fname1);

            for &fname in [fname1, &fname2].iter() {

                if fname.starts_with(word) {
                    entries.push(Pair{
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
    fn complete_command_name(&self, line: &str, pos: usize, ctx: &Context<'_> ) -> Result<(usize, Vec<Pair>), ReadlineError> {


        let mut entries: Vec<Pair> = Vec::new();
        
        let (start, word) = extract_word(line, pos, ESCAPE_CHAR, &DEFAULT_BREAK_CHARS);
        // TODO check if it is the very first word
        for command in self.commands.iter() {
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

impl<'a> Hinter for XferInteractorHelper<'a> {
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
            hinter: HistoryHinter {},
            xfer: self.xfer,
            commands: [
                "cd",
                "launch",
                "ls",
                "pwd",
                "reset",
                "reboot"
            ]
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