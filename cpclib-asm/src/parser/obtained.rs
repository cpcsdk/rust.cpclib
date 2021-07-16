use std::ops::{DerefMut, Deref};

use cpclib_tokens::Token;


use super::{ParserContext, Z80Span};

pub struct LocatedToken<'src, 'ctx> {
    token: Token,
    span: Z80Span<'src, 'ctx>
}

impl<'src, 'ctx>  Deref for LocatedToken<'src, 'ctx>  {
    type Target = Token;
    fn deref(&self) -> &Self::Target {
        &self.token
    }
}

impl<'src, 'ctx>  DerefMut for LocatedToken<'src, 'ctx>  {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.token
    }
}

impl<'src, 'ctx> LocatedToken<'src, 'ctx> {
    pub fn token(&self) -> &Token {
        &self.token
    }

    pub fn span(&self) -> &Z80Span<'src, 'ctx> {
        &self.span
    }

    pub fn context(&self) -> &'ctx ParserContext {
        self.span().extra
    }
}
