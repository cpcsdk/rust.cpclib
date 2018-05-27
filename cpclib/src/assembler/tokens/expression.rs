
use std::fmt;
use std::fmt::{Debug, Display, Formatter};

use assembler::assembler::{assemble_opcode,assemble_db_or_dw,assemble_defs,Bytes,SymbolsTable};

///TODO add the ability to manipulate value of different symbol
#[derive(PartialEq, Eq, Clone)]
pub enum Expr {
// types to manipulate
  Value(i32),
  Label(String),

  // Arithmetic operations
  Add(Box<Expr>, Box<Expr>),
  Sub(Box<Expr>, Box<Expr>),
  Mul(Box<Expr>, Box<Expr>),
  Div(Box<Expr>, Box<Expr>),
  Mod(Box<Expr>, Box<Expr>),

  Paren(Box<Expr>),

  // Boolean operations
  Equal(Box<Expr>, Box<Expr>),
  LowerOrEqual(Box<Expr>, Box<Expr>),
  GreaterOrEqual(Box<Expr>, Box<Expr>),
  StrictlyGreater(Box<Expr>, Box<Expr>),
  StrictlyLower(Box<Expr>, Box<Expr>),
}


impl Expr {

    /// Simple evaluation without context => can only evaluate number based operations.
    pub fn eval(&self) -> Result<i32, String> {
        let sym = SymbolsTable::default();
        self.resolve(&sym)
    }


    pub fn resolve(&self, sym: &SymbolsTable) -> Result<i32, String> {
        use self::Expr::*;

        let oper = |left: &Expr, right: &Expr, oper:Oper| -> Result<i32, String>{
            let res_left = left.resolve(sym);
            let res_right = right.resolve(sym);

            match (res_left, res_right) {
                (Ok(a), Ok(b)) => {
                    match oper {
                        Oper::Add => Ok(a+b),
                        Oper::Sub => Ok(a-b),
                        Oper::Mul => Ok(a*b),
                        Oper::Div => Ok(a/b),
                        Oper::Mod => Ok(a%b),

                        Oper::Equal => Ok( (a == b) as i32),

                        Oper::LowerOrEqual => Ok( (a <= b) as i32),
                        Oper::StrictlyLower=> Ok( (a < b) as i32),
                        Oper::GreaterOrEqual => Ok( (a >= b) as i32),
                        Oper::StrictlyGreater=> Ok( (a > b) as i32),
                    }
                },
                (Err(a), Ok(b))  => Err(format!("Unable to make the operation {:?}: {}", oper, a)),
                (Ok(a), Err(b))  => Err(format!("Unable to make the operation {:?}: {}", oper, b)),
                (Err(a), Err(b))  => Err(format!("Unable to make the operation {:?}: {} & {}", oper, a, b))
            }

        };

        match *self {
            Value(val) => Ok(val),

            Label(ref label) => {
                let val = sym.value(label);
                if val.is_some() {
                    Ok(val.unwrap())
                }
                else {
                    Err(format!("{} not found in symbol table", label))
                }
            }

            Add(ref left, ref right) => oper(left, right, Oper::Add),
            Sub(ref left, ref right) => oper(left, right, Oper::Sub),
            Mul(ref left, ref right) => oper(left, right, Oper::Mul),
            Div(ref left, ref right) => oper(left, right, Oper::Div),
            Mod(ref left, ref right) => oper(left, right, Oper::Mod),


            Equal(ref left, ref right) => oper(left, right, Oper::Equal),
            LowerOrEqual(ref left, ref right) => oper(left, right, Oper::LowerOrEqual),
            GreaterOrEqual(ref left, ref right) => oper(left, right, Oper::GreaterOrEqual),
            StrictlyGreater(ref left, ref right) => oper(left, right, Oper::StrictlyGreater),
            StrictlyLower(ref left, ref right) => oper(left, right, Oper::StrictlyLower),

            Paren(ref e) => e.resolve(sym),

            _ => Err(String::from("Need to implement the operation"))
        }
    }





    /// Check if it is necessary to read within a symbol table
    pub fn is_context_independant(&self) -> bool {
        use self::Expr::*;
        match *self {
            Label(ref label) => false,
            _ => true
        }
    }

}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Oper {
  Add,
  Sub,
  Mul,
  Div,
  Mod,

  Equal,
  LowerOrEqual,
  GreaterOrEqual,
  StrictlyGreater,
  StrictlyLower,
}


impl Display for Oper {
    fn fmt(&self, format: &mut Formatter) -> fmt::Result {
        use self::Oper::*;

        match self {
            &Add => write!(format, "+"),
            &Sub => write!(format, "-"),
            &Mul => write!(format, "*"),
            &Div => write!(format, "/"),
            &Mod => write!(format, "%"),

            &Equal => write!(format, "=="),
            &LowerOrEqual => write!(format, "<="),
            &GreaterOrEqual => write!(format, ">="),
            &StrictlyGreater  => write!(format, ">"),
            &StrictlyLower => write!(format, "<"),
        }
    }

}

impl Display for Expr {
  fn fmt(&self, format: &mut Formatter) -> fmt::Result {
    use self::Expr::*;
    match self {
      &Value(val) => write!(format, "0x{:x}", val),
      &Label(ref label) => write!(format, "{}", label),

      &Add(ref left, ref right) => write!(format, "{} + {}", left, right),
      &Sub(ref left, ref right) => write!(format, "{} - {}", left, right),
      &Mul(ref left, ref right) => write!(format, "{} * {}", left, right),
      &Mod(ref left, ref right) => write!(format, "{} % {}", left, right),
      &Div(ref left, ref right) => write!(format, "{} / {}", left, right),

      &Paren(ref expr) => write!(format, "({})", expr),


      &Equal(ref left, ref right) => write!(format, "{} == {}", left, right),
      &GreaterOrEqual(ref left, ref right) => write!(format, "{} >= {}", left, right),
      &StrictlyGreater(ref left, ref right) => write!(format, "{} > {}", left, right),
      &StrictlyLower(ref left, ref right) => write!(format, "{} < {}", left, right),
      &LowerOrEqual(ref left, ref right) => write!(format, "{} <= {}", left, right),
    }
  }
}

impl Debug for Expr {
  fn fmt(&self, format: &mut Formatter) -> fmt::Result {
    use self::Expr::*;
    match *self {
      Value(val) => write!(format, "{}", val),
      Label(ref label) => write!(format, "{}", label),

      Add(ref left, ref right) => write!(format, "({:?} + {:?})", left, right),
      Sub(ref left, ref right) => write!(format, "({:?} - {:?})", left, right),
      Mul(ref left, ref right) => write!(format, "({:?} * {:?})", left, right),
      Mod(ref left, ref right) => write!(format, "({:?} % {:?})", left, right),
      Div(ref left, ref right) => write!(format, "({:?} / {:?})", left, right),

      Paren(ref expr) => write!(format, "[{:?}]", expr),


      Equal(ref left, ref right) => write!(format, "{:?} == {:?}", left, right),
      GreaterOrEqual(ref left, ref right) => write!(format, "{:?} >= {:?}", left, right),
      StrictlyGreater(ref left, ref right) => write!(format, "{:?} > {:?}", left, right),
      StrictlyLower(ref left, ref right) => write!(format, "{:?} < {:?}", left, right),
      LowerOrEqual(ref left, ref right) => write!(format, "{:?} <= {:?}", left, right),

    }
  }
}
