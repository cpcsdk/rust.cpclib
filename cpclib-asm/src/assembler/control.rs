
use cpclib_tokens::{Expr, ExprElement, FormattedExpr};

use super::{visit_assert, Env};
use crate::error::AssemblerError;
use crate::preamble::Z80Span;


/// This structure store the necessary information to replay the assembled stuff of previous passes when we do not reassemble
#[derive(Debug, Clone, PartialEq, Eq)]
struct ControlOutputByte {
    // The value to output
    value: u8
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ControlOutputBytes {
    // The value to output
    bytes: Vec<u8>
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ControlOrg {
    code: u16,
    output: u16
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ControlAssert {
    exp: Expr,
    txt: Option<Vec<FormattedExpr>>,
    span: Option<Z80Span>
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ControlOutputCommand {
    Assert(ControlAssert),
    Byte(ControlOutputByte),
    Bytes(ControlOutputBytes),
    Org(ControlOrg)
}

impl From<ControlAssert> for ControlOutputCommand {
    fn from(c: ControlAssert) -> Self {
        Self::Assert(c)
    }
}

impl From<ControlOutputByte> for ControlOutputCommand {
    fn from(b: ControlOutputByte) -> Self {
        Self::Byte(b)
    }
}

impl From<ControlOutputBytes> for ControlOutputCommand {
    fn from(b: ControlOutputBytes) -> Self {
        Self::Bytes(b)
    }
}

impl From<ControlOrg> for ControlOutputCommand {
    fn from(value: ControlOrg) -> Self {
        Self::Org(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlOutputStore {
    pub(crate) remaining_passes: u8,
    pub(crate) commands: Vec<ControlOutputCommand>
}

impl ControlAssert {
    fn execute(&mut self, env: &mut Env) -> Result<(), AssemblerError> {
        visit_assert(&self.exp, self.txt.as_ref(), env, self.span.as_ref())?;
        Ok(())
    }
}

impl ControlOutputByte {
    fn execute(&mut self, env: &mut Env) -> Result<(), AssemblerError> {
        env.output_byte(self.value)?;
        Ok(())
    }
}

impl ControlOutputBytes {
    fn execute(&mut self, env: &mut Env) -> Result<(), AssemblerError> {
        env.output_bytes(&self.bytes)?;
        Ok(())
    }
}

impl ControlOrg {
    fn execute(&mut self, env: &mut Env) -> Result<(), AssemblerError> {
        env.visit_org_set_arguments(self.code, self.output)
    }
}

impl ControlOutputCommand {
    fn execute(&mut self, env: &mut Env) -> Result<(), AssemblerError> {
        match self {
            ControlOutputCommand::Assert(cmd) => cmd.execute(env),
            ControlOutputCommand::Byte(cmd) => cmd.execute(env),
            ControlOutputCommand::Bytes(cmd) => cmd.execute(env),
            ControlOutputCommand::Org(cmd) => cmd.execute(env),
            _ => unimplemented!()
        }
    }
}

impl ControlOutputStore {
    pub fn with_passes(passes: u8) -> Self {
        ControlOutputStore {
            remaining_passes: passes,
            commands: Default::default()
        }
    }

    pub fn store_byte(&mut self, value: u8) {
        match self.commands.last_mut() {
            Some(ControlOutputCommand::Byte(ControlOutputByte { value: previous })) => {
                let new_command = ControlOutputBytes {
                    bytes: vec![*previous, value]
                };
                self.commands.pop();
                self.commands.push(new_command.into());
            },
            Some(ControlOutputCommand::Bytes(ControlOutputBytes { bytes })) => {
                bytes.push(value);
            },
            _ => {
                self.commands.push(ControlOutputByte { value }.into());
            }
        }
    }

    pub fn store_org(&mut self, code: u16, output: u16) {
        self.commands.push(ControlOrg { code, output }.into())
    }

    pub fn store_assert(
        &mut self,
        exp: Expr,
        txt: Option<Vec<FormattedExpr>>,
        span: Option<Z80Span>
    ) {
        self.commands.push(ControlAssert { exp, txt, span }.into())
    }

    pub fn has_remaining_passes(&self) -> bool {
        self.remaining_passes > 0
    }

    pub fn execute(&mut self, env: &mut Env) -> Result<(), AssemblerError> {
        for cmd in &mut self.commands {
            cmd.execute(env)?;
        }
        Ok(())
    }

    pub fn new_pass(&mut self) {
        self.remaining_passes -= 1;
        self.commands.clear();
    }

    // pub fn extend<I: Iterator<Item = ControlOutputCommand> >(&mut self, iter: I) {
    // self.commands.extend(iter)
    // }
}
