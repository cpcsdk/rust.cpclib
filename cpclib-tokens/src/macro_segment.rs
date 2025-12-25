use std::ops::Deref;

use cpclib_common::smallvec::SmallVec;
use memchr::memchr;

/// Tokenize a macro body into MacroSegments
pub fn tokenize_macro_body<'l, 'p>(
    listing: &'l str,
    params: &'p [impl AsRef<str> + 'p]
) -> TokenizedMacroContent {
    let mut segments: SmallVec<[MacroSegment; 8]> = SmallVec::with_capacity(listing.len() / 8);
    let mut cursor = 0;
    let param_names: std::collections::HashMap<&'p str, usize> = params
        .iter()
        .enumerate()
        .map(|(idx, p)| {
            let s: &'p str = p.as_ref();
            let key = if let Some(stripped) = s.strip_prefix("r#") {
                stripped
            }
            else {
                s
            };
            (key, idx)
        })
        .collect();
    let bytes = listing.as_bytes();
    while let Some(rel_open) = memchr(b'{', &bytes[cursor..]) {
        let open = cursor + rel_open;
        if open > cursor {
            segments.push(MacroSegment::Lit {
                start: cursor,
                end: open
            });
        }
        let after_open = open + 1;
        if let Some(rel_close) = memchr(b'}', &bytes[after_open..]) {
            let close = after_open + rel_close;
            let key = &listing[after_open..close];
            if let Some(&idx) = param_names.get(key) {
                segments.push(MacroSegment::Arg { index: idx });
                cursor = close + 1;
                continue;
            }
            segments.push(MacroSegment::Lit {
                start: open,
                end: close + 1
            });
            cursor = close + 1;
        }
        else {
            segments.push(MacroSegment::Lit {
                start: open,
                end: listing.len()
            });
            cursor = listing.len();
        }
    }
    if cursor < listing.len() {
        segments.push(MacroSegment::Lit {
            start: cursor,
            end: listing.len()
        });
    }

    TokenizedMacroContent {
        segments: segments.into_vec()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum MacroSegment {
    Lit { start: usize, end: usize },
    Arg { index: usize }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct TokenizedMacroContent {
    pub segments: Vec<MacroSegment>
}

impl Deref for TokenizedMacroContent {
    type Target = [MacroSegment];

    fn deref(&self) -> &Self::Target {
        &self.segments
    }
}
