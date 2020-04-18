use crate::*;
use crate::builder;

/// Based on some suggestions provide here: http://z80-heaven.wikidot.com/optimization
/// propose some classes to optimize code

/// This trait concerns simple optimisations that transform one Instruction in another one
trait SingleInstructionModifier {
    /// REturn True if the token can be replaces
    fn match(&self, token: &Token) -> bool;
    /// Generate the good value (never use if it does not match)
    fn replace(&self, token: &Token) -> Token;

    fn replace_if_match(&self, token: &Token) -> Token {
        if self.match(token) {
            self.replace(token)
        }
        else {
            *token
        }
    }
}

/// Favor xor a to Ld A
struct ResetA;
/// Favor or a over cp 0
struct CompareA;

impl SingleInstructionModifier for ResetA {
    fn match(&self, token: &Token) -> bool {
        match token {
            Token::OpCode(
                Mnemonic::Ld, 
                Some(DataAccess::Register8(Register8::A)),
                Some(DataAccess::Expression(Expression::Value(0))) // TODO allow to compute on the fly if it is 0
            ) => true,
            _ => false
        }
    }

    fn replace(&self, token: &Token) -> Token {
        xor_a()
    }
}


impl SingleInstructionModifier for CompareA {
    fn match(&self, token: &Token) -> bool {
        match token {
            Token::OpCode(
                Mnemonic::Cp, 
                Some(DataAccess::Expression(Expression::Value(0))), // TODO allow to compute on the fly if it is 0
                None
            ) => true,
            _ => false
        }
    }

    fn replace(&self, token: &Token) -> Token {
        or_a()
    }
}


/// This trait concerns the modification of several instructions in a row
trait MultipleInstructionsModifier {
    /// If there is no rewrite, return None, Otherwise return the modified stream
    fn rewrite_one(tokens: &[Token]) -> Option<Vec<Token>>;

    /// Rewrite at maximum n times
    pub fn rewrite_n(tokens: &[Token], n: usize) -> Option<Vec<Token>> {
        let previous_run = None;
        for i in 0..n {
            let to_rewrite = match previous_run {
                Some(ref tokens) => tokens,
                None => tokens
            };

            let result = self.rewrite_one(to_rewrite);
            if result.is_none() {
                return previous_run;
            }
            else {
                previous_run = result;
            }
        }
        return previous_run;
    }

    /// Rewrite until it is not possible anymore
    pub fn rewrite(tokens: &[Token]) -> Option<Vec<Token>> {
        let previous_run = None;
        loop {
            let tokens = match previous_run {
                Some(ref tokens) => tokens,
                None => tokens
            };

            let current_run = self.rewrite_one(tokens);
            if current_run.is_none() {
                return previous_run;
            }
            else {
                previous_run = current_run;
            }
        }
    }

}

/// Fuse two successive DB in a single DB directive
#[derive(Default, Debug)]
struct FuseTwoSuccessiveDefbIntoDefb;

impl MultipleInstructionsModifier for FuseTwoSuccessiveDefbIntoDefb {
    /// If there is no rewrite, return None, Otherwise return the modified stream
    fn rewrite_one(tokens: &[Token]) -> Option<Vec<Token>> {
        match tokens {
            beginning @.., Token::Defb(ref a), Token::Defb(ref b), end @.. 
            => {
                let mut bytes = Vec::new();
                bytes.append(a);
                bytes.append(b);

                let mut tokens = Vec::new();
                tokens.append_elements(beginning);
                tokens.push(builder::defb_elements(bytes);
                tokens.append_elements(end);

                Some(tokens)
            },

            _ => None
        }
    }


}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn optimize_consecutive_db_standard_case() {
        let from = [builder::nop(), builder::defb(0xa), builder::defb(0xa), builder::nop()];
        let to = [builder::nop(), builder::defb_elements([0xa, 0xb].into())];
        let obtained = FuseTwoSuccessiveDefbIntoDefb::default().rewrite(&from);

        assert_eq!(
            to,
            obtained.as_slice()
        )
    }
}