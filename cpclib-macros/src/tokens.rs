use cpclib_asm::preamble::{
    BinaryFunction, DataAccess, Expr, FlagTest, Listing, Mnemonic, Register16, Register8,
    StableTickerAction, Token, UnaryFunction
};
use cpclib_common::smol_str::SmolStr;
use proc_macro2::*;
use quote::TokenStreamExt;

fn upper_first(repr: &str) -> String {
    format!("{}{}", repr[0..=0].to_uppercase(), repr[1..].to_lowercase())
}

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

impl MyToTokens for str {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Literal::string(self));
    }
}

impl MyToTokens for String {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.as_str().to_tokens(tokens);
        tokens.append(Punct::new('.', Spacing::Joint));
        tokens.append(Ident::new("to_string", Span::call_site()));
        let inner_token = TokenStream::new();
        tokens.append(Group::new(Delimiter::Parenthesis, inner_token));
    }
}

impl MyToTokens for SmolStr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Ident::new("SmolStr", Span::call_site()));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Ident::new("new", Span::call_site()));

        let mut inner_token = TokenStream::new();
        (**self).to_tokens(&mut inner_token);
        tokens.append(Group::new(Delimiter::Parenthesis, inner_token));
    }
}

impl<T: ?Sized + MyToTokens> MyToTokens for Box<T> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Ident::new("Box", Span::call_site()));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Ident::new("new", Span::call_site()));

        let mut inner_token = TokenStream::new();
        (**self).to_tokens(&mut inner_token);
        tokens.append(Group::new(Delimiter::Parenthesis, inner_token));
    }
}

impl<T1: Sized + MyToTokens, T2: ?Sized + MyToTokens> MyToTokens for (T1, T2) {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut inner_token = TokenStream::new();
        self.0.to_tokens(&mut inner_token);
        inner_token.append(Punct::new(',', Spacing::Joint));
        self.1.to_tokens(&mut inner_token);
        tokens.append(Group::new(Delimiter::Parenthesis, inner_token));
    }
}

impl<T: Sized + MyToTokens> MyToTokens for &[T] {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Ident::new("vec!", Span::call_site()));

        let mut inner_token = TokenStream::new();
        for elem in self.iter() {
            elem.to_tokens(&mut inner_token);
            inner_token.append(Punct::new(':', Spacing::Joint));
        }
        tokens.append(Group::new(Delimiter::Bracket, inner_token));
    }
}

// impl<T> MyToTokens for T where T: ToTokens {
// fn to_tokens(&self, tokens: &mut TokenStream) {
// self.to_tokens()
// }
// }

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

fn no_param(name: &str, tokens: &mut TokenStream) {
    tokens.append(Ident::new(name, Span::call_site()));
}

fn one_param<T>(name: &str, t: &T, tokens: &mut TokenStream)
where T: MyToTokens + ?Sized {
    tokens.append(Ident::new(name, Span::call_site()));
    let mut inside = TokenStream::new();
    t.to_tokens(&mut inside);
    tokens.append(Group::new(Delimiter::Parenthesis, inside));
}

fn two_params<T1, T2>(name: &str, t1: &T1, t2: &T2, tokens: &mut TokenStream)
where
    T1: MyToTokens,
    T2: MyToTokens
{
    tokens.append(Ident::new(name, Span::call_site()));

    let mut inside = TokenStream::new();
    t1.to_tokens(&mut inside);
    inside.append(Punct::new(',', Spacing::Joint));
    t2.to_tokens(&mut inside);

    tokens.append(Group::new(Delimiter::Parenthesis, inside));
}


fn two_named_params<T1, T2>(name: &str, field1: &str,  t1: &T1, field2: &str, t2: &T2, tokens: &mut TokenStream)
where
    T1: MyToTokens,
    T2: MyToTokens
{
    tokens.append(Ident::new(name, Span::call_site()));

    let mut inside = TokenStream::new();
    inside.append(Ident::new(field1, Span::call_site()));
    inside.append(Punct::new(':', Spacing::Joint));
    t1.to_tokens(&mut inside);
    inside.append(Punct::new(',', Spacing::Joint));
    inside.append(Ident::new(field2, Span::call_site()));
    inside.append(Punct::new(':', Spacing::Joint));
    t2.to_tokens(&mut inside);

    tokens.append(Group::new(Delimiter::Bracket, inside));
}

fn three_params<T1, T2, T3>(name: &str, t1: &T1, t2: &T2, t3: &T3, tokens: &mut TokenStream)
where
    T1: MyToTokens,
    T2: MyToTokens,
    T3: MyToTokens
{
    tokens.append(Ident::new(name, Span::call_site()));

    let mut inside = TokenStream::new();
    t1.to_tokens(&mut inside);
    inside.append(Punct::new(',', Spacing::Joint));
    t2.to_tokens(&mut inside);
    inside.append(Punct::new(',', Spacing::Joint));
    t3.to_tokens(&mut inside);

    tokens.append(Group::new(Delimiter::Parenthesis, inside));
}

fn four_params<T1, T2, T3, T4>(
    name: &str,
    t1: &T1,
    t2: &T2,
    t3: &T3,
    t4: &T4,
    tokens: &mut TokenStream
) where
    T1: MyToTokens,
    T2: MyToTokens,
    T3: MyToTokens,
    T4: MyToTokens
{
    tokens.append(Ident::new(name, Span::call_site()));

    let mut inside = TokenStream::new();
    t1.to_tokens(&mut inside);
    inside.append(Punct::new(',', Spacing::Joint));
    t2.to_tokens(&mut inside);
    inside.append(Punct::new(',', Spacing::Joint));
    t3.to_tokens(&mut inside);
    inside.append(Punct::new(',', Spacing::Joint));
    t4.to_tokens(&mut inside);

    tokens.append(Group::new(Delimiter::Parenthesis, inside));
}

impl MyToTokens for Token {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Ident::new("Token", Span::call_site()));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Punct::new(':', Spacing::Joint));

        match self {
            Self::Comment(arg) => {
                one_param("Comment", arg, tokens);
            }

            Self::Defs(args) => {
                tokens.append(Ident::new("Defs", Span::call_site()));

                let mut vec_tokens = TokenStream::new();
                vec_tokens.append(Punct::new('&', Spacing::Joint));
                vec_tokens.append(Ident::new("vec", Span::call_site()));
                vec_tokens.append(Punct::new('!', Spacing::Joint));

                let mut inner_token = TokenStream::new();
                for arg in args.iter() {
                    arg.to_tokens(&mut inner_token);
                    inner_token.append(Punct::new(',', Spacing::Joint));
                }
                vec_tokens.append(Group::new(Delimiter::Bracket, inner_token));

                tokens.append(Group::new(Delimiter::Parenthesis, vec_tokens));
            }

            Self::Equ{label, expr} => {
                two_named_params("Equ", "label", label, "expr", expr, tokens);

            }

            Self::Label(arg) => {
                one_param("Label", arg, tokens);
            }

            Self::OpCode(mnemo, arg1, arg2, arg3) => {
                tokens.append(Ident::new("OpCode", Span::call_site()));
                let mut inner_content = TokenStream::new();

                mnemo.to_tokens(&mut inner_content);
                inner_content.append(Punct::new(',', Spacing::Joint));

                arg1.to_tokens(&mut inner_content);
                inner_content.append(Punct::new(',', Spacing::Joint));

                arg2.to_tokens(&mut inner_content);
                inner_content.append(Punct::new(',', Spacing::Joint));

                arg3.to_tokens(&mut inner_content);

                tokens.append(Group::new(Delimiter::Parenthesis, inner_content));
            }

            Self::Org{val1, val2} => {
                two_named_params("Org", "val1", val1, "val2", val2, tokens);
            }

            Self::StableTicker(arg) => {
                one_param("StableTicker", arg, tokens);
            }

            Self::Repeat(exp, lst, lab, count) => {
                four_params("Repeat", exp, lst, lab, count, tokens);
            }

            _ => unimplemented!("impl MyToTokens for Token {{ fn to_tokens ...}} {:?}", self)
        }
    }
}

impl<S: AsRef<str>> MyToTokens for StableTickerAction<S> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        no_param("StableTickerAction", tokens);
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Punct::new(':', Spacing::Joint));

        match self {
            StableTickerAction::Start(label) => {
                one_param("Start", label.as_ref(), tokens);
            }

            StableTickerAction::Stop => {
                no_param("Stop", tokens);
            }
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
            Mnemonic::Nop2 => "Nops2".to_owned(),
            _ => upper_first(&self.to_string())
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
            }

            DataAccess::FlagTest(arg) => {
                one_param("FlagTest", arg, tokens);
            }

            DataAccess::Memory(arg) => {
                one_param("Memory", arg, tokens);
            }

            DataAccess::PortC => {
                no_param("PortC", tokens);
            }

            DataAccess::Register8(reg) => {
                tokens.append(Ident::new("Register8", Span::call_site()));
                let mut inside = TokenStream::new();
                reg.to_tokens(&mut inside);
                tokens.append(Group::new(Delimiter::Parenthesis, inside));
            }

            DataAccess::Register16(reg) => {
                tokens.append(Ident::new("Register16", Span::call_site()));
                let mut inside = TokenStream::new();
                reg.to_tokens(&mut inside);
                tokens.append(Group::new(Delimiter::Parenthesis, inside));
            }

            DataAccess::MemoryRegister16(reg) => {
                tokens.append(Ident::new("MemoryRegister16", Span::call_site()));
                let mut inside = TokenStream::new();
                reg.to_tokens(&mut inside);
                tokens.append(Group::new(Delimiter::Parenthesis, inside))
            }

            _ => unimplemented!("DataAccess::{:?}", self)
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

        let repr = upper_first(&self.to_string());

        tokens.append(Ident::new(&repr, Span::call_site()));
    }
}

impl MyToTokens for FlagTest {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Ident::new("FlagTest", Span::call_site()));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Punct::new(':', Spacing::Joint));

        no_param(&self.to_string(), tokens);
    }
}

impl MyToTokens for UnaryFunction {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Ident::new("UnaryFunction", Span::call_site()));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Punct::new(':', Spacing::Joint));

        no_param(&upper_first(&self.to_string()), tokens);
    }
}

impl MyToTokens for BinaryFunction {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Ident::new("BinaryFunction", Span::call_site()));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Punct::new(':', Spacing::Joint));

        no_param(&upper_first(&self.to_string()), tokens);
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
                inside.append(Literal::u32_unsuffixed(val.unsigned_abs()));
                tokens.append(Group::new(Delimiter::Parenthesis, inside));
            }

            Expr::String(val) => {
                one_param("String", val, tokens);
            }

            Expr::Label(val) => {
                one_param("Label", val, tokens);
            }

            Expr::Paren(val) => {
                one_param("Paren", val, tokens);
            }

            Expr::UnaryFunction(func, arg) => {
                two_params("UnaryFunction", func, arg, tokens);
            }

            Expr::BinaryFunction(func, arg1, arg2) => {
                three_params("BinaryFunction", func, arg1, arg2, tokens);
            }

            _ => unimplemented!("Expr::{:?}", self)
        }
    }
}
