mod shared;
mod html;
mod text;

use std::collections::HashMap;
use std::io::Write;

use cpclib_common::itertools::Itertools;

use super::*;
use super::format::{
	blank, format_address_for, format_deferred_line_with_template_for,
	format_line_with_template_for, hex_byte_for, logical_address_width,
	render_source_column
};

pub(crate) use self::shared::*;

