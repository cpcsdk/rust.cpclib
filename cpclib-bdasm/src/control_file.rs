use std::fmt;
use std::fs::File;
use std::io::Write;

use cpclib_common::camino::Utf8PathBuf;

use crate::{DataBlocString, Result};

/// Control file directive types
#[derive(Debug)]
pub enum ControlDirective {
    Origin(u16),
    Skip(usize),
    Length(u16),
    DataBloc(DataBlocString),
    Label { name: String, address: u16 },
    CpcString(DataBlocString)
}

impl fmt::Display for ControlDirective {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ControlDirective::Origin(addr) => {
                write!(f, "origin 0x{:04x}", addr)
            },
            ControlDirective::Skip(count) => {
                write!(f, "skip {}", count)
            },
            ControlDirective::Length(len) => {
                write!(f, "length 0x{:04x}", len)
            },
            ControlDirective::DataBloc(spec) => {
                write!(f, "data {}", spec)
            },
            ControlDirective::Label { name, address } => {
                write!(f, "label {}=0x{:04x}", name, address)
            },
            ControlDirective::CpcString(spec) => {
                write!(f, "cpcstring {}", spec)
            }
        }
    }
}

/// Control file for saving/loading disassembly configuration
#[derive(Debug, Default)]
pub struct ControlFile {
    pub directives: Vec<ControlDirective>
}

impl fmt::Display for ControlFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "; Control file for bdasm disassembler")?;
        writeln!(f, "; Format: <directive> <parameters>")?;
        writeln!(
            f,
            "; Directives: origin, skip, length, data, label, cpcstring"
        )?;
        writeln!(f)?;

        for directive in &self.directives {
            writeln!(f, "{}", directive)?;
        }

        Ok(())
    }
}

impl ControlFile {
    /// Merge CLI arguments into the control file
    /// CLI arguments take precedence over control file directives
    pub fn merge_cli(&mut self, cli: &crate::BdAsmCli) {
        // Add origin if provided in CLI (overrides control file)
        if let Some(origin) = cli.origin {
            // Remove existing origin directives
            self.directives
                .retain(|d| !matches!(d, ControlDirective::Origin(_)));
            self.directives.insert(0, ControlDirective::Origin(origin));
        }

        // Add skip if provided in CLI (overrides control file)
        if let Some(skip) = cli.skip {
            // Remove existing skip directives
            self.directives
                .retain(|d| !matches!(d, ControlDirective::Skip(_)));
            self.directives
                .insert(0, ControlDirective::Skip(skip as usize));
        }

        // Add length if provided in CLI (overrides control file)
        if let Some(length) = cli.length {
            // Remove existing length directives
            self.directives
                .retain(|d| !matches!(d, ControlDirective::Length(_)));
            self.directives.insert(0, ControlDirective::Length(length));
        }

        // Add labels from CLI
        for label in &cli.label {
            let split = label.split('=').collect::<Vec<_>>();
            if split.len() == 2 {
                let label_name = split[0].to_string();
                if let Ok(address) = crate::parser::parse_u16_value(split[1]) {
                    self.directives.push(ControlDirective::Label {
                        name: label_name,
                        address
                    });
                }
            }
        }

        // Add data blocs from CLI
        for spec in &cli.data_bloc {
            self.directives
                .push(ControlDirective::DataBloc(spec.clone()));
        }
    }

    /// Get the skip value from the control file
    pub fn get_skip(&self) -> usize {
        self.directives
            .iter()
            .find_map(|d| {
                if let ControlDirective::Skip(s) = d {
                    Some(*s)
                }
                else {
                    None
                }
            })
            .unwrap_or(0)
    }

    /// Get the length value from the control file
    pub fn get_length(&self) -> Option<u16> {
        self.directives.iter().find_map(|d| {
            if let ControlDirective::Length(len) = d {
                Some(*len)
            }
            else {
                None
            }
        })
    }

    /// Get the origin value from the control file
    pub fn get_origin(&self) -> Option<u16> {
        self.directives.iter().find_map(|d| {
            if let ControlDirective::Origin(addr) = d {
                Some(*addr)
            }
            else {
                None
            }
        })
    }
}

/// Save control file to disk
pub fn save_control_file(path: &Utf8PathBuf, control: &ControlFile) -> Result<()> {
    let mut file = File::create(path)?;
    write!(file, "{}", control)?;
    Ok(())
}
