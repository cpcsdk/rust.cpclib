use crate::Visited;
use crate::ParserContext;
use crate::AssemblerError; 
use crate::LocatedToken;
use crate::Env;
use crate::preamble::LocatedListing;
use crate::preamble::parse_z80_str_with_context;

use cpclib_tokens::ListingElement;
use cpclib_tokens::TestKindElement;
use cpclib_tokens::ToSimpleToken;
use cpclib_tokens::Token;
use cpclib_tokens::BinaryTransformation;
use cpclib_tokens::symbols::SymbolsTableTrait;
use crate::implementation::instructions::Cruncher;
use cpclib_disc::amsdos::AmsdosHeader;

use std::any::Any;
use std::collections::HashMap;
use std::io::Read;
use std::fs::File;
use std::borrow::Cow;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::ops::Deref;


use ouroboros::*;

/// Tokens are read only elements extracted from the parser
/// ProcessedTokens allow to maintain their state during assembling
#[derive(Debug, Clone)]
pub struct ProcessedToken<'token, T: Visited + ToSimpleToken + Debug + ListingElement + Sync> {
	/// The token being processed by the assembler
	token: &'token T,
	state: Option<ProcessedTokenState<'token, T>>
}

/// Specific state to maintain for the current token
#[derive(Debug, Clone)]
enum ProcessedTokenState<'token, T:  Visited + ToSimpleToken + ListingElement + Debug + Sync> {
	/// A state is expected but has not been yet specified
	Expected,
    /// If state encodes previous choice
    If(IfState<'token, T>),
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

impl Clone for IncludeState {
    fn clone(&self) -> Self {
        todo!()
    }
}

impl Debug for IncludeState {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> { 
        write!(fmt, "IncludeState")
     }
}

/// Store for each branch (if passed at some point) the test result and the listing
#[derive(Debug, Clone)]
struct IfState<'token, T: Visited + ToSimpleToken + Debug + ListingElement + Sync>{
    // The token that contains the tests and listings
    token: &'token T,
    if_token_adr_to_used_decision: std::collections::HashMap<usize, bool>,
    if_token_adr_to_unused_decision: std::collections::HashMap<usize, bool>,
    // Processed listing build on demand
    tests_listing: HashMap<usize, Vec<ProcessedToken<'token, T>>>,
    // else listing build on demand
    else_listing: Option<Vec<ProcessedToken<'token, T>>>
}


impl<'token, T: Visited + ToSimpleToken + Debug + ListingElement + Sync> IfState<'token, T> {
    fn new(token: &'token T) -> Self {
        Self {
            token,
            if_token_adr_to_used_decision: Default::default(),
            if_token_adr_to_unused_decision: Default::default(),
            tests_listing: Default::default(),
            else_listing: None
        }
    }


    fn choose_listing_to_assemble(&mut self, env: &Env) -> Result< Option<&mut [ProcessedToken<'token, T>]>, AssemblerError> {

        let mut selected_idx = None;
        let mut request_additional_pass = false;

        for idx in 0..self.token.if_nb_tests() {
            dbg!(idx);
            let (test, _) = self.token.if_test(idx);
            let token_adr = test as *const _ as usize;

            // Expression must be true
            if test.is_true_test() {
                let exp = test.expr_unchecked();
                // Expression must be true
                let value = env.resolve_expr_must_never_fail(exp)?;
                if value.bool()? {
                    selected_idx = Some(idx);
                    break
                }
            
            }

            // Expression must be false
            else if test.is_false_test()  {
                let exp = test.expr_unchecked();
                let value = env.resolve_expr_must_never_fail(exp)?;
                if !value.bool()? {
                    selected_idx = Some(idx);
                    break
                }
            }

            else if test.is_label_used_test() {
                    let label = test.label_unchecked();
                    let decision = env.symbols().is_used(label);

                    // Add an extra pass if the test differ
                    if let Some(res) = self.if_token_adr_to_used_decision.get(&token_adr) {
                        if *res != decision {
                            request_additional_pass = true;
                        }
                    }

                    // replace the previously stored value
                    self.if_token_adr_to_used_decision
                        .insert(token_adr.clone(), decision);

                    if decision {
                        selected_idx = Some(idx);
                        break
                    }
                }

                else if test.is_label_nused_test() {
                    let label = test.label_unchecked();
                    let decision = !env.symbols().is_used(label);

                    // Add an extra pass if the test differ
                    if let Some(res) = self.if_token_adr_to_unused_decision.get(&token_adr) {
                        if *res != decision {
                            request_additional_pass = true;
                        }
                    }

                    // replace the previously stored value
                    self.if_token_adr_to_unused_decision
                        .insert(token_adr.clone(), decision);

                    if decision {
                        selected_idx = Some(idx);
                        break
                    }
                }

                // Label must exist
            else if test.is_label_exists_test() {
                    let label = test.label_unchecked();
                    if env.symbols().symbol_exist_in_current_pass(label)? {
                        selected_idx = Some(idx);
                        break
                    }
                }

                // Label must not exist
            else {
                    let label = test.label_unchecked();
                    if !env.symbols().symbol_exist_in_current_pass(label)? {
                        selected_idx = Some(idx);
                        break;
                    }
                }
        }

        dbg!(&selected_idx);

        let selected_listing = match selected_idx {
            Some(selected_idx) => {
                // build the listing if never done
                if self.tests_listing.get(&selected_idx).is_none() {
                    let listing = self.token.if_test(selected_idx).1;
                    let listing = build_list(listing, env);
                    self.tests_listing.insert(selected_idx, listing);
                }
                self.tests_listing.get_mut(&selected_idx)
            },
            None => {
                // build else listing if needed
                if self.else_listing.is_none() && self.token.if_else().is_some() {
                    let listing = self.token.if_else();
                    self.else_listing  = listing.map(|listing| build_list(listing, env));
                }
                self.else_listing.as_mut()
            }
        };

        // update env to request an additional pass
        let request_additional_pass = *env.request_additional_pass.read().unwrap().deref()
            | request_additional_pass;
        *env.request_additional_pass.write().unwrap() =request_additional_pass;



        Ok(
            selected_listing.map(|l| l.as_mut_slice())
        )

    }
} 

impl<'token, T: Visited + ToSimpleToken + Debug + ListingElement + Sync> ToSimpleToken for ProcessedToken<'token, T> {
	fn as_simple_token(&self) -> Cow<Token> {
		self.token.as_simple_token()
	}
}

pub type AssemblerInfo = AssemblerError;


pub fn build_processed_token<'token, T: ToSimpleToken + Visited + Debug + Sync + ListingElement> (token: &'token T, env: &Env) -> ProcessedToken<'token, T> {

    if token.is_if() {
        let state = IfState::new(token);
        ProcessedToken {
            state: Some(ProcessedTokenState::If(state)),
            token
        }
    }
    else if token.is_include() || token.is_incbin() {
        ProcessedToken {
            state: Some(ProcessedTokenState::Expected),
            token
        }
    }
    else {
        ProcessedToken {
            state: None,
            token
        }
    }

}


pub fn build_list<'token, T: ToSimpleToken + Visited + Debug + Sync + ListingElement> (tokens: &'token[T], env: &Env) -> Vec<ProcessedToken<'token, T>> {
    use rayon::prelude::*;

    tokens
    .par_iter()
    .map(|t| {
        build_processed_token(t, env)
    })
    .map(|mut t| {
        t.read_referenced_file(&env); 
        t
    }) // Read its files but ignore errors if any (which must happen a lot for incbin)
    .collect::<Vec::<_>>()

}

/// Visit all the tokens until an error occurs
pub fn visit_processed_tokens<'token, T:ToSimpleToken + Visited + Debug +ListingElement + Sync>(tokens: & mut [ProcessedToken<'token, T>], env: &mut Env) -> Result<(), AssemblerError> {
    for token in tokens.iter_mut() {
        token.visited(env)?;
    }

    Ok(())
}


impl<'token, T:ToSimpleToken + Visited + Debug + ListingElement + Sync> ProcessedToken<'token, T> {


	/// Read the data for the appropriate tokens.
	/// Possibly returns information to be printed by the caller
    pub fn read_referenced_file(&mut self, env: &Env) -> Result<Option<AssemblerInfo>, AssemblerError> {


		match self.state {
			Some(ProcessedTokenState::Expected) => {/* need to read the ressource */},
			Some(ProcessedTokenState::Include{..}) => {return Ok(None)},
			Some(ProcessedTokenState::Incbin{..}) => { /* TODO check if paramters changed */ return Ok(None)}
            Some(ProcessedTokenState::If(..)) => {/* we qwant to read only for the selected branch*/ return Ok(None)}
			None => {return Ok(None)}
		};

		let ctx = &env.ctx;

		// whatever is the representation of the token, returns it simple version
		let token: Cow<Token> = self.token.as_simple_token();
		let token = token.as_ref();

		let mut info = None;

        todo!("Do not manipulate token but Listing Element api");

        self.state = match token {
            Token::Include(ref fname, _namespace, _once) => {
                let content = read_source(fname, ctx)?;

                let new_ctx = {
                    let mut new_ctx = ctx.clone();
                    new_ctx.set_current_filename(fname);
                    new_ctx
                };

                let listing = parse_z80_str_with_context(content, new_ctx)?;
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

impl<'token, T: Visited + ToSimpleToken + Debug + ListingElement + Sync>  ProcessedToken<'token, T> {
	/// Due to the state management, the signature requires mutability
	pub fn visited(&mut self, env: &mut Env) -> Result<(), AssemblerError> {

        let mut request_additional_pass = false;

        // Read file if needed
		if self.token.is_include() || self.token.is_incbin()
		{
			self.read_referenced_file(env)?;
		}	

        // Handle the tokens depending on their specific state
		match &mut self.state {
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
            },

            Some(ProcessedTokenState::If(if_state)) => {
                let listing = if_state.choose_listing_to_assemble(env)?;

                if let Some(listing) = listing {
                    visit_processed_tokens(listing, env)?;
                }

                Ok(())
            }

			Some(other) => unimplemented!("Specific behavior requiring a state not implemented. {:?}", other),
            // no state imply a standard visit
			None => self.token.visited(env),

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
