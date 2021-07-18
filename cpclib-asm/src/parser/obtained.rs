use std::{fs::File, io::Read, ops::{DerefMut, Deref}, thread::LocalKey};

use cpclib_tokens::{BaseListing, BinaryTransformation, CrunchType, Expr, Listing, ListingElement, TestKind, Token};
use itertools::Itertools;

use crate::{error::AssemblerError, implementation::expression::ExprEvaluationExt, preamble::{parse_z80_str, parse_z80_strboxed_with_contextboxed}};

use super::{ParserContext, Z80Span};
use super::parse_z80_str_with_context;

///! This crate is related to the adaptation of tokens and listing for the case where they are parsed

#[derive(Clone, Debug)]
/// Add span information for a Token.
/// This hierarchy is a mirror of the original token one
pub enum LocatedToken<'src, 'ctx> {
	/// A token without any listing embedding
	Standard{
		/// The original token without any span information
		token: Token,
		/// The span that correspond to the token
		span: Z80Span<'src, 'ctx>
	},
	CrunchedSection(CrunchType, LocatedListing<'src, 'ctx>, Z80Span<'src, 'ctx>),
	Include(String, Option<LocatedListing<'src,'ctx>>, Z80Span<'src, 'ctx>),
	If(Vec<(TestKind, LocatedListing<'src, 'ctx>)>, Option<LocatedListing<'src, 'ctx>>, Z80Span<'src, 'ctx>),
	Repeat(Expr, LocatedListing<'src, 'ctx>, Option<String>, Z80Span<'src, 'ctx>),
	RepeatUntil(Expr, LocatedListing<'src, 'ctx>, Z80Span<'src, 'ctx>),
	Rorg(Expr, LocatedListing<'src, 'ctx>, Z80Span<'src, 'ctx>),
	Switch(Vec<(Expr, LocatedListing<'src, 'ctx>)>, Z80Span<'src, 'ctx>),
	While(Expr, LocatedListing<'src, 'ctx>, Z80Span<'src, 'ctx>),
}

impl<'src, 'ctx> Deref for  LocatedToken<'src, 'ctx> {
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

impl<'src, 'ctx> LocatedToken<'src, 'ctx> {
	/// We can obtain a token only for "standard ones". Those that rely on listing need to be handled differently
	pub fn token(&self) -> Result<&Token, ()> {
		match self {
			Self::Standard{ token, ..} => Ok(token),
			_ => Err(())
		}
	}
	
	/// Get the span of the current token
	pub fn span(&self) -> &Z80Span<'src, 'ctx> {
		match self {
			Self::Standard{span, ..} |
			Self::CrunchedSection(_, _, span) |
			Self::Include(_, _, span) |
			Self::If(_, _, span) |
			Self::Repeat(_, _, _, span) |
			Self::RepeatUntil(_, _, span) |
			Self::Rorg(_, _, span) |
			Self::Switch(_, span) |
			Self::While(_, _, span) => span
		}
	}
	
	pub fn context(&self) -> &'ctx ParserContext {
		self.span().extra
	}
}


impl<'src, 'ctx>  LocatedToken<'src, 'ctx> {
	pub fn as_token(&self) -> Token {
		match self {
			LocatedToken::Standard{ token, ..} => token.clone(),
			LocatedToken::CrunchedSection(_, _, span)  => unimplemented!(),
			LocatedToken::Include(_, _, span)  => unimplemented!(),
			LocatedToken::If(_, _, span)  => unimplemented!(),
			LocatedToken::Repeat(_, _, _, span)  => unimplemented!(),
			LocatedToken::RepeatUntil(_, _, span)  => unimplemented!(),
			LocatedToken::Rorg(_, _, span)  => unimplemented!(),
			LocatedToken::Switch(_, span)  => unimplemented!(),
			LocatedToken::While(_, _, span)  => unimplemented!(),
		}
	}
	
	pub fn parse_token(value: & str) -> Result<LocatedToken, String> {
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
	/// TODO move this code elswhere as it can be useful in other contexts
	pub fn read_referenced_file(&mut self, ctx: &ParserContext) -> Result<(), AssemblerError> {
		match self {
			LocatedToken::Include(ref fname, ref mut listing, span) if listing.is_none() => {
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
						let content = Box::new(content);
						let mut new_ctx = Box::new(ctx.clone());
						new_ctx.set_current_filename(fname);
						listing.replace(parse_z80_strboxed_with_contextboxed(content, new_ctx)?);
					}
				}
			}
			
			LocatedToken::Standard{token:Token::Incbin {
				fname,
				offset,
				length,
				extended_offset: _,
				off: _,
				ref mut content,
				transformation,
			}, span} if content.is_none() => {
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
								content.replace(data);
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
								content.replace(crunched);
							}
							
							BinaryTransformation::Aplib => {
								if data.len() == 0 {
									return Err(AssemblerError::EmptyBinaryFile(
										fname.to_string_lossy().to_string(),
									));
								}
								
								let crunched = crate::crunchers::apultra::compress(&data);
								content.replace(crunched);
							}
						}
					}
				}
			}
			
			// Rorg may embed some instructions that read files
			LocatedToken::Rorg(_, ref mut listing, _) => {
				for token in listing.iter_mut() {
					token.read_referenced_file(ctx)?;
				}
			}
			_ => {}
		}
		
		Ok(())
	}
	
}
/// Implement this trait for type previousy defined without source location.

pub trait Locate<'src, 'ctx>  {
	type Output;
	
	fn locate(self, span: Z80Span<'src, 'ctx>) -> Self::Output;
}

impl<'src, 'ctx> Locate<'src, 'ctx> for Token {
	type Output = LocatedToken<'src, 'ctx>;
	
	fn locate(self, span: Z80Span<'src, 'ctx>) -> LocatedToken<'src, 'ctx> {
		if self.has_at_least_one_listing() {/*/
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
			LocatedToken::Standard{token: self, span}
		}
	}
}



impl ListingElement for LocatedToken<'_, '_> {}

pub type InnerLocatedListing<'src, 'ctx> = BaseListing<LocatedToken<'src, 'ctx>>;

/// Represents a Listing of located tokens
#[derive(Clone, Debug)]
pub struct LocatedListing<'src, 'ctx>(InnerLocatedListing<'src, 'ctx> );

impl<'src, 'ctx>  Deref for LocatedListing<'src, 'ctx> {
	type Target = InnerLocatedListing<'src, 'ctx>;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl<'src, 'ctx>  DerefMut for LocatedListing<'src, 'ctx> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl<'src, 'ctx>  From<Vec<LocatedToken<'src, 'ctx>>> for LocatedListing<'src, 'ctx> {
	fn from(tokens: Vec<LocatedToken<'src, 'ctx>>) -> Self {
		Self(tokens.into())
	}
}

impl<'src, 'ctx> LocatedListing<'src, 'ctx> {
	pub fn as_listing(&self) -> Listing {
		self.0.iter()
		.map( |lt| lt.as_token())
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
		token.map(|lt|lt.as_token())
	}
}