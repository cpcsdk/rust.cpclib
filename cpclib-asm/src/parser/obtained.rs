use std::{
    convert::TryFrom,
    fs::File,
    io::Read,
    ops::{Deref, DerefMut},
    rc::Rc,
    thread::LocalKey,
};

use cpclib_tokens::{
    BaseListing, BinaryTransformation, CrunchType, Expr, Listing, ListingElement, TestKind, Token,
};
use itertools::Itertools;

use crate::{
    error::AssemblerError,
    implementation::expression::ExprEvaluationExt,
    preamble::{parse_z80_str, parse_z80_strrc_with_contextrc},
};

use super::parse_z80_str_with_context;
use super::{parse_z80_code, ParserContext, Z80Span};

///! This crate is related to the adaptation of tokens and listing for the case where they are parsed

#[derive(Clone, Debug)]
/// Add span information for a Token.
/// This hierarchy is a mirror of the original token one
pub enum LocatedToken {
    /// A token without any listing embedding
    Standard {
        /// The original token without any span information
        token: Token,
        /// The span that correspond to the token
        span: Z80Span,
    },
    CrunchedSection(CrunchType, LocatedListing, Z80Span),
    Include(String, std::cell::RefCell<Option<LocatedListing>>, Z80Span),
    If(
        Vec<(TestKind, LocatedListing)>,
        Option<LocatedListing>,
        Z80Span,
    ),
    Repeat(Expr, LocatedListing, Option<String>, Z80Span),
    RepeatUntil(Expr, LocatedListing, Z80Span),
    Rorg(Expr, LocatedListing, Z80Span),
    Switch(Vec<(Expr, LocatedListing)>, Z80Span),
    While(Expr, LocatedListing, Z80Span),
}

impl Deref for LocatedToken {
    type Target = Token;
    fn deref(&self) -> &Self::Target {
        match self.token() {
            Ok(t) => t,
            Err(_) => {
                panic!("{:?} cannot be dereferenced as it contains a listing", self)
            }
        }
    }
}

impl LocatedToken {
    /// We can obtain a token only for "standard ones". Those that rely on listing need to be handled differently
    pub fn token(&self) -> Result<&Token, ()> {
        match self {
            Self::Standard { token, .. } => Ok(token),
            _ => Err(()),
        }
    }

    /// Get the span of the current token
    pub fn span(&self) -> &Z80Span {
        match self {
            Self::Standard { span, .. }
            | Self::CrunchedSection(_, _, span)
            | Self::Include(_, _, span)
            | Self::If(_, _, span)
            | Self::Repeat(_, _, _, span)
            | Self::RepeatUntil(_, _, span)
            | Self::Rorg(_, _, span)
            | Self::Switch(_, span)
            | Self::While(_, _, span) => span,
        }
    }

    pub fn context(&self) -> &(Rc<String>, Rc<ParserContext>) {
        &self.span().extra
    }
}

impl LocatedToken {
    pub fn as_token(&self) -> Token {
        match self {
            LocatedToken::Standard { token, .. } => token.clone(),
            LocatedToken::CrunchedSection(c, l, _span) => {
                Token::CrunchedSection(*c, l.as_listing())
            }
            LocatedToken::Include(s, l, _span) => Token::Include(
                s.clone(),
                l.borrow().as_ref().map(|l| l.as_listing()).into(),
            ),
            LocatedToken::If(v, e, _span) => Token::If(
                v.iter()
                    .map(|(k, l)| (k.clone(), l.as_listing()))
                    .collect_vec(),
                e.as_ref().map(|l| l.as_listing()),
            ),
            LocatedToken::Repeat(e, l, s, _span) => {
                Token::Repeat(e.clone(), l.as_listing(), s.clone())
            }
            LocatedToken::RepeatUntil(e, l, _span) => Token::RepeatUntil(e.clone(), l.as_listing()),
            LocatedToken::Rorg(e, l, _span) => Token::Rorg(e.clone(), l.as_listing()),
            LocatedToken::Switch(v, _span) => Token::Switch(
                v.iter()
                    .map(|(e, l)| (e.clone(), l.as_listing()))
                    .collect_vec(),
            ),
            LocatedToken::While(e, l, _span) => Token::While(e.clone(), l.as_listing()),
        }
    }

    pub fn parse_token(value: &str) -> Result<LocatedToken, String> {
        let tokens = {
            let res = parse_z80_str(value);
            match res {
                Ok(tokens) => tokens,
                Err(_e) => {
                    return Err("ERROR -- need to code why ...".to_owned());
                }
            }
        };
        match tokens.len() {
            0 => Err("No ASM found.".to_owned()),
            1 => {
                let token = tokens[0].clone();
                Ok(token)
            }
            _ => Err(format!(
                "{} tokens are present instead of one",
                tokens.len()
            )),
        }
    }

    /// Modify the few tokens that need to read files
    /// Works in read only tokens thanks to RefCell
    pub fn read_referenced_file(&self, ctx: &ParserContext) -> Result<(), AssemblerError> {
        dbg!(12);
        match self {
            LocatedToken::Include(ref fname, ref cell, span) => {
                dbg!(34);
                match ctx.get_path_for(fname) {
                    Err(e) => {
                        return Err(AssemblerError::IOError {
                            msg: format!("{:?} not found. {:?}", fname, e),
                        });
                    }
                    Ok(ref fname) => {
                        let mut f = File::open(&fname).map_err(|e| AssemblerError::IOError {
                            msg: format!("Unable to open {:?}. {}", fname, e),
                        })?;

                        let mut content = Vec::new();
                        f.read_to_end(&mut content)
                            .map_err(|e| AssemblerError::IOError {
                                msg: format!("Unable to read {:?}. {}", fname, e.to_string()),
                            })?;

                        let result = chardet::detect(&content);
                        let coder = encoding::label::encoding_from_whatwg_label(
                            chardet::charset2encoding(&result.0),
                        );

                        let content = match coder {
                            Some(coder) => {
                                let utf8reader = coder
                                    .decode(&content, encoding::DecoderTrap::Ignore)
                                    .expect("Error");
                                utf8reader.to_string()
                            }
                            None => {
                                return Err(AssemblerError::IOError {
                                    msg: format!("Encoding error for {:?}.", fname),
                                });
                            }
                        };

                        let content = Rc::new(content);
                        dbg!(&content);
                        let new_ctx = {
                            let mut new_ctx = ctx.deref().clone();
                            new_ctx.set_current_filename(fname);
                            Rc::new(new_ctx)
                        };

                        let listing = dbg!(parse_z80_strrc_with_contextrc(content, new_ctx))?;
                        cell.replace(Some(listing));
                        assert!(cell.borrow().is_some());
                        dbg!(self);
                    }
                }
            }

            LocatedToken::Standard {
                token:
                    Token::Incbin {
                        fname,
                        offset,
                        length,
                        extended_offset: _,
                        off: _,
                        ref content,
                        transformation,
                    },
                span,
            } if content.borrow().is_none() => {
                //TODO manage the optional arguments
                match ctx.get_path_for(&fname) {
                    Err(_e) => {
                        return Err(AssemblerError::IOError {
                            msg: format!("{:?} not found", fname),
                        });
                    }
                    Ok(ref fname) => {
                        let mut f = File::open(&fname).map_err(|_e| AssemblerError::IOError {
                            msg: format!("Unable to open {:?}", fname),
                        })?;

                        use std::io::{Seek, SeekFrom};
                        if offset.is_some() {
                            f.seek(SeekFrom::Start(offset.as_ref().unwrap().eval()? as _));
                            // TODO use the symbol table for that
                        }

                        let mut data = Vec::new();

                        if length.is_some() {
                            let mut f = f.take(length.as_ref().unwrap().eval()? as _);
                            f.read_to_end(&mut data)
                                .map_err(|e| AssemblerError::IOError {
                                    msg: format!("Unable to read {:?}. {}", fname, e),
                                })?;
                        } else {
                            f.read_to_end(&mut data)
                                .map_err(|e| AssemblerError::IOError {
                                    msg: format!("Unable to read {:?}. {}", fname, e.to_string()),
                                })?;
                        };

                        match transformation {
                            BinaryTransformation::None => {
                                content.replace(data.into());
                            }

                            BinaryTransformation::Exomizer => {
                                unimplemented!("Need to implement exomizer crunching")
                            }

                            BinaryTransformation::Lz49 => {
                                if data.len() == 0 {
                                    return Err(AssemblerError::EmptyBinaryFile(
                                        fname.to_string_lossy().to_string(),
                                    ));
                                }

                                let crunched = crate::crunchers::lz49::lz49_encode_legacy(&data);
                                content.replace(crunched.into());
                            }

                            BinaryTransformation::Aplib => {
                                if data.len() == 0 {
                                    return Err(AssemblerError::EmptyBinaryFile(
                                        fname.to_string_lossy().to_string(),
                                    ));
                                }

                                let crunched = crate::crunchers::apultra::compress(&data);
                                content.replace(crunched.into());
                            }
                        }
                    }
                }
            }

            // Rorg may embed some instructions that read files
            LocatedToken::Rorg(_, ref listing, _) => {
                for token in listing.iter() {
                    token.read_referenced_file(ctx)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn fix_local_macro_labels_with_seed(&mut self, seed: usize) {
        match self {
            LocatedToken::Standard { token, span } => {
                token.fix_local_macro_labels_with_seed(seed)
            },
            LocatedToken::CrunchedSection(_, _, _) => todo!(),
            LocatedToken::Include(_, _, _) => todo!(),
           
            Self::If(v, o, _) => {
                v.iter_mut()
                    .map(|(t, l)| l)
                    .for_each(|l| l.fix_local_macro_labels_with_seed(seed));
                o.as_mut().map(|l| l.fix_local_macro_labels_with_seed(seed));
            }

            Self::Switch(l, _) => {
                l.iter_mut().for_each(|(e, l)| {
                    e.fix_local_macro_labels_with_seed(seed);
                    l.fix_local_macro_labels_with_seed(seed);
                });
            }



            Self::Repeat(e, l, _, _)
            | Self::RepeatUntil(e, l, _)
            | Self::Rorg(e, l, _)
            | Self::While(e, l, _) => {
                e.fix_local_macro_labels_with_seed(seed);
                l.fix_local_macro_labels_with_seed(seed);
            }
        }
    }
}
/// Implement this trait for type previousy defined without source location.

pub trait Locate {
    type Output;

    fn locate(self, span: Z80Span) -> Self::Output;
}

impl Locate for Token {
    type Output = LocatedToken;

    fn locate(self, span: Z80Span) -> LocatedToken {
        if self.has_at_least_one_listing() {
            /*/
            match self {
                Token::CrunchedSection(a, b) => {
                    LocatedToken::CrunchedSection(a, b, span)
                },
                Token::Include(a,b) => {
                    LocatedToken::Include(a, b, span)
                },
                Token::If(a, b) => {
                    LocatedToken::If(a, b, span)
                },
                Token::Repeat(a,b, c,) => {
                    LocatedToken::Repeat(a,b,c,span)
                },
                Token::RepeatUntil(a, b) => {
                    LocatedToken::RepeatUntil(a, b, span)
                },
                Token::Rorg(a, b) => {
                    LocatedToken::Rorg(a, b, span)
                },
                Token::Switch(a) => {
                    LocatedToken::Switch(a, span)
                },
                Token::While(a, b) => {
                    LocatedToken::While(a, b, span)
                },
                _ => unreachable!()

            }*/
            unreachable!()
        } else {
            LocatedToken::Standard { token: self, span }
        }
    }
}

impl ListingElement for LocatedToken {}

pub type InnerLocatedListing = BaseListing<LocatedToken>;

/// Represents a Listing of located tokens
/// Lifetimes 'src and 'ctx are in fact the same and correspond to hte lifetime of the object itself
#[derive(Clone, Debug)]
pub struct LocatedListing {
    /// The real listing
    listing: InnerLocatedListing,
    /// Its source code
    src: Rc<String>,
    /// Its Parsing Context
    ctx: Rc<ParserContext>,
}

impl LocatedListing {
    /// Create an empty listing. Code as to be parsed afterwhise
    pub fn new_empty(str: String, ctx: ParserContext) -> Self {
        Self {
            listing: Default::default(),
            src: Rc::new(str),
            ctx: Rc::new(ctx),
        }
    }

    pub fn new_empty_span(span: Z80Span) -> Self {
        Self {
            listing: Default::default(),
            src: Rc::clone(&span.extra.0),
            ctx: Rc::clone(&span.extra.1),
        }
    }

    pub fn src(&self) -> &Rc<String> {
        &self.src
    }

    pub fn ctx(&self) -> &Rc<ParserContext> {
        &self.ctx
    }

    pub fn span(&self) -> Z80Span {
        Z80Span::new_extra_from_rc(Rc::clone(&self.src), Rc::clone(&self.ctx))
    }

    pub fn fix_local_macro_labels_with_seed(&mut self, seed: usize) {
        self.iter_mut()
            .for_each(|e| e.fix_local_macro_labels_with_seed(seed));
    }
}

impl Deref for LocatedListing {
    type Target = InnerLocatedListing;
    fn deref(&self) -> &Self::Target {
        &self.listing
    }
}
impl DerefMut for LocatedListing {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.listing
    }
}

impl TryFrom<Vec<LocatedToken>> for LocatedListing {
    type Error = ();

    /// Conversion fails only when the vec is empty.
    /// In this case a workaround has to be used
    fn try_from(tokens: Vec<LocatedToken>) -> Result<Self, Self::Error> {
        match tokens.first() {
            Some(token) => {
                let extra = &token.span().extra;
                let src = Rc::clone(&extra.0);
                let ctx = Rc::clone(&extra.1);
                Ok(LocatedListing {
                    listing: tokens.into(),
                    ctx,
                    src,
                })
            }
            None => Err(()),
        }
    }
}

impl LocatedListing {
    pub fn as_listing(&self) -> Listing {
        self.deref()
            .iter()
            .map(|lt| lt.as_token())
            .collect_vec()
            .into()
    }
}

pub trait ParseToken {
    type Output: ListingElement;
    fn parse_token(src: &str) -> Result<Self::Output, String>;
}

impl ParseToken for Token {
    type Output = Token;
    fn parse_token(src: &str) -> Result<Self::Output, String> {
        let token = LocatedToken::parse_token(src);
        token.map(|lt| lt.as_token())
    }
}