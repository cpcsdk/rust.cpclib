//! Reconstructing the call stack from SP and memory.
//!
//! The emulator reports one frame: the program counter. Everything above it has
//! to be recovered from the stack, and the Z80 gives no help - a return address
//! is sixteen bits that look exactly like any other sixteen bits. The only
//! evidence available is what sits *before* the address a `CALL` would have
//! pushed: if `V` is a return address, then `V-3` must be a three-byte `CALL`.
//!
//! This follows [DeZog]'s implementation (`getCallStackFromEmulator` in
//! `src/remotes/remotebase.ts`) rather than an invention of ours, including one
//! decision that matters more than it looks:
//!
//! **No `RST` check.** DeZog implemented one and then removed it, with the
//! reasoning kept in a comment: an `RST` is rare, but one byte in thirty-two
//! matches an `RST` opcode by chance. Testing for it invents more frames than
//! it finds, so a program that uses `RST` shows the frames above the `RST` and
//! not the `RST` itself - a gap being better than a fiction.
//!
//! What comes out is still a heuristic. Data that happens to sit after three
//! bytes shaped like a `CALL` produces a frame that is not real. The caller is
//! expected to present a frame whose address resolves to no source line as
//! exactly that, rather than dressing it up.
//!
//! [DeZog]: https://github.com/maziac/DeZog

/// DeZog's cap, for the same reason: a stack pointer pointing into the weeds
/// would otherwise walk the whole address space, and nobody reads past a
/// hundred frames anyway.
pub const MAX_STACK_ITEMS: usize = 100;

/// How many unexplained words may sit between two frames before the walk stops
/// believing what it finds below them.
///
/// A routine's locals are the registers it pushed: a handful, occasionally a
/// dozen. Ninety-seven is not a call frame, it is the stack's older contents -
/// and any word down there whose `value - 3` happens to be a `CALL` opcode
/// becomes an invented frame with an invented name. That is exactly what
/// produced `0xC4BA [97 pushed]` on every stop of a real session, under a
/// perfectly good three-frame stack.
///
/// Generous rather than tight: this is the point past which the *evidence* is
/// gone, not a claim about how many registers a routine may push.
pub const MAX_LOCALS_BETWEEN_FRAMES: usize = 32;

/// One reconstructed caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallFrame {
    /// The page whose image holds the `CALL` this frame returned from.
    ///
    /// Falls out of the search for free: to decide whether `V-3` is a `CALL` at
    /// all, every page has to be tried, and the one that answered yes is the
    /// page the code was assembled into. That is the bank question answered
    /// without asking the emulator anything.
    pub page: Option<u8>,
    /// The address the `RET` will jump to.
    pub return_address: u16,
    /// Where the `CALL` itself is - three bytes earlier. This is the line the
    /// user wrote, and the one worth showing.
    pub call_site: u16,
    /// The address that `CALL` names, so the frame can say which routine it
    /// entered even when that routine has no symbol.
    pub called: u16,
    /// The values this frame pushed that are not return addresses: its locals,
    /// its saved registers, whatever it put there. Free, because they are what
    /// the walk skipped over.
    pub locals: Vec<u16>
}

/// Whether the three bytes at a candidate's `V-3` are a `CALL`.
///
/// `CD nn nn` is the unconditional form; the conditional ones - `C4 CC D4 DC E4
/// EC F4 FC` - are all `11ccc100`, which is the single mask below. Spelling the
/// eight out individually is the same set with more chances to mistype one.
pub fn is_call_opcode(opcode: u8) -> bool {
    opcode == 0xCD || (opcode & 0b1100_0111) == 0b1100_0100
}

/// Walk the stack, newest frame first.
///
/// * `sp` - the stack pointer at the stop.
/// * `stack` - the bytes from `sp` upwards, as read in one go.
/// * `read` - the byte at an address in the program image. Returning `None`
///   (firmware, unmapped) means "cannot tell", and a candidate that cannot be
///   checked is not claimed as a frame.
/// Walk the stack, newest frame first.
///
/// `read` is given a page and an address. A single-bank program passes a
/// closure that ignores the page; a banked one tries each page it emitted into,
/// and the page that turns out to hold a `CALL` is recorded on the frame.
pub fn walk_paged(
    stack: &[u8],
    pages: &[u8],
    read: impl Fn(u8, u16) -> Option<u8>
) -> Vec<CallFrame> {
    let mut frames: Vec<CallFrame> = Vec::new();
    // Values seen before the first return address belong to the innermost
    // frame - the one the emulator itself reported - so they are collected
    // separately and handed back by `locals_of_innermost_frame`.
    let mut pending_locals: Vec<u16> = Vec::new();

    let items = (stack.len() / 2).min(MAX_STACK_ITEMS);
    for index in 0..items {
        let value = u16::from_le_bytes([stack[index * 2], stack[index * 2 + 1]]);
        // A return address at 0, 1 or 2 would mean a `CALL` before the start of
        // memory. Nothing pushes those but data.
        let Some(call_site) = value.checked_sub(3)
        else {
            pending_locals.push(value);
            continue;
        };

        // The first page holding a `CALL` here wins. A logical address that is
        // a `CALL` in two banks is genuinely ambiguous, and the frame is shown
        // either way - only the source line it resolves to would differ.
        let found = pages.iter().find_map(|page| {
            match (
                read(*page, call_site),
                read(*page, call_site + 1),
                read(*page, call_site + 2)
            ) {
                (Some(opcode), Some(low), Some(high)) if is_call_opcode(opcode) => {
                    Some((*page, u16::from_le_bytes([low, high])))
                },
                // Either it is not a `CALL`, or the bytes cannot be read at all
                // - firmware, unmapped memory. "I cannot tell" is not "yes".
                _ => None
            }
        });
        let Some((page, call)) = found
        else {
            pending_locals.push(value);
            // Past this much unexplained stack, a `CALL` three bytes before a
            // value is coincidence rather than evidence. Stop, keeping the
            // frames found while there was still a chain to follow.
            if pending_locals.len() > MAX_LOCALS_BETWEEN_FRAMES {
                pending_locals.clear();
                break;
            }
            continue;
        };

        frames.push(CallFrame {
            page: Some(page),
            return_address: value,
            call_site,
            called: call,
            locals: std::mem::take(&mut pending_locals)
        });
    }

    // Whatever is left was pushed by the outermost frame found, above every
    // return address; it has nowhere better to go.
    if let Some(last) = frames.last_mut() {
        last.locals.extend(pending_locals);
    }
    frames
}

/// The single-page form, for a program that never banks.
pub fn walk(stack: &[u8], read: impl Fn(u16) -> Option<u8>) -> Vec<CallFrame> {
    walk_paged(stack, &[0], |_, address| read(address))
}

/// How many bytes to read from `sp` to cover the walk.
///
/// Stops at `top_of_stack` when one is known - reading past the top of the
/// stack is reading someone else's memory, and every word up there is a
/// candidate for a spurious frame.
pub fn bytes_to_read(sp: u16, top_of_stack: Option<u16>) -> usize {
    let cap = MAX_STACK_ITEMS * 2;
    match top_of_stack {
        Some(top) if top > sp => ((top - sp) as usize).min(cap),
        Some(_) => 0,
        None => cap.min(0x1_0000 - sp as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `CALL` the Z80 has, and nothing else.
    #[test]
    fn the_mask_matches_exactly_the_call_opcodes() {
        let calls = [0xCD, 0xC4, 0xCC, 0xD4, 0xDC, 0xE4, 0xEC, 0xF4, 0xFC];
        for opcode in calls {
            assert!(is_call_opcode(opcode), "0x{opcode:02X} is a CALL");
        }
        for opcode in 0u16..=0xFF {
            let opcode = opcode as u8;
            assert_eq!(
                is_call_opcode(opcode),
                calls.contains(&opcode),
                "0x{opcode:02X}"
            );
        }
    }

    /// `RST` must *not* be recognised - the case DeZog deliberately removed.
    #[test]
    fn rst_is_not_treated_as_a_call() {
        for opcode in [0xC7u8, 0xCF, 0xD7, 0xDF, 0xE7, 0xEF, 0xF7, 0xFF] {
            assert!(!is_call_opcode(opcode), "RST 0x{opcode:02X} is not a CALL");
        }
    }

    /// An image where `CALL nnnn` sits at chosen addresses.
    fn image(calls: &[(u16, u16)]) -> impl Fn(u16) -> Option<u8> + use<> {
        let mut memory = vec![0u8; 0x1_0000];
        for &(at, target) in calls {
            memory[at as usize] = 0xCD;
            memory[at as usize + 1] = target as u8;
            memory[at as usize + 2] = (target >> 8) as u8;
        }
        move |address: u16| Some(memory[address as usize])
    }

    fn stack(values: &[u16]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    /// In a paged program the page that holds the `CALL` is the page the
    /// frame is in - recovered from the walk itself, with nothing asked of the
    /// emulator.
    #[test]
    fn the_page_holding_the_call_is_recorded() {
        let mut page0 = vec![0u8; 0x1_0000];
        let mut page1 = vec![0u8; 0x1_0000];
        // Only page 1 has a CALL at 0x4000.
        page1[0x4000] = 0xCD;
        page1[0x4001] = 0x00;
        page1[0x4002] = 0x50;
        let read = move |page: u8, address: u16| {
            let image = if page == 0 { &page0 } else { &page1 };
            Some(image[address as usize])
        };

        let frames = walk_paged(&stack(&[0x4003]), &[0, 1], read);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].page, Some(1), "the page the code really lives in");
        assert_eq!(frames[0].called, 0x5000);
    }

    #[test]
    fn a_return_address_preceded_by_a_call_becomes_a_frame() {
        let read = image(&[(0x4000, 0x5000)]);
        let frames = walk(&stack(&[0x4003]), read);

        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].return_address, 0x4003);
        assert_eq!(frames[0].call_site, 0x4000, "the frame is at the CALL");
        assert_eq!(frames[0].called, 0x5000);
    }

    #[test]
    fn a_word_that_is_not_a_return_address_makes_no_frame() {
        let read = image(&[(0x4000, 0x5000)]);
        // 0x1234 - 3 is 0x1231, which holds nothing.
        let frames = walk(&stack(&[0x1234]), read);
        assert!(frames.is_empty());
    }

    /// The values a frame pushed come back attached to it, at no extra cost.
    #[test]
    fn values_below_a_return_address_become_that_frames_locals() {
        let read = image(&[(0x4000, 0x5000), (0x6000, 0x4000)]);
        let frames = walk(&stack(&[0xDEAD, 0xBEEF, 0x4003, 0x6003]), read);

        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].return_address, 0x4003);
        assert_eq!(frames[0].locals, vec![0xDEAD, 0xBEEF]);
        assert_eq!(frames[1].return_address, 0x6003);
        assert!(frames[1].locals.is_empty());
    }

    /// Every conditional form is followed, not only `CD`.
    #[test]
    fn a_conditional_call_is_followed_too() {
        let mut memory = vec![0u8; 0x1_0000];
        memory[0x4000] = 0xDC; // CALL C,0x5000
        memory[0x4001] = 0x00;
        memory[0x4002] = 0x50;
        let read = move |address: u16| Some(memory[address as usize]);

        let frames = walk(&stack(&[0x4003]), read);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].called, 0x5000);
    }

    /// A candidate whose `CALL` site cannot be read - firmware, unmapped - is
    /// not claimed. "I cannot tell" is not "yes".
    #[test]
    fn an_unreadable_call_site_produces_no_frame() {
        let frames = walk(&stack(&[0x4003]), |_| None);
        assert!(frames.is_empty());
    }

    #[test]
    fn the_walk_stops_at_a_hundred_items() {
        let read = image(&[(0x4000, 0x5000)]);
        let frames = walk(&stack(&vec![0x4003; 500]), read);
        assert_eq!(frames.len(), MAX_STACK_ITEMS);
    }

    #[test]
    fn the_read_stops_at_the_top_of_the_stack() {
        assert_eq!(bytes_to_read(0xBFF0, Some(0xC000)), 16);
        assert_eq!(bytes_to_read(0xBFF0, None), MAX_STACK_ITEMS * 2);
        assert_eq!(
            bytes_to_read(0xC000, Some(0xC000)),
            0,
            "an empty stack is read as nothing, not as the whole of memory"
        );
        assert_eq!(
            bytes_to_read(0xFFF0, None),
            16,
            "and never past the end of the address space"
        );
    }
}

#[cfg(test)]
mod deep_stack_tests {
    use super::*;

    /// A `CALL`-shaped coincidence far down the stack is not a frame.
    ///
    /// Reported from a real session: three good frames, and beneath them
    /// `0xC4BA [97 pushed]` - a name no symbol matched, at an address two pages
    /// claimed, on every single stop. Ninety-seven words of older stack, and
    /// one of them happened to have a `CALL` three bytes below it.
    #[test]
    fn a_coincidence_below_a_wall_of_stack_is_not_a_frame() {
        let mut memory = vec![0u8; 0x1_0000];
        // A real `call 0x5000` at 0x4000, returning to 0x4003.
        memory[0x4000] = 0xCD;
        memory[0x4002] = 0x50;
        // ...and a coincidence: `call 0x9000` at 0x8000, "returning" to 0x8003.
        memory[0x8000] = 0xCD;
        memory[0x8002] = 0x90;

        // The real return address, then a wall of unexplained words, then the
        // coincidence.
        let mut stack: Vec<u8> = vec![0x03, 0x40];
        stack.extend(std::iter::repeat_n([0xEF, 0xBE], 40).flatten());
        stack.extend([0x03, 0x80]);

        let frames = walk(&stack, |address| Some(memory[address as usize]));
        assert_eq!(frames.len(), 1, "only the one there is evidence for: {frames:?}");
        assert_eq!(frames[0].called, 0x5000);
        assert!(
            frames[0].locals.len() <= MAX_LOCALS_BETWEEN_FRAMES,
            "and it is not handed the whole wall: {:?}",
            frames[0].locals.len()
        );
    }

    /// A normal stack is untouched - a few locals between frames still chain.
    #[test]
    fn ordinary_frames_still_chain_through_their_locals() {
        let mut memory = vec![0u8; 0x1_0000];
        memory[0x4000] = 0xCD;
        memory[0x4002] = 0x50;
        memory[0x6000] = 0xCD;
        memory[0x6002] = 0x70;

        // Return address, four pushed registers, then the outer return address.
        let mut stack: Vec<u8> = vec![0x03, 0x40];
        stack.extend(std::iter::repeat_n([0xAD, 0xDE], 4).flatten());
        stack.extend([0x03, 0x60]);

        let frames = walk(&stack, |address| Some(memory[address as usize]));
        assert_eq!(frames.len(), 2, "{frames:?}");
        assert_eq!(frames[0].called, 0x5000);
        assert_eq!(frames[1].called, 0x7000);
        assert_eq!(frames[1].locals.len(), 4);
    }
}
