
use cpc::xfer::CpcXfer;
extern crate rustyline;

use rustyline::error::ReadlineError;
use rustyline::hint::Hinter;
use rustyline::{Cmd, CompletionType, Config, EditMode, Editor, Helper, KeyPress}
;
use rustyline::highlight::Highlighter;
use rustyline::completion::{Completer, FilenameCompleter, Pair, extract_word};
use std::borrow::Cow::{self, Borrowed, Owned};

use crate::parser::{parse_command, XferCommand};

pub fn treat_line(xfer: &CpcXfer, line: &str) {

    let parse= parse_command(line);
    if let Ok((_, command)) = parse  {
        println!("{:?}", command);

        match command {

           XferCommand::Pwd => {
               match xfer.current_working_directory() {
                   Ok(pwd) => {
                        println!("{}", pwd);
                   },
                   Err(e) => {
                       eprintln!("{}", e);
                   }
               }
           },


           XferCommand::Ls(path) => {
                let content = xfer.current_folder_content();
                if content.is_err() {
                       eprintln!("{}", content.err().unwrap());
                       return;
                }
                for file in content.unwrap().files() {
                    println!("{:?}", file);
                }

           },


           XferCommand::Cd(path) => {
               let path = match path {
                   None => "/",
                   Some(ref path) => &path
               };

               let res = xfer.cd(path);
               if res.is_err() {
                       eprintln!("{}", res.err().unwrap());
                       return;
                }

           },


           XferCommand::Reboot => {
               xfer.reset_m4();
           },

           XferCommand::Reset => {
               xfer.reset_cpc();
           },

           _ => unimplemented!()
        }
    }
}


pub fn start(xfer: CpcXfer) {
	let history = "history.m4";

	let mut rl = Editor::<()>::new();
	if rl.load_history(history).is_err() {
        println!("No previous history.");
    }

	loop {
		let prompt = format!("arnold@{} $ ", xfer.hostname());

		let readline = rl.readline(&prompt);
		 match readline {
		    Ok(line) => {
                rl.add_history_entry(line.as_ref());
                treat_line(&xfer, &line);
            },
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break
            },
            Err(ReadlineError::Eof) => {
                break
            },
            Err(err) => {
                println!("Error: {:?}", err);
                break
            }
        }
	}
	rl.save_history(history).unwrap();
}