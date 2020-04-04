use proc_macro2::*;
use quote::{TokenStreamExt, ToTokens};
use cpclib_asm::preamble::*;

/// Create another trait as we cannot implement ToToken directly :(
pub trait MyToTokens {
    fn to_tokens(&self, tokens: &mut TokenStream);
}

impl<T> MyToTokens for Option<T> 
where T: MyToTokens
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            None => tokens.append(Ident::new("None", Span::call_site())),
            Some(t) => {
                tokens.append(Ident::new("Some", Span::call_site()));
                let mut inner = TokenStream::new();
                t.to_tokens(&mut inner);
                tokens.append(Group::new(Delimiter::Parenthesis, inner));
            }

        }
    }
}

impl MyToTokens for Listing {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Ident::new("Listing", Span::call_site()));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Ident::new("new_with", Span::call_site()));

        let mut vec_tokens = TokenStream::new();
        vec_tokens.append(Punct::new('&', Spacing::Joint));
        vec_tokens.append(Ident::new("vec", Span::call_site()));
        vec_tokens.append(Punct::new('!', Spacing::Joint));

        let mut inner_token = TokenStream::new();
        for token in self.listing() {
            token.to_tokens(&mut inner_token);
            inner_token.append(Punct::new(',', Spacing::Joint));
        }
        vec_tokens.append(Group::new(Delimiter::Bracket, inner_token));

        tokens.append(Group::new(Delimiter::Parenthesis, vec_tokens));
    }
}

impl MyToTokens for Token {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Ident::new("Token", Span::call_site()));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Punct::new(':', Spacing::Joint));

        match self {
            Self::OpCode(mnemo, arg1, arg2) => {

                tokens.append(Ident::new("OpCode", Span::call_site()));
                let mut inner_content = TokenStream::new();

                mnemo.to_tokens(&mut inner_content);
                inner_content.append(Punct::new(',', Spacing::Joint));

                arg1.to_tokens(&mut inner_content);
                inner_content.append(Punct::new(',', Spacing::Joint));

                arg2.to_tokens(&mut inner_content);

                tokens.append(Group::new(Delimiter::Parenthesis, inner_content));
            }
            _ => unimplemented!()
        }
    }
}

impl MyToTokens for Mnemonic {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Ident::new("Mnemonic", Span::call_site()));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Punct::new(':', Spacing::Joint));

        let mnemo = match self {

            Mnemonic::ExAf => "ExAf".to_owned(),
            Mnemonic::ExHlDe => "ExHlDe".to_owned(),
            Mnemonic::ExMemSp => "ExMemSp".to_owned(),
            Mnemonic::Nops2 => "Nops2".to_owned(),
            _ => {
                let repr = self.to_string();
                format!("{}{}", repr.as_str()[0..=0].to_uppercase(),  repr[1..].to_lowercase())
            }
        };

        tokens.append(Ident::new(&mnemo, Span::call_site()));
    }
}

impl MyToTokens for DataAccess {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Ident::new("DataAccess", Span::call_site()));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Punct::new(':', Spacing::Joint));

        match self {
            DataAccess::Expression(exp) => {
                tokens.append(Ident::new("Expression", Span::call_site()));
                let mut inside = TokenStream::new();
                exp.to_tokens(&mut inside);
                tokens.append(Group::new(Delimiter::Parenthesis, inside));
            },

            DataAccess::Register8(reg) => {
                tokens.append(Ident::new("Register8", Span::call_site()));
                let mut inside = TokenStream::new();
                reg.to_tokens(&mut inside);
                tokens.append(Group::new(Delimiter::Parenthesis, inside));
            },

            DataAccess::Register16(reg) => {
                tokens.append(Ident::new("Register16", Span::call_site()));
                let mut inside = TokenStream::new();
                reg.to_tokens(&mut inside);
                tokens.append(Group::new(Delimiter::Parenthesis, inside));
            },
            
            _=> unimplemented!("{:?}", self)
        }
    }
}


impl MyToTokens for Register8 {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Ident::new("Register8", Span::call_site()));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Ident::new(&self.to_string(), Span::call_site()));
    }
}

impl MyToTokens for Register16 {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Ident::new("Register16", Span::call_site()));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Punct::new(':', Spacing::Joint));

        let repr = {
            let repr = self.to_string();
            format!("{}{}", repr.as_str()[0..=0].to_uppercase(),  repr[1..].to_lowercase())
        };

        tokens.append(Ident::new(&repr, Span::call_site()));
    }
}

impl MyToTokens for Expr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Ident::new("Expr", Span::call_site()));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Punct::new(':', Spacing::Joint));

        match self {
            Expr::Value(val) => {
                tokens.append(Ident::new("Value", Span::call_site()));
                let mut inside = TokenStream::new();
                if *val < 0 {
                    inside.append(Punct::new('-', Spacing::Joint));

                }
                inside.append(Literal::u32_unsuffixed(val.abs() as u32));
                tokens.append(Group::new(Delimiter::Parenthesis, inside));
            }
            _=> unimplemented!("{:?}", self)
        }
    }

}