use std::{borrow::Cow, clone, convert::TryFrom, fs::File, io::Read, ops::{Deref, DerefMut}, sync::{Mutex, RwLock}};
use std::sync::Arc;
use cpclib_common::itertools::Itertools;
use cpclib_common::rayon::prelude::*;
use cpclib_disc::amsdos::AmsdosHeader;
use cpclib_tokens::{
    BaseListing, BinaryTransformation, CrunchType, Expr, Listing, ListingElement, TestKind, Token,
};

use crate::{
    error::AssemblerError,
    implementation::expression::ExprEvaluationExt,
    preamble::{parse_z80_str, parse_z80_strrc_with_contextrc},
};

use super::{ParserContext, Z80Span};
use crate::implementation::instructions::Cruncher;

///! This crate is related to the adaptation of tokens and listing for the case where they are parsed

#[derive(Debug)]
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
    Include(
        String,
        RwLock<Option<LocatedListing>>,
        Option<String>,
        Z80Span,
    ),
    If(
        Vec<(TestKind, LocatedListing)>,
        Option<LocatedListing>,
        Z80Span,
    ),
    Repeat(Expr, LocatedListing, Option<String>, Option<Expr>, Z80Span),
    Iterate(String, Vec<Expr>, LocatedListing, Z80Span),
    RepeatUntil(Expr, LocatedListing, Z80Span),
    Rorg(Expr, LocatedListing, Z80Span),
    Switch(Vec<(Expr, LocatedListing)>, Z80Span),
    While(Expr, LocatedListing, Z80Span),
    Module(String, LocatedListing, Z80Span),
}

impl Clone for LocatedToken {
    fn clone(&self) -> Self {
        match self {
            LocatedToken::Standard { token, span } => LocatedToken::Standard { token: token.clone(), span: span.clone()},
            LocatedToken::CrunchedSection(a, b, c) => LocatedToken::CrunchedSection(a.clone(), b.clone(), c.clone()),
            LocatedToken::Include(filename, listing, namespace, span) => {
                Self::Include(
                    filename.clone(),
                    RwLock::new(listing.read().unwrap().clone()),
                    namespace.clone(),
                    span.clone()
                )
            },
            LocatedToken::If(a, b, c) => LocatedToken::If(a.clone(), b.clone(), c.clone()) ,
            LocatedToken::Repeat(a, b, c, d, e) => LocatedToken::Repeat(a.clone(), b.clone(), c.clone(), d.clone(), e.clone()) ,
            LocatedToken::Iterate(_, _, _, _) => todo!(),
            LocatedToken::RepeatUntil(_, _, _) => todo!(),
            LocatedToken::Rorg(_, _, _) => todo!(),
            LocatedToken::Switch(_, _) => todo!(),
            LocatedToken::While(_, _, _) => todo!(),
            LocatedToken::Module(_, _, _) => todo!(),
        }
    }
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
            | Self::Include(_, _, _, span)
            | Self::If(_, _, span)
            | Self::Module(_, _, span)
            | Self::Iterate(_, _, _, span)
            | Self::Repeat(_, _, _, _, span)
            | Self::RepeatUntil(_, _, span)
            | Self::Rorg(_, _, span)
            | Self::Switch(_, span)
            | Self::While(_, _, span) => span,
        }
    }

    pub fn context(&self) -> &(Arc<String>, Arc<ParserContext>) {
        &self.span().extra
    }
}

impl LocatedToken {
    pub fn as_token(&self) -> Cow<Token> {
        match self {
            LocatedToken::Standard { token, .. } => Cow::Borrowed(token),
            LocatedToken::CrunchedSection(c, l, _span) => {
                Cow::Owned(Token::CrunchedSection(*c, l.as_listing()))
            },
            LocatedToken::Include(s, l, module, _span) => Cow::Owned(Token::Include(
                s.clone(),
                l.read().unwrap().as_ref().map(|l| l.as_listing()).into(),
                module.clone(),
            )),
            LocatedToken::If(v, e, _span) => Cow::Owned(Token::If(
                v.iter()
                    .map(|(k, l)| (k.clone(), l.as_listing()))
                    .collect_vec(),
                e.as_ref().map(|l| l.as_listing()),
            )),
            LocatedToken::Repeat(e, l, s, start, _span) => {
                Cow::Owned(Token::Repeat(e.clone(), l.as_listing(), s.clone(), start.clone()))
            }
            LocatedToken::RepeatUntil(e, l, _span) => Cow::Owned(Token::RepeatUntil(e.clone(), l.as_listing())),
            LocatedToken::Rorg(e, l, _span) => Cow::Owned(Token::Rorg(e.clone(), l.as_listing())),
            LocatedToken::Switch(v, _span) => Cow::Owned(Token::Switch(
                v.iter()
                    .map(|(e, l)| (e.clone(), l.as_listing()))
                    .collect_vec(),
            )),
            LocatedToken::While(e, l, _span) => Cow::Owned(Token::While(e.clone(), l.as_listing())),
            LocatedToken::Iterate(name, values, code, _span) => todo!(),
            LocatedToken::Module(_, _, _) => todo!(),
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
        match self {
            LocatedToken::Include(ref fname, ref cell, _namespace, span) => {
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

                        let content = Arc::new(content);
                        let new_ctx = {
                            let mut new_ctx = ctx.deref().clone();
                            new_ctx.set_current_filename(fname);
                            Arc::new(new_ctx)
                        };

                        let listing = parse_z80_strrc_with_contextrc(content, new_ctx)?;
                        cell.write().unwrap().replace(listing);
                        assert!(cell.read().unwrap().is_some());
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
            } if content.read().unwrap().is_none() => {
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

                        // load the full file
                        let mut data = Vec::new();
                        f.read_to_end(&mut data)
                            .map_err(|e| AssemblerError::IOError {
                                msg: format!("Unable to read {:?}. {}", fname, e.to_string()),
                            })?;

                        // get a slice on the data to ease its cut
                        let mut data = &data[..];

                        if data.len() >= 128 {
                            let header = AmsdosHeader::from_buffer(&data);
                            let info = if header.is_checksum_valid() {
                                data = &data[128..];

                                AssemblerError::RelocatedInfo{
                                    info: Box::new(
                                        AssemblerError::AssemblingError{
                                            msg: format!("{:?} is a valid Amsdos file. It is included without its header.", fname)
                                        }
                                    ),
                                    span: span.clone()
                                }
                            } else {
                                AssemblerError::RelocatedInfo{
                                    info: Box::new(
                                        AssemblerError::AssemblingError{
                                            msg: format!("{:?} does not contain a valid Amsdos file. It is fully included.", fname)
                                        }
                                    ),
                                    span: span.clone()
                                }
                            };

                            eprintln!("{}", info);
                        }

                        if offset.is_some() {
                            let offset = offset.as_ref().unwrap().eval()?.int() as usize;
                            if offset >= data.len() {
                                return Err(AssemblerError::AssemblingError {
                                    msg: format!(
                                        "Unable to read {:?}. Only {} are available",
                                        fname,
                                        data.len()
                                    ),
                                });
                            }
                            data = &data[offset..];
                        }

                        if length.is_some() {
                            let length = length.as_ref().unwrap().eval()?.int() as usize;
                            data = &data[..length];
                            if data.len() != length {
                                return Err(AssemblerError::AssemblingError {
                                    msg: format!(
                                        "Unable to read {:?}. Only {} are available",
                                        fname,
                                        data.len()
                                    ),
                                });
                            }
                        }

                        match transformation {
                            BinaryTransformation::None => {
                            content.write().unwrap().replace(data.to_vec());
                            }

                            other => {
                                if data.len() == 0 {
                                    return Err(AssemblerError::EmptyBinaryFile(
                                        fname.to_string_lossy().to_string(),
                                    ));
                                }

                                let crunch_type = other.crunch_type().unwrap();
                                let crunched = crunch_type.crunch(&data)?;
                                content.write().unwrap().replace(crunched.into());
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

    /*
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


            Self::RepeatUntil(e, l, _)
            | Self::Rorg(e, l, _)
            | Self::While(e, l, _) => {
                e.fix_local_macro_labels_with_seed(seed);
                l.fix_local_macro_labels_with_seed(seed);
            }

            Self::Repeat(e, l, _, s, _) => {

                e.fix_local_macro_labels_with_seed(seed);
                l.fix_local_macro_labels_with_seed(seed);
                s.as_mut().map(|s| s.fix_local_macro_labels_with_seed(seed));
            }
        }
    }
    */
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
    src: Arc<String>,
    /// Its Parsing Context
    ctx: Arc<ParserContext>,
}


impl LocatedListing {
    /// Create an empty listing. Code as to be parsed afterwhise
    pub fn new_empty(str: String, ctx: ParserContext) -> Self {
        Self {
            listing: Default::default(),
            src: Arc::new(str),
            ctx: Arc::new(ctx),
        }
    }

    pub fn new_empty_span(span: Z80Span) -> Self {
        Self {
            listing: Default::default(),
            src: Arc::clone(&span.extra.0),
            ctx: Arc::clone(&span.extra.1),
        }
    }

    pub fn src(&self) -> &Arc<String> {
        &self.src
    }

    pub fn ctx(&self) -> &Arc<ParserContext> {
        &self.ctx
    }

    pub fn span(&self) -> Z80Span {
        Z80Span::new_extra_from_rc(Arc::clone(&self.src), Arc::clone(&self.ctx))
    }

    /*
    pub fn fix_local_macro_labels_with_seed(&mut self, seed: usize) {
        self.iter_mut()
            .for_each(|e| e.fix_local_macro_labels_with_seed(seed));
    }
    */
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
                let src = Arc::clone(&extra.0);
                let ctx = Arc::clone(&extra.1);
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
    pub fn as_cowed_listing(&self) -> BaseListing<Cow<Token>> {
        self.deref()
            .par_iter()
            .map(|lt| lt.as_token())
            .collect::<Vec<_>>()
            .into()
    }

    pub fn as_listing(&self) -> BaseListing<Token> {
        self.deref()
            .par_iter()
            .map(|lt| lt.as_token())
            .map(|c| -> Token { c.into_owned()})
            .collect::<Vec<Token>>()
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
        token.map(|lt| lt.as_token().into_owned())
    }
}


