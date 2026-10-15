//! `asm`: one command line for every assembler bndbuild knows, so a project
//! can switch assembler without rewriting its rules - the counterpart, for
//! assemblers, of what `cpc`/`emu` is for emulators.
//!
//! The common options are translated into the chosen assembler's own; an
//! option it has no equivalent for is refused rather than silently dropped.
//! Anything after `--` is handed to the assembler untouched.

use std::marker::PhantomData;
use std::sync::Arc;

use clap::{CommandFactory, Parser, ValueEnum};
use cpclib_asm::EnvEventObserver;
use cpclib_common::camino::Utf8PathBuf;
use cpclib_common::itertools::Itertools;
use cpclib_runner::delegated::DelegatedRunner;
use cpclib_runner::runner::assembler::ExternAssembler;
use cpclib_runner::runner::{Runner, RunnerWithClap, TaskStdin, TaskStdout};

use super::assembler::BasmRunner;
use crate::task::ASM_CMDS;

/// The assemblers `asm` can drive.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AsmChoice {
    #[default]
    Basm,
    Fantams,
    Rasm,
    Sjasmplus,
    Uz80,
    Vasm
}

impl AsmChoice {
    fn name(&self) -> &'static str {
        match self {
            Self::Basm => "basm",
            Self::Fantams => "fantams",
            Self::Rasm => "rasm",
            Self::Sjasmplus => "sjasmplus",
            Self::Uz80 => "uz80",
            Self::Vasm => "vasm"
        }
    }

    fn extern_assembler(&self) -> Option<ExternAssembler> {
        match self {
            Self::Basm => None,
            Self::Fantams => Some(ExternAssembler::Fantams(Default::default())),
            Self::Rasm => Some(ExternAssembler::Rasm(Default::default())),
            Self::Sjasmplus => Some(ExternAssembler::Sjasmplus(Default::default())),
            Self::Uz80 => Some(ExternAssembler::Uz80(Default::default())),
            Self::Vasm => Some(ExternAssembler::Vasm(Default::default()))
        }
    }
}

/// Assemble a file with any of the supported assemblers, using the same
/// options whichever is chosen.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "asm",
    no_binary_name = true,
    disable_version_flag = true,
    after_help = "Options an assembler has no equivalent for are refused. \
                  Arguments after -- are given to the assembler as they are."
)]
pub struct AsmCli {
    /// The assembler to use
    #[arg(short, long, value_enum, default_value_t = AsmChoice::Basm)]
    pub assembler: AsmChoice,

    /// The file to assemble
    pub input: Utf8PathBuf,

    /// The file to produce
    #[arg(short, long)]
    pub output: Option<Utf8PathBuf>,

    /// A directory where included files are searched (repeatable)
    #[arg(short = 'I', long = "include", value_name = "DIR")]
    pub includes: Vec<Utf8PathBuf>,

    /// Define a symbol, 1 when no value is given (repeatable)
    #[arg(short = 'D', long = "define", value_name = "NAME[=VALUE]")]
    pub defines: Vec<String>,

    /// Write the symbol table to this file
    #[arg(long, value_name = "FILE")]
    pub sym: Option<Utf8PathBuf>,

    /// Write the listing to this file
    #[arg(long, value_name = "FILE")]
    pub lst: Option<Utf8PathBuf>,

    /// Produce a snapshot instead of a raw binary
    #[arg(long, conflicts_with = "amsdos")]
    pub snapshot: bool,

    /// Prefix the binary with an AMSDOS header
    #[arg(long)]
    pub amsdos: bool,

    /// Arguments given to the assembler as they are (after --)
    #[arg(last = true)]
    pub extra: Vec<String>
}

impl AsmCli {
    /// `-D NAME` and `-D NAME=VALUE` as `(NAME, VALUE)`, VALUE being 1 by default.
    fn definitions(&self) -> Vec<(&str, &str)> {
        self.defines
            .iter()
            .map(|d| d.split_once('=').unwrap_or((d.as_str(), "1")))
            .collect()
    }

    fn unsupported(&self, what: &str) -> String {
        format!(
            "{} has no equivalent for {what}; pass its own option after -- instead",
            self.assembler.name()
        )
    }

    /// The command line of the chosen assembler.
    pub fn assembler_arguments(&self) -> Result<Vec<String>, String> {
        let mut args: Vec<String> = Vec::new();
        let input = self.input.to_string();

        match self.assembler {
            AsmChoice::Basm => {
                args.push(input);
                if let Some(output) = &self.output {
                    args.extend(["-o".to_owned(), output.to_string()]);
                }
                for dir in &self.includes {
                    args.extend(["-I".to_owned(), dir.to_string()]);
                }
                for (name, value) in self.definitions() {
                    args.extend(["-D".to_owned(), format!("{name}={value}")]);
                }
                if let Some(sym) = &self.sym {
                    args.extend(["--sym".to_owned(), sym.to_string()]);
                }
                if let Some(lst) = &self.lst {
                    args.extend(["--lst".to_owned(), lst.to_string()]);
                }
                if self.snapshot {
                    args.push("--snapshot".to_owned());
                }
                if self.amsdos {
                    args.push("--binary".to_owned());
                }
            },

            // rasm <inputfile> [options]
            AsmChoice::Rasm => {
                if self.lst.is_some() {
                    return Err(self.unsupported("--lst"));
                }
                if self.snapshot {
                    return Err(self
                        .unsupported("--snapshot (a snapshot comes from BUILDSNA in the source)"));
                }
                if self.amsdos {
                    return Err(self.unsupported(
                        "--amsdos (the header comes from SAVE ...,AMSDOS in the source)"
                    ));
                }
                args.push(input);
                if let Some(output) = &self.output {
                    args.extend(["-ob".to_owned(), output.to_string()]);
                }
                for dir in &self.includes {
                    args.push(format!("-I{dir}"));
                }
                for (name, value) in self.definitions() {
                    args.push(format!("-D{name}={value}"));
                }
                if let Some(sym) = &self.sym {
                    args.extend(["-s".to_owned(), "-os".to_owned(), sym.to_string()]);
                }
            },

            AsmChoice::Sjasmplus => {
                if self.snapshot {
                    return Err(self
                        .unsupported("--snapshot (a snapshot comes from SAVESNA in the source)"));
                }
                if self.amsdos {
                    return Err(self
                        .unsupported("--amsdos (the header comes from SAVEAMSDOS in the source)"));
                }
                if let Some(output) = &self.output {
                    args.push(format!("--raw={output}"));
                }
                for dir in &self.includes {
                    args.push(format!("--inc={dir}"));
                }
                for (name, value) in self.definitions() {
                    args.push(format!("-D{name}={value}"));
                }
                if let Some(sym) = &self.sym {
                    args.push(format!("--sym={sym}"));
                }
                if let Some(lst) = &self.lst {
                    args.push(format!("--lst={lst}"));
                }
                args.push(input);
            },

            // vasmz80_oldstyle: its default output module is not a binary
            AsmChoice::Vasm => {
                if self.sym.is_some() {
                    return Err(self.unsupported("--sym"));
                }
                if self.snapshot {
                    return Err(self.unsupported("--snapshot"));
                }
                if self.amsdos {
                    return Err(self.unsupported("--amsdos"));
                }
                args.push("-Fbin".to_owned());
                if let Some(output) = &self.output {
                    args.extend(["-o".to_owned(), output.to_string()]);
                }
                for dir in &self.includes {
                    args.push(format!("-I{dir}"));
                }
                for (name, value) in self.definitions() {
                    args.push(format!("-D{name}={value}"));
                }
                if let Some(lst) = &self.lst {
                    args.extend(["-L".to_owned(), lst.to_string()]);
                }
                args.push(input);
            },

            // Only the options this project already relies on are used.
            AsmChoice::Uz80 => {
                if !self.includes.is_empty() {
                    return Err(self.unsupported("-I"));
                }
                if !self.defines.is_empty() {
                    return Err(self.unsupported("-D"));
                }
                if self.sym.is_some() {
                    return Err(self.unsupported("--sym"));
                }
                if self.lst.is_some() {
                    return Err(self.unsupported("--lst"));
                }
                if self.snapshot {
                    return Err(self.unsupported("--snapshot"));
                }
                if self.amsdos {
                    return Err(self.unsupported("--amsdos"));
                }
                args.push(input);
                if let Some(output) = &self.output {
                    args.push(format!("-o{output}"));
                }
            },

            // The extension of -o selects what fantams writes.
            AsmChoice::Fantams => {
                if !self.includes.is_empty() {
                    return Err(self.unsupported("-I"));
                }
                if !self.defines.is_empty() {
                    return Err(self.unsupported("-D"));
                }
                if self.lst.is_some() {
                    return Err(self.unsupported("--lst"));
                }
                if self.amsdos {
                    return Err(self.unsupported("--amsdos"));
                }
                let output_is_sna = self
                    .output
                    .as_ref()
                    .and_then(|o| o.extension())
                    .is_some_and(|e| e.eq_ignore_ascii_case("sna"));
                if self.snapshot && !output_is_sna {
                    return Err(
                        "fantams writes a snapshot when the output ends with .sna: use -o file.sna"
                            .to_owned()
                    );
                }
                args.push(input);
                if let Some(output) = &self.output {
                    args.extend(["-o".to_owned(), output.to_string()]);
                }
                if let Some(sym) = &self.sym {
                    args.push(format!("--sym={sym}"));
                }
            }
        }

        args.extend(self.extra.iter().cloned());
        Ok(args)
    }
}

pub struct AsmFacadeRunner<E: EnvEventObserver> {
    command: clap::Command,
    _phantom: PhantomData<E>
}

impl<E: EnvEventObserver> Default for AsmFacadeRunner<E> {
    fn default() -> Self {
        Self {
            command: AsmCli::command(),
            _phantom: Default::default()
        }
    }
}

impl<E: EnvEventObserver + 'static> AsmFacadeRunner<E> {
    fn assemble<S: AsRef<str>>(
        &self,
        itr: &[S],
        o: &Arc<E>,
        stdin: Option<TaskStdin>,
        stdout: Option<TaskStdout>
    ) -> Result<(), String> {
        let itr = itr.iter().map(|s| s.as_ref()).collect_vec();
        let cli = match AsmCli::try_parse_from(&itr) {
            Ok(cli) => cli,
            Err(e) if e.kind() == clap::error::ErrorKind::DisplayHelp => {
                o.emit_stdout(&e.render().ansi().to_string());
                return Ok(());
            },
            Err(e) => {
                o.emit_stderr(&e.to_string());
                return Err("Argument parsing failed".to_owned());
            }
        };

        let args = cli.assembler_arguments()?;
        match cli.assembler.extern_assembler() {
            None => BasmRunner::<E>::default().inner_run_redirected(&args, o, stdin, stdout),
            Some(assembler) => {
                DelegatedRunner::<E>::new(
                    assembler.configuration(),
                    assembler.get_command().to_owned()
                )
                .inner_run_redirected(&args, o, stdin, stdout)
            },
        }
    }
}

impl<E: EnvEventObserver + 'static> RunnerWithClap for AsmFacadeRunner<E> {
    fn get_clap_command(&self) -> &clap::Command {
        &self.command
    }
}

impl<E: EnvEventObserver + 'static> Runner for AsmFacadeRunner<E> {
    type EventObserver = Arc<E>;

    fn inner_run<S: AsRef<str>>(&self, itr: &[S], o: &Self::EventObserver) -> Result<(), String> {
        self.assemble(itr, o, None, None)
    }

    fn inner_run_redirected<S: AsRef<str>>(
        &self,
        itr: &[S],
        o: &Self::EventObserver,
        stdin: Option<TaskStdin>,
        stdout: Option<TaskStdout>
    ) -> Result<(), String> {
        self.assemble(itr, o, stdin, stdout)
    }

    fn get_command(&self) -> &str {
        ASM_CMDS[0]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn translate(line: &str) -> Result<Vec<String>, String> {
        let cli = AsmCli::try_parse_from(line.split_whitespace()).map_err(|e| e.to_string())?;
        cli.assembler_arguments()
    }

    const COMMON: &str = "main.asm -o out.bin -I inc -D DEBUG -D SPEED=3";

    #[test]
    fn basm_is_the_default() {
        assert_eq!(
            translate(&format!("{COMMON} --sym out.sym --lst out.lst --amsdos")).unwrap(),
            [
                "main.asm", "-o", "out.bin", "-I", "inc", "-D", "DEBUG=1", "-D", "SPEED=3",
                "--sym", "out.sym", "--lst", "out.lst", "--binary"
            ]
        );
    }

    #[test]
    fn rasm() {
        assert_eq!(
            translate(&format!("-a rasm {COMMON} --sym out.sym")).unwrap(),
            [
                "main.asm",
                "-ob",
                "out.bin",
                "-Iinc",
                "-DDEBUG=1",
                "-DSPEED=3",
                "-s",
                "-os",
                "out.sym"
            ]
        );
        assert!(translate("-a rasm main.asm --lst out.lst").is_err());
    }

    #[test]
    fn sjasmplus() {
        assert_eq!(
            translate(&format!(
                "-a sjasmplus {COMMON} --sym out.sym --lst out.lst"
            ))
            .unwrap(),
            [
                "--raw=out.bin",
                "--inc=inc",
                "-DDEBUG=1",
                "-DSPEED=3",
                "--sym=out.sym",
                "--lst=out.lst",
                "main.asm"
            ]
        );
    }

    #[test]
    fn vasm() {
        assert_eq!(
            translate(&format!("-a vasm {COMMON} --lst out.lst")).unwrap(),
            [
                "-Fbin",
                "-o",
                "out.bin",
                "-Iinc",
                "-DDEBUG=1",
                "-DSPEED=3",
                "-L",
                "out.lst",
                "main.asm"
            ]
        );
        assert!(translate("-a vasm main.asm --sym out.sym").is_err());
    }

    #[test]
    fn uz80_only_takes_the_output() {
        assert_eq!(
            translate("-a uz80 main.asm -o out.bin").unwrap(),
            ["main.asm", "-oout.bin"]
        );
        assert!(translate("-a uz80 main.asm -I inc").is_err());
    }

    #[test]
    fn fantams() {
        assert_eq!(
            translate("-a fantams main.asm -o out.sna --sym out.sym --snapshot").unwrap(),
            ["main.asm", "-o", "out.sna", "--sym=out.sym"]
        );
        assert!(translate("-a fantams main.asm -o out.bin --snapshot").is_err());
        assert!(translate("-a fantams main.asm -D X").is_err());
    }

    #[test]
    fn the_arguments_after_double_dash_are_kept() {
        assert_eq!(
            translate("-a rasm main.asm -- -amper -dams").unwrap(),
            ["main.asm", "-amper", "-dams"]
        );
    }

    #[test]
    fn help_is_rendered() {
        let help = AsmCli::command().render_long_help().to_string();
        assert!(help.contains("--assembler"));
        assert!(help.contains("fantams"));
    }
}
