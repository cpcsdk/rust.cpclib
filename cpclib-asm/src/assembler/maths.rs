use cpclib_tokens::ExprResult;

use crate::{AssemblerError, ExpressionError};

pub fn min(a: &ExprResult, b: &ExprResult) -> Result<ExprResult, AssemblerError> {
    Ok(a.min(b).clone())
}

pub fn max(a: &ExprResult, b: &ExprResult) -> Result<ExprResult, AssemblerError> {
    Ok(a.max(b).clone())
}

pub fn pow(a: &ExprResult, b: &ExprResult) -> Result<ExprResult, AssemblerError> {
    let power = b.int()?;
    match a {
        ExprResult::Float(f) => Ok(f.into_inner().powf(power as f64).into()),
        ExprResult::Value(v) => Ok(v.pow(power as _).into()),

        ExprResult::List(_) => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("pow cannot be applied to a list")
                })
            )))
        },

        _ => {
            Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
                Box::new(AssemblerError::AssemblingError {
                    msg: format!("pow cannot be applied to a string")
                })
            )))
        },
    }
}

pub fn high(arg: &ExprResult) -> Result<ExprResult, AssemblerError> {
    let arg = arg
        .int()
        .map_err(|e| AssemblerError::ExpressionTypeError(e))?;
    Ok(((arg >> 8) & 0xFF).into())
}

pub fn low(arg: &ExprResult) -> Result<ExprResult, AssemblerError> {
    let arg = arg
        .int()
        .map_err(|e| AssemblerError::ExpressionTypeError(e))?;
    Ok((arg & 0xFF).into())
}

pub fn peek(arg: &ExprResult, env: &crate::Env) -> Result<ExprResult, AssemblerError> {
    let arg = arg.int()?;
    if arg < 0 || arg > 0xFFFF {
        return Err(AssemblerError::ExpressionError(ExpressionError::OwnError(
            Box::new(AssemblerError::AssemblingError {
                msg: format!("Impossible to read memory address 0x{:X}", arg)
            })
        )));
    }
    else {
        Ok(env.peek(&env.logical_to_physical_address(arg as _)).into())
    }
}

pub fn floor(arg: &ExprResult) -> Result<ExprResult, AssemblerError> {
    (arg.floor()).map_err(|e| AssemblerError::ExpressionTypeError(e))
}

pub fn ceil(arg: &ExprResult) -> Result<ExprResult, AssemblerError> {
    (arg.ceil()).map_err(|e| AssemblerError::ExpressionTypeError(e))
}

pub fn frac(arg: &ExprResult) -> Result<ExprResult, AssemblerError> {
    (arg.frac()).map_err(|e| AssemblerError::ExpressionTypeError(e))
}

pub fn int(arg: &ExprResult) -> Result<ExprResult, AssemblerError> {
    (arg.int())
        .map(|i| i.into())
        .map_err(|e| AssemblerError::ExpressionTypeError(e))
}

pub fn char(arg: &ExprResult) -> Result<ExprResult, AssemblerError> {
    (arg.char())
        .map(|i| i.into())
        .map_err(|e| AssemblerError::ExpressionTypeError(e))
}

pub fn sin(arg: &ExprResult, env: &crate::Env) -> Result<ExprResult, AssemblerError> {
    if env.options().parse_options().is_orgams() {
        dbg!("We need to check things here");
        dbg!(Ok((512.0
            * (arg.float()? * 3.1415926545 / (256.0 / 2.0)).sin())
        .into()))
    }
    else {
        arg.sin()
    }
    .map_err(|e| AssemblerError::ExpressionTypeError(e))
}

pub fn cos(arg: &ExprResult) -> Result<ExprResult, AssemblerError> {
    (arg.cos()).map_err(|e| AssemblerError::ExpressionTypeError(e))
}
pub fn asin(arg: &ExprResult) -> Result<ExprResult, AssemblerError> {
    (arg.asin()).map_err(|e| AssemblerError::ExpressionTypeError(e))
}
pub fn acos(arg: &ExprResult) -> Result<ExprResult, AssemblerError> {
    (arg.acos()).map_err(|e| AssemblerError::ExpressionTypeError(e))
}
pub fn abs(arg: &ExprResult) -> Result<ExprResult, AssemblerError> {
    (arg.abs()).map_err(|e| AssemblerError::ExpressionTypeError(e))
}
pub fn ln(arg: &ExprResult) -> Result<ExprResult, AssemblerError> {
    (arg.ln()).map_err(|e| AssemblerError::ExpressionTypeError(e))
}
pub fn log10(arg: &ExprResult) -> Result<ExprResult, AssemblerError> {
    (arg.log10()).map_err(|e| AssemblerError::ExpressionTypeError(e))
}
pub fn exp(arg: &ExprResult) -> Result<ExprResult, AssemblerError> {
    (arg.exp()).map_err(|e| AssemblerError::ExpressionTypeError(e))
}
pub fn sqrt(arg: &ExprResult) -> Result<ExprResult, AssemblerError> {
    (arg.sqrt()).map_err(|e| AssemblerError::ExpressionTypeError(e))
}
