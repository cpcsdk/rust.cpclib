//! Legal reorderings of a small window of adjacent instructions - for
//! crunch-friendly instruction-order search ("does swapping these two
//! register-independent instructions make this block compress smaller"),
//! not a peephole *rewrite* - see this module's own scope note below.
//!
//! Deliberately a different, simpler question from what `constraints.rs`
//! answers. That module's `regsNotUsedAfter`/`flagsNotUsedAfter` etc.
//! decide whether a whole matched *region* can be rewritten or deleted,
//! which needs a real control-flow-following liveness walk (values can
//! still be read arbitrarily far downstream, through calls). Reordering a
//! handful of *adjacent* instructions is a strictly local question - do
//! these two instructions' own reads/writes conflict - answerable directly
//! from [`crate::effects::effects_of`] and [`crate::dependency::Dependency`]
//! with no CFG walk at all. So this sits beside `constraints.rs`, built on
//! the same lower-level primitives, rather than through it.
//!
//! Bounded to windows of 2-4 instructions (a caller-given search window,
//! matching real feedback's own "small groups of 3-4" scope): a full
//! permutation search is `O(n!)`, and each candidate is meant to be
//! assembled and crunched for real by the caller, so this stays small
//! enough that trying every legal permutation is actually cheap.

use cpclib_tokens::{ListingElement, Mnemonic};

use crate::effects::{Effects, effects_of};
use crate::dependency::Dependency;
use crate::stream::build_without_addresses;

/// Whether `mnemonic` can transfer control elsewhere (a jump, call, return,
/// or loop-decrement branch) or suspend normal execution (`halt`).
///
/// These are a hard barrier for reordering, not just another read/write
/// dependency: `Effects`-based conflict checking only reasons about
/// straight-line dataflow between instructions that all execute in
/// sequence, but a branch's own *presence* changes which instructions run
/// at all. Live-confirmed bug this guards against: an earlier version
/// proposed swapping `ld (SWITCH_PAGE+1),a` with the following `jr
/// c,PAGE2` (real skyline code) - the store and the jump have no
/// register/flag/memory conflict `must_preserve_order` can see, but moving
/// the store after the jump skips it whenever the branch is taken, which
/// is a real behavior change no dependency check catches.
///
/// Built on [`cpclib_z80flow::diverts_control`] (the same table `cfg`/
/// `liveness` use, rather than a second hand-written mnemonic list that
/// could drift from it) plus `HALT`, which that function deliberately
/// excludes for its own, different question ("does control still reach the
/// next instruction" - yes, once the interrupt that wakes it fires) but
/// which is still a real reordering hazard here: moving an instruction
/// across a `HALT` changes *when* the CPU suspends, an observable timing
/// change even though the same code eventually runs either way.
fn is_control_flow(mnemonic: Mnemonic) -> bool {
    mnemonic == Mnemonic::Halt || cpclib_z80flow::diverts_control(Some(mnemonic))
}

/// Widest window this module will search. Not a suggestion to the caller -
/// [`legal_reorderings`] returns an empty result above it, deliberately: a
/// full permutation search past a handful of instructions stops being a
/// reasonable in-process cost (`5! = 120` real assemble+crunch candidates
/// for one window), and this module's whole point is staying cheap enough
/// to try every legal permutation rather than heuristically pruning.
pub const MAX_WINDOW: usize = 4;

/// Every register/flag `effects` reads, as [`Dependency`]s.
fn reads_of(effects: &Effects) -> Vec<Dependency> {
    effects
        .reads
        .iter()
        .map(|&r| Dependency::Reg(r))
        .chain(effects.reads_flags.iter().map(|&f| Dependency::Flag(f)))
        .collect()
}

/// Every register/flag `effects` writes, as [`Dependency`]s.
fn writes_of(effects: &Effects) -> Vec<Dependency> {
    effects
        .writes
        .iter()
        .map(|&r| Dependency::Reg(r))
        .chain(effects.writes_flags.iter().map(|&f| Dependency::Flag(f)))
        .collect()
}

fn any_overlap(a: &[Dependency], b: &[Dependency]) -> bool {
    a.iter().any(|x| b.iter().any(|y| x.matches(*y)))
}

/// Whether the relative order of `a` and `b` (originally `a` before `b`)
/// must be preserved - i.e. whether they have a true (`a` writes what `b`
/// reads), anti (`a` reads what `b` writes), or output (both write the same
/// thing) dependency between them, via [`Dependency::matches`]'s pair/half-
/// aware overlap check.
///
/// Memory and port effects are conservatively treated as always
/// conflicting when *both* instructions touch them - real address aliasing
/// between two memory accesses is undecidable from the token stream alone
/// (the same policy `constraints.rs`'s `memoryNotWritten`/`memoryNotUsed`
/// already uses: decline rather than guess).
pub fn must_preserve_order(a: &Effects, b: &Effects) -> bool {
    let a_touches_world = a.reads_memory || a.writes_memory || a.reads_port || a.writes_port;
    let b_touches_world = b.reads_memory || b.writes_memory || b.reads_port || b.writes_port;
    if a_touches_world && b_touches_world {
        return true;
    }

    let (a_reads, a_writes) = (reads_of(a), writes_of(a));
    let (b_reads, b_writes) = (reads_of(b), writes_of(b));

    any_overlap(&a_writes, &b_reads)   // RAW
        || any_overlap(&a_reads, &b_writes) // WAR
        || any_overlap(&a_writes, &b_writes) // WAW
}

/// Whether reordering a window's original positions into `perm` (a
/// permutation of `0..effects.len()`) preserves every real dependency: for
/// every original pair `i < j` whose order [`must_preserve_order`] says
/// matters, `perm` must still place `i` before `j`.
fn permutation_is_legal(perm: &[usize], effects: &[Effects]) -> bool {
    let n = effects.len();
    let mut position_of = vec![0usize; n];
    for (new_pos, &original) in perm.iter().enumerate() {
        position_of[original] = new_pos;
    }
    for i in 0..n {
        for j in (i + 1)..n {
            if must_preserve_order(&effects[i], &effects[j]) && position_of[i] > position_of[j] {
                return false;
            }
        }
    }
    true
}

/// All permutations of `0..n`, via straightforward recursive swapping
/// (Heap's algorithm) - `n` is bounded by [`MAX_WINDOW`] (at most `4! =
/// 24`), so there is no need for a cleverer generator.
fn all_permutations(n: usize) -> Vec<Vec<usize>> {
    let mut indices: Vec<usize> = (0..n).collect();
    let mut results = Vec::new();
    fn heap(k: usize, indices: &mut Vec<usize>, results: &mut Vec<Vec<usize>>) {
        if k == 1 {
            results.push(indices.clone());
            return;
        }
        for i in 0..k {
            heap(k - 1, indices, results);
            if k % 2 == 0 {
                indices.swap(i, k - 1);
            }
            else {
                indices.swap(0, k - 1);
            }
        }
    }
    if n == 0 {
        return vec![vec![]];
    }
    heap(n, &mut indices, &mut results);
    results
}

/// Every legal, non-identity reordering of `tokens` - a small window of
/// **adjacent real instructions only** (not labels, not directives, not a
/// fake instruction/`JQ` that expands to something other than exactly one
/// real opcode - each token must normalize to exactly one
/// [`crate::analysis_op::AnalysisOp`], checked below rather than assumed),
/// as permutations of `0..tokens.len()`.
///
/// Takes a slice of *references* rather than owned tokens - matching
/// [`build_without_addresses`]'s own shape - because the real-world token
/// type this is meant to run on, `LocatedToken`, deliberately has no
/// `Clone` impl at all (unlike the plain `Token` this module's own tests
/// use, which does), so a signature requiring ownership would be unusable
/// on real parsed source.
///
/// Empty when the window has fewer than 2 or more than [`MAX_WINDOW`]
/// tokens, when any token isn't a describable single real instruction, when
/// [`effects_of`] cannot describe one of them at all - failing closed in
/// every case, per this crate's own "anything undescribable must never
/// read as touches nothing" rule (see `effects.rs`'s own doc comment) - or
/// when any instruction in the window is a branch/call/ret/djnz/rst/halt
/// (see [`is_control_flow`]).
pub fn legal_reorderings<T: ListingElement>(tokens: &[&T]) -> Vec<Vec<usize>> {
    let n = tokens.len();
    if !(2..=MAX_WINDOW).contains(&n) {
        return Vec::new();
    }

    let stream = build_without_addresses(tokens);
    if stream.ops().len() != n {
        return Vec::new();
    }

    // Hard barrier: a window containing any branch/call/ret/djnz/rst/halt
    // is never reorderable, regardless of what the dependency check below
    // would otherwise allow - see `is_control_flow`'s own doc comment.
    if stream.ops().iter().any(|op| op.mnemonic().is_some_and(is_control_flow)) {
        return Vec::new();
    }

    let mut effects = Vec::with_capacity(n);
    for op in stream.ops() {
        let Some(e) = effects_of(op)
        else {
            return Vec::new();
        };
        effects.push(e);
    }

    all_permutations(n)
        .into_iter()
        .filter(|perm| perm.as_slice() != (0..n).collect::<Vec<_>>().as_slice())
        .filter(|perm| permutation_is_legal(perm, &effects))
        .collect()
}

#[cfg(test)]
mod tests {
    use cpclib_tokens::{DataAccess, Mnemonic, Register8, Token};

    use super::*;

    fn ld_reg_imm(reg: Register8, value: i32) -> Token {
        Token::new_opcode(
            Mnemonic::Ld,
            Some(DataAccess::Register8(reg)),
            Some(DataAccess::Expression(cpclib_tokens::Expr::from(value)))
        )
    }

    fn ld_reg_reg(dst: Register8, src: Register8) -> Token {
        Token::new_opcode(
            Mnemonic::Ld,
            Some(DataAccess::Register8(dst)),
            Some(DataAccess::Register8(src))
        )
    }

    fn refs(tokens: &[Token]) -> Vec<&Token> {
        tokens.iter().collect()
    }

    #[test]
    fn two_independent_instructions_may_swap_either_way() {
        // `ld a,1` / `ld b,2` - disjoint registers, no flags either
        // instruction reads, so both orders are legal.
        let tokens = vec![ld_reg_imm(Register8::A, 1), ld_reg_imm(Register8::B, 2)];
        let perms = legal_reorderings(&refs(&tokens));
        assert_eq!(perms, vec![vec![1, 0]], "{perms:?}");
    }

    #[test]
    fn a_true_dependency_forbids_any_reordering() {
        // `ld a,1` / `ld b,a` - the second instruction reads what the first
        // just wrote, a real RAW dependency. No legal reordering exists.
        let tokens = vec![ld_reg_imm(Register8::A, 1), ld_reg_reg(Register8::B, Register8::A)];
        assert_eq!(legal_reorderings(&refs(&tokens)), Vec::<Vec<usize>>::new());
    }

    #[test]
    fn an_output_dependency_on_the_same_register_forbids_reordering() {
        // `ld a,1` / `ld a,2` - both write A. Reordering would change the
        // final value of A, a real WAW hazard.
        let tokens = vec![ld_reg_imm(Register8::A, 1), ld_reg_imm(Register8::A, 2)];
        assert_eq!(legal_reorderings(&refs(&tokens)), Vec::<Vec<usize>>::new());
    }

    #[test]
    fn three_pairwise_independent_instructions_have_every_non_identity_permutation_legal() {
        // ld a,1 / ld b,2 / ld c,3 - fully independent, so all 6
        // permutations of the window are behaviorally identical, and 5 of
        // them (everything but the identity) are reported.
        let tokens = vec![
            ld_reg_imm(Register8::A, 1),
            ld_reg_imm(Register8::B, 2),
            ld_reg_imm(Register8::C, 3),
        ];
        let perms = legal_reorderings(&refs(&tokens));
        assert_eq!(perms.len(), 5, "{perms:?}");
        assert!(!perms.contains(&vec![0, 1, 2]), "identity must be excluded: {perms:?}");
    }

    #[test]
    fn a_dependency_chain_only_allows_reorderings_that_keep_it_intact() {
        // ld a,1 (writes A) / ld b,2 (independent) / ld c,a (reads A,
        // depends on the first) - the first and third must stay in that
        // relative order; the middle instruction is free to move anywhere
        // around them. Legal permutations: [0,1,2] excluded (identity);
        // [1,0,2] (b first, doesn't disturb the a-chain); [0,2,1] (a, c,
        // then b - c still after a). [1,2,0]/[2,0,1]/[2,1,0] all put c
        // before a - illegal.
        let a = ld_reg_imm(Register8::A, 1);
        let b = ld_reg_imm(Register8::B, 2);
        let c = ld_reg_reg(Register8::C, Register8::A);
        let tokens = vec![a, b, c];
        let perms = legal_reorderings(&refs(&tokens));
        for perm in &perms {
            let pos_a = perm.iter().position(|&x| x == 0).unwrap();
            let pos_c = perm.iter().position(|&x| x == 2).unwrap();
            assert!(pos_a < pos_c, "a must stay before c in every legal permutation: {perm:?}");
        }
        assert!(perms.contains(&vec![1, 0, 2]), "{perms:?}");
        assert!(perms.contains(&vec![0, 2, 1]), "{perms:?}");
    }

    /// Regression test for the exact real bug reported against skyline:
    /// `ld (SWITCH_PAGE+1),a` followed by `jr c,PAGE2` (skyline_int.asm,
    /// lines 294-295) has no register/flag/memory conflict
    /// `must_preserve_order` can see between the two instructions
    /// themselves, but swapping them would skip the store whenever the
    /// branch is taken - a real behavior change. No reordering of this
    /// window may ever be proposed.
    #[test]
    fn a_conditional_jump_forbids_any_reordering_around_it() {
        let store = Token::new_opcode(
            Mnemonic::Ld,
            Some(DataAccess::Memory(cpclib_tokens::Expr::from(0x1234))),
            Some(DataAccess::Register8(Register8::A))
        );
        let branch = Token::new_opcode(
            Mnemonic::Jr,
            Some(DataAccess::FlagTest(cpclib_tokens::FlagTest::C)),
            Some(DataAccess::Expression(0.into()))
        );
        let tokens = vec![store, branch];
        assert_eq!(legal_reorderings(&refs(&tokens)), Vec::<Vec<usize>>::new());
    }

    /// Every control-flow mnemonic is a hard barrier, not just `JR` -
    /// spot-check the rest of the list so a future edit that narrows
    /// `is_control_flow` gets caught immediately.
    #[test]
    fn every_control_flow_mnemonic_is_a_hard_barrier() {
        for mnemonic in [
            Mnemonic::Jp,
            Mnemonic::Call,
            Mnemonic::Ret,
            Mnemonic::Reti,
            Mnemonic::Retn,
            Mnemonic::Djnz,
            Mnemonic::Rst,
            Mnemonic::Halt
        ] {
            assert!(is_control_flow(mnemonic), "{mnemonic:?} should be a hard barrier");
        }
        assert!(!is_control_flow(Mnemonic::Ld), "Ld must not be treated as control flow");
    }

    #[test]
    fn a_window_outside_two_to_four_instructions_is_rejected() {
        let one = vec![ld_reg_imm(Register8::A, 1)];
        assert_eq!(legal_reorderings(&refs(&one)), Vec::<Vec<usize>>::new());

        let five: Vec<Token> = (0..5)
            .map(|i| {
                ld_reg_imm(
                    [Register8::A, Register8::B, Register8::C, Register8::D, Register8::E][i],
                    i as i32
                )
            })
            .collect();
        assert_eq!(legal_reorderings(&refs(&five)), Vec::<Vec<usize>>::new());
    }

    #[test]
    fn must_preserve_order_is_symmetric_in_what_it_tests() {
        let writes_a = effects_of_test(&ld_reg_imm(Register8::A, 1));
        let reads_a = effects_of_test(&ld_reg_reg(Register8::B, Register8::A));
        assert!(must_preserve_order(&writes_a, &reads_a));
        let independent = effects_of_test(&ld_reg_imm(Register8::D, 4));
        assert!(!must_preserve_order(&writes_a, &independent));
    }

    fn effects_of_test(token: &Token) -> Effects {
        let tokens = [token.clone()];
        let refs: Vec<&Token> = tokens.iter().collect();
        let stream = build_without_addresses(&refs);
        effects_of(&stream.ops()[0]).expect("test instruction should have known effects")
    }
}
