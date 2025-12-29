use cpclib_tokens::ExprResult;

use crate::error::{AssemblerError, ExpressionError};

pub fn min(args: &[ExprResult]) -> Result<ExprResult, Box<AssemblerError>> {
    if args.len() < 2 {
        return Err(Box::new(AssemblerError::FunctionWithWrongNumberOfArguments(
            "min".to_string(),
            either::Either::Left(2),
            args.len(),
        )));
    }
    let mut min = &args[0];
    for arg in &args[1..] {
        min = min.min(arg);
    }
    Ok(min.clone())
}

pub fn max(args: &[ExprResult]) -> Result<ExprResult, Box<AssemblerError>> {
    if args.len() < 2 {
        return Err(Box::new(AssemblerError::FunctionWithWrongNumberOfArguments(
            "max".to_string(),
            either::Either::Left(2),
            args.len(),
        )));
    }
    let mut max = &args[0];
    for arg in &args[1..] {
        max = max.max(arg);
    }
    Ok(max.clone())
}

pub fn pow(a: &ExprResult, b: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    let power = b.int()?;
    match a {
        ExprResult::Float(f) => Ok(f.into_inner().powf(power as f64).into()),
        ExprResult::Value(v) => Ok(v.pow(power as _).into()),

        ExprResult::List(_) => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: "pow cannot be applied to a list".to_string()
                }))
            )))
        },

        _ => {
            Err(Box::new(AssemblerError::ExpressionError(
                ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                    msg: "pow cannot be applied to a string".to_string()
                }))
            )))
        },
    }
}

pub fn high(arg: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    let arg = arg.int().map_err(AssemblerError::ExpressionTypeError)?;
    Ok((arg >> 8 & 0xFF).into())
}

pub fn low(arg: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    let arg = arg.int().map_err(AssemblerError::ExpressionTypeError)?;
    Ok((arg & 0xFF).into())
}

pub fn peek(arg: &ExprResult, env: &crate::Env) -> Result<ExprResult, Box<AssemblerError>> {
    let arg = arg.int()?;
    if !(0..=0xFFFF).contains(&arg) {
        Err(Box::new(AssemblerError::ExpressionError(
            ExpressionError::OwnError(Box::new(AssemblerError::AssemblingError {
                msg: format!("Impossible to read memory address 0x{:X}", arg)
            }))
        )))
    }
    else {
        Ok(env.peek(&env.logical_to_physical_address(arg as _)).into())
    }
}

pub fn floor(arg: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    Ok(arg.floor().map_err(AssemblerError::ExpressionTypeError)?)
}

pub fn ceil(arg: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    Ok(arg.ceil().map_err(AssemblerError::ExpressionTypeError)?)
}

pub fn frac(arg: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    Ok(arg.frac().map_err(AssemblerError::ExpressionTypeError)?)
}

pub fn int(arg: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    Ok(arg
        .int()
        .map(|i| i.into())
        .map_err(AssemblerError::ExpressionTypeError)?)
}

pub fn char(arg: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    Ok(arg
        .char()
        .map(|i| i.into())
        .map_err(AssemblerError::ExpressionTypeError)?)
}

pub fn sin(arg: &ExprResult, env: &crate::Env) -> Result<ExprResult, Box<AssemblerError>> {
    if env.options().parse_options().is_orgams() {
        dbg!("We need to check things here");
        return Ok((512.0 * (arg.float()? * 3.1415926545 / (256.0 / 2.0)).sin()).into());
    }
    Ok(if env.options().parse_options().is_orgams() {
        dbg!("We need to check things here");
        dbg!(Ok((512.0
            * (arg.float()? * 3.1415926545 / (256.0 / 2.0)).sin())
        .into()))
    }
    else {
        arg.sin()
    }
    .map_err(AssemblerError::ExpressionTypeError)?)
}

pub fn cos(arg: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    Ok(arg.cos().map_err(AssemblerError::ExpressionTypeError)?)
}
pub fn asin(arg: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    Ok(arg.asin().map_err(AssemblerError::ExpressionTypeError)?)
}
pub fn acos(arg: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    Ok(arg.acos().map_err(AssemblerError::ExpressionTypeError)?)
}
pub fn abs(arg: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    Ok(arg.abs().map_err(AssemblerError::ExpressionTypeError)?)
}
pub fn ln(arg: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    Ok(arg.ln().map_err(AssemblerError::ExpressionTypeError)?)
}
pub fn log10(arg: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    Ok(arg.log10().map_err(AssemblerError::ExpressionTypeError)?)
}
pub fn exp(arg: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    Ok(arg.exp().map_err(AssemblerError::ExpressionTypeError)?)
}
pub fn sqrt(arg: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    Ok(arg.sqrt().map_err(AssemblerError::ExpressionTypeError)?)
}
