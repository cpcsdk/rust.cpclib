use std::f64;

// fmod(n,m) calcule le modulo m de n
pub fn fmod(n: &ExprResult, m: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    let n = n.float()?;
    let m = m.float()?;
    Ok((n % m).into())
}

// atan2(y,x) calcule l'arc tangente de y/x
pub fn atan2(y: &ExprResult, x: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    let y = y.float()?;
    let x = x.float()?;
    Ok(y.atan2(x).into())
}

// hypot(x,y) calcule l'hypothénuse du triangle rectangle de côté x et y
pub fn hypot(x: &ExprResult, y: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    let x = x.float()?;
    let y = y.float()?;
    Ok(x.hypot(y).into())
}

// ldexp(x,exp) calcule x * 2 puissance exp
pub fn ldexp(x: &ExprResult, exp: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    let x = x.float()?;
    let exp = exp.int()?;
    Ok((x * 2f64.powi(exp as i32)).into())
}

// fdim(x,y) renvoie la différence positive de x-y, résultat toujours positif ou nul
pub fn fdim(x: &ExprResult, y: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    let x = x.float()?;
    let y = y.float()?;
    Ok((x - y).max(0.0).into())
}

// fstep(n,v) si n>=v alors renvoie 1 sinon 0
pub fn fstep(n: &ExprResult, v: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    let n = n.float()?;
    let v = v.float()?;
    Ok((if n >= v { 1.0 } else { 0.0 }).into())
}

// fmax(a,b) renvoie le maximum des deux valeurs
pub fn fmax(a: &ExprResult, b: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    let a = a.float()?;
    let b = b.float()?;
    Ok(a.max(b).into())
}

// fmin(a,b) renvoie le minimum des deux valeurs
pub fn fmin(a: &ExprResult, b: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    let a = a.float()?;
    let b = b.float()?;
    Ok(a.min(b).into())
}

// clamp(n,min,max) retourne la valeur n en la forçant dans l'intervale min:max
pub fn clamp(n: &ExprResult, min: &ExprResult, max: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    let n = n.float()?;
    let min = min.float()?;
    let max = max.float()?;
    Ok(n.max(min).min(max).into())
}

// lerp(i1,i2,n) calcule i1+n*(i2-i1)
pub fn lerp(i1: &ExprResult, i2: &ExprResult, n: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    let i1 = i1.float()?;
    let i2 = i2.float()?;
    let n = n.float()?;
    Ok((i1 + n * (i2 - i1)).into())
}

// isgreater(v,seuil) si v est supérieur strict au seuil, renvoie 1 sinon 0
pub fn isgreater(v: &ExprResult, seuil: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    let v = v.float()?;
    let seuil = seuil.float()?;
    Ok((if v > seuil { 1.0 } else { 0.0 }).into())
}

// isless(v,seuil) si v est inférieur strict au seuil, renvoie 1 sinon 0
pub fn isless(v: &ExprResult, seuil: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    let v = v.float()?;
    let seuil = seuil.float()?;
    Ok((if v < seuil { 1.0 } else { 0.0 }).into())
}

// fremain(n,d) calcule le reste du quotient de n et d
pub fn fremain(n: &ExprResult, d: &ExprResult) -> Result<ExprResult, Box<AssemblerError>> {
    let n = n.float()?;
    let d = d.float()?;
    Ok((n - d * (n / d).floor()).into())
}
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
