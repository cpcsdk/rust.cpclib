use bumpalo::Bump;
use cpclib_common::{nom::{InputLength, Parser, error::{ParseError, ErrorKind}, IResult, Err}};

pub fn many_till<'bump, I, O, P, E, F, G>(
	mut f: F,
	mut g: G,
	bump: &'bump Bump
  ) -> impl FnMut(I) -> IResult<I, (Vec<O, &'bump Bump>, P), E>
  where
	I: Clone + InputLength,
	F: Parser<I, O, E>,
	G: Parser<I, P, E>,
	E: ParseError<I>,
  {
	move |mut i: I| {
	  let mut res = Vec::new_in(bump);
	  loop {
		let len = i.input_len();
		match g.parse(i.clone()) {
		  Ok((i1, o)) => return Ok((i1, (res, o))),
		  Err(Err::Error(_)) => {
			match f.parse(i.clone()) {
			  Err(Err::Error(err)) => return Err(Err::Error(E::append(i, ErrorKind::ManyTill, err))),
			  Err(e) => return Err(e),
			  Ok((i1, o)) => {
				// infinite loop check: the parser must always consume
				if i1.input_len() == len {
				  return Err(Err::Error(E::from_error_kind(i1, ErrorKind::ManyTill)));
				}
  
				res.push(o);
				i = i1;
			  }
			}
		  }
		  Err(e) => return Err(e),
		}
	  }
	}
  }



  pub fn separated_list0<'bump, I, O, O2, E, F, G>(
	mut sep: G,
	mut f: F,
	bump: &'bump Bump
  ) -> impl FnMut(I) -> IResult<I, Vec<O, &'bump Bump>, E>
  where
	I: Clone + InputLength,
	F: Parser<I, O, E>,
	G: Parser<I, O2, E>,
	E: ParseError<I>,
  {
	move |mut i: I| {
	  let mut res = Vec::new_in(bump);
  
	  match f.parse(i.clone()) {
		Err(Err::Error(_)) => return Ok((i, res)),
		Err(e) => return Err(e),
		Ok((i1, o)) => {
		  res.push(o);
		  i = i1;
		}
	  }
  
	  loop {
		let len = i.input_len();
		match sep.parse(i.clone()) {
		  Err(Err::Error(_)) => return Ok((i, res)),
		  Err(e) => return Err(e),
		  Ok((i1, _)) => {
			// infinite loop check: the parser must always consume
			if i1.input_len() == len {
			  return Err(Err::Error(E::from_error_kind(i1, ErrorKind::SeparatedList)));
			}
  
			match f.parse(i1.clone()) {
			  Err(Err::Error(_)) => return Ok((i, res)),
			  Err(e) => return Err(e),
			  Ok((i2, o)) => {
				res.push(o);
				i = i2;
			  }
			}
		  }
		}
	  }
	}
  }
  