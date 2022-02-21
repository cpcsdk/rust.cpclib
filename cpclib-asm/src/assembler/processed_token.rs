use crate::Visited;
use crate::ParserContext;
use crate::AssemblerError; 
use crate::parse_z80_strrc_with_contextrc;
use crate::LocatedToken;
use crate::Env;
use crate::preamble::LocatedListing;

use cpclib_tokens::ListingElement;
use cpclib_tokens::Token;
use cpclib_tokens::BinaryTransformation;
use crate::implementation::instructions::Cruncher;
use cpclib_disc::amsdos::AmsdosHeader;

use std::any::Any;
use std::borrow::BorrowMut;
use std::fmt::write;
use std::io::Read;
use std::fs::File;
use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;
use std::ops::Deref;
use std::fmt::Debug;
use std::fmt::Formatter;

use cpclib_common::itertools::Itertools;

use ouroboros::*;

/// Tokens are read only elements extracted from the parser
/// ProcessedTokens allow to maintain their state during assembling
#[derive(Debug)]
pub struct ProcessedToken<'token, T: Visited + AsSimpleToken + Debug> {
	/// The token being processed by the assembler
	token: &'token T,
	state: Option<ProcessedTokenState>
}

/// Specific state to maintain for the current token
#[derive(Debug)]
enum ProcessedTokenState {
	/// A state is expected but has not been yet specified
	Expected,
	/// Included file must read at some moment the file to handle
	Include(IncludeState),
	/// Included binary needs to be read
	/// TODO add parameters
	Incbin{data: Vec<u8>}
}

#[self_referencing]
struct IncludeState {
    listing: LocatedListing,
    #[borrows(listing)]
    #[covariant]
    processed_tokens: Vec<ProcessedToken<'this, LocatedToken>>
}

impl Debug for IncludeState {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> { 
        write!(fmt, "IncludeState")
     }
}

pub trait AsSimpleToken {
	/// Convert the token in its simplest form
	fn as_simple_token(&self) -> Cow<Token>;
}

impl AsSimpleToken for Token {
	fn as_simple_token(&self) -> Cow<Token> {
		Cow::Borrowed(self)
	}
}

impl AsSimpleToken for LocatedToken {
	fn as_simple_token(&self) -> Cow<Token> {
		self.as_token()
	}
}

/* There is a bug in rust compiler that forbids to use that :(
default impl<'token, T:AsSimpleToken + Visited + Debug>  From<&'token T> for ProcessedToken<'token, T> {
	fn from(token: &'token T) -> Self {
		let state = match token.as_simple_token().as_ref() {
			Token::Include(..) | Token::Incbin{..} => Some(ProcessedTokenState::Expected),
			_ => None
		};

		ProcessedToken{
			token,
			state
		}
	}
	}
}
*/


 impl From<&'token Token> for ProcessedToken<'token, Token> {
	fn from(token: &'token Token) -> Self {
		let state = match token {
			Token::Include(..) | Token::Incbin{..} => Some(ProcessedTokenState::Expected),
			_ => None
		};

		ProcessedToken{
			token,
			state
		}
	}
}

// Explicit version in LocatedToken to not convert it in Token which is a lost of time
impl From<&'token LocatedToken> for ProcessedToken<'token, LocatedToken> {
	fn from(token: &'token LocatedToken) -> Self {
		let state = match token {
			LocatedToken::Standard{
                token: Token::Include(..) | Token::Incbin{..},
                ..
            } => Some(ProcessedTokenState::Expected),
			_ => None
		};

		ProcessedToken{
			token,
			state
		}
	}
}


pub type AssemblerInfo = AssemblerError;



pub fn build_list<'token, T:'static + AsSimpleToken + Visited + Debug> (tokens: &'token[T], env: &Env) -> Vec<ProcessedToken<'token, T>> {


    tokens.iter()
    .map(|t| {
        // ugly workaround of a rust compiler bug that forbids to play with ProcessedToken::from(t)

        match  (t as &'token dyn Any).downcast_ref::<LocatedToken>() {
            Some(t) => {
                let t: &'token LocatedToken = unsafe{std::mem::transmute(t)};
                let t = ProcessedToken::from(t);
                let t: ProcessedToken<'token, T> =  unsafe{std::mem::transmute(t)}; // totally safe as we a transmuting from one type to the strictly same one
                return t;
            }
            None => {},
        }

        match  (t as &'token dyn Any).downcast_ref::<Token>() {
            Some(t) => {
                let t: &'token Token = unsafe{std::mem::transmute(t)};
                let t = ProcessedToken::from(t);
                let t: ProcessedToken<'token, T> =  unsafe{std::mem::transmute(t)}; // totally safe as we a transmuting from one type to the strictly same one
                return t;
            }
            None => panic!("Unhandled type..."),
        }

    })
    .map(|mut t| {
        t.read_referenced_file(&env); 
        t
    }) // Read its files but ignore errors if any (which must happen a lot for incbin)
    .collect_vec()

}

/// Visit all the tokens until an error occurs
pub fn visit_processed_tokens<'token, T:AsSimpleToken + Visited + Debug>(tokens: & mut [ProcessedToken<'token, T>], env: &mut Env) -> Result<(), AssemblerError> {
    for token in tokens.iter_mut() {
        token.visited(env)?;
    }

    Ok(())
}


impl<'token, T:AsSimpleToken + Visited + Debug> ProcessedToken<'token, T> {


	/// Read the data for the appropriate tokens.
	/// Possibly returns information to be printed by the caller
    pub fn read_referenced_file(&mut self, env: &Env) -> Result<Option<AssemblerInfo>, AssemblerError> {


		match self.state {
			Some(ProcessedTokenState::Expected) => {/* need to read the ressource */},
			Some(ProcessedTokenState::Include{..}) => {return Ok(None)},
			Some(ProcessedTokenState::Incbin{..}) => { /* TODO check if paramters changed */ return Ok(None)}
			None => {return Ok(None)}
		};

		let ctx = &env.ctx;

		// whatever is the representation of the token, returns it simple version
		let token: Cow<Token> = self.token.as_simple_token();
		let token = token.as_ref();

		let mut info = None;

        self.state = match token {
            Token::Include(ref fname, _namespace, _once) => {
                let content = read_source(fname, ctx)?;

                let content = Arc::new(content);
                let new_ctx = {
                    let mut new_ctx = ctx.deref().clone();
                    new_ctx.set_current_filename(fname);
                    Arc::new(new_ctx)
                };

                let listing = parse_z80_strrc_with_contextrc(content, new_ctx)?;
                let includeState = IncludeStateBuilder {
                    listing,
                    processed_tokens_builder: |listing: &LocatedListing| build_list(listing.as_slice(), env)
                }.build();
				Some(ProcessedTokenState::Include(includeState))
            }

            Token::Incbin {
                        fname,
                        offset,
                        length,
                        extended_offset: _,
                        off: _,
                        transformation
            } => {
                // TODO manage the optional arguments
                match ctx.get_path_for(&fname) {
                    Err(_e) => {
                        return Err(AssemblerError::IOError {
                            msg: format!("{:?} not found", fname)
                        });
                    }
                    Ok(ref fname) => {
                        let mut f = File::open(&fname).map_err(|_e| {
                            AssemblerError::IOError {
                                msg: format!("Unable to open {:?}", fname)
                            }
                        })?;

                        // load the full file
                        let mut data = Vec::new();
                        f.read_to_end(&mut data).map_err(|e| {
                            AssemblerError::IOError {
                                msg: format!("Unable to read {:?}. {}", fname, e.to_string())
                            }
                        })?;

                        // get a slice on the data to ease its cut
                        let mut data = &data[..];

                        if data.len() >= 128 {
                            let header = AmsdosHeader::from_buffer(&data);
                            let info = Some(if header.is_checksum_valid() {
                                data = &data[128..];

                                AssemblerError::AssemblingError{
									msg: format!("{:?} is a valid Amsdos file. It is included without its header.", fname)
								}
                            }
                            else {
                               AssemblerError::AssemblingError{
                                            msg: format!("{:?} does not contain a valid Amsdos file. It is fully included.", fname)
                                        }
							});

                        }

						match offset {
							Some(ref offset) => {
								let offset = env.resolve_expr_must_never_fail(offset)?.int()? as usize;
								if offset >= data.len() {
									return Err(AssemblerError::AssemblingError {
										msg: format!(
											"Unable to read {:?}. Only {} are available",
											fname,
											data.len()
										)
									});
								}
								data = &data[offset..];
							},
							None => {}
						}

                       
                        match length {
							Some(length) => {
								let length = env.resolve_expr_must_never_fail(length)?.int()? as usize;
								if data.len() < length {
									return Err(AssemblerError::AssemblingError {
										msg: format!(
											"Unable to read {:?}. Only {} bytes are available ({} expected)",
											fname,
											data.len(),
											length
										)
									});
								}
								data = &data[..length];
							},
							None => {}
						}

                        let data = match transformation {
                            BinaryTransformation::None => {
                                data.to_vec()
                            }

                            other => {
                                if data.len() == 0 {
                                    return Err(AssemblerError::EmptyBinaryFile(
                                        fname.to_string_lossy().to_string()
                                    ));
                                }

                                let crunch_type = other.crunch_type().unwrap();
                                crunch_type.crunch(&data)?
                            }
                        };
						Some(ProcessedTokenState::Incbin{data})
                    }
                }
            },
/*
            // Rorg may embed some instructions that read files
            LocatedToken::Rorg(_, ref listing, _) => {
                for token in listing.iter() {
                    token.read_referenced_file(ctx)?;
                }
            }
*/
            _ => {unreachable!()}
        };

        Ok(info)
    }
}


/// Read the content of the source file.
/// Uses the context to obtain the appropriate file other the included directories
pub fn read_source(fname: &str, ctx: &ParserContext) -> Result<String, AssemblerError> {
    match ctx.get_path_for(fname) {
        Err(e) => {
            Err(AssemblerError::IOError {
                msg: format!("{:?} not found. {:?}", fname, e)
            })
        }
        Ok(ref fname) => {
            let mut f = File::open(&fname).map_err(|e| {
                AssemblerError::IOError {
                    msg: format!("Unable to open {:?}. {}", fname, e)
                }
            })?;

            let mut content = Vec::new();
            f.read_to_end(&mut content).map_err(|e| {
                AssemblerError::IOError {
                    msg: format!("Unable to read {:?}. {}", fname, e.to_string())
                }
            })?;

            let result = chardet::detect(&content);
            let coder =
                encoding::label::encoding_from_whatwg_label(chardet::charset2encoding(&result.0));

            let content = match coder {
                Some(coder) => {
                    let utf8reader = coder
                        .decode(&content, encoding::DecoderTrap::Ignore)
                        .expect("Error");
                    utf8reader.to_string()
                }
                None => {
                    return Err(AssemblerError::IOError {
                        msg: format!("Encoding error for {:?}.", fname)
                    });
                }
            };

            Ok(content)
        }
    }
}

impl<'token, T: Visited + AsSimpleToken + Debug>  ProcessedToken<'token, T> {
	/// Due to the state management, the signature requires mutability
	pub fn visited(&mut self, env: &mut Env) -> Result<(), AssemblerError> {

		if let Some(_) = &self.state
		{
			self.read_referenced_file(env)?;
		}	

		match &mut self.state {
			None => self.token.visited(env),
            Some(ProcessedTokenState::Include(ref mut state)) => {
                match self.token.as_simple_token().as_ref() {
                    Token::Include(fname, namespace, once) => {
                        let fname = env
                            .ctx // TODO get span context if available
                            .get_path_for(fname)
                            .unwrap_or("will_fail".into());
                        if (!*once) || (!env.has_included(&fname)) {
                            // inclusion requested
                            env.mark_included(fname);

                            // handle module if necessary
                            if let Some(namespace) = namespace {
                                env.enter_namespace(namespace)?;
                                // TODO handle the locating of error
                                    //.map_err(|e| e.locate(span.clone()))?;
                            }
                            
                            // Visit the included listing
                            state.with_processed_tokens_mut(|tokens| {
                                visit_processed_tokens(tokens, env)
                            })?;

                            // Remove module if necessary
                            if namespace.is_some() {
                                env.leave_namespace()?;
                                //.map_err(|e| e.locate(span.clone()))?;
                            }

                            Ok(())

                        } else {
                            // no inclusion
                            Ok(())
                        }
                    },
                    _ => unreachable!()
                }
            },

            Some(ProcessedTokenState::Incbin{ref data}) => {
                env.visit_incbin(data)
            }
			other => unimplemented!("Specific behavior requiring a state not implemented. {:?}", other)
		}
	}
}

		            /*
            
            let fname = span
                .context()
                .get_path_for(fname)
                .unwrap_or("will_fail".into());
            if (!*once) || (!env.has_included(&fname)) {
                env.mark_included(fname);

                if cell.read().unwrap().is_some() {
                    if let Some(namespace) = namespace {
                        env.enter_namespace(namespace)
                            .map_err(|e| e.locate(span.clone()))?;
                    }

                    env.visit_listing(cell.read().unwrap().as_ref().unwrap())?;

                    if namespace.is_some() {
                        env.leave_namespace().map_err(|e| e.locate(span.clone()))?;
                    }
                    Ok(())
                }
                else {
                    outer_token
                        .read_referenced_file(&outer_token.context().1)
                        .and_then(|_| visit_located_token(outer_token, env))
                        .map_err(|e| e.locate(span.clone()))
                }
                .map_err(|err| {
                    AssemblerError::IncludedFileError {
                        span: span.clone(),
                        error: Box::new(err)
                    }
                })
            }
            else {
                Ok(()) // we include nothing
            }
            */
