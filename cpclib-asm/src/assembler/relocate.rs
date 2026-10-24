//! Support of `RELOCATE_START`, `RELOCATE_END` and `RELOCATE_TABLE`.
//!
//! Every label defined between `RELOCATE_START` and `RELOCATE_END` is
//! *relocatable*. `RELOCATE_TABLE` lists the position of every 16 bits value
//! (instruction operand or `DW`) whose expression is a single relocatable address.
//! Only the position of the value is needed, not its value, so the table has the same
//! size whatever the pass.

use std::collections::HashSet;

use cpclib_tokens::{BinaryOperation, Expr, UnaryOperation};

/// Key under which `$` is stored when met inside a relocatable block
const DOLLAR: &str = "$";

/// A 16 bits value that may need to be relocated
#[derive(Clone, Debug)]
pub struct RelocationCandidate {
    /// Logical address of the first byte of the value
    pub address: u16,
    /// The expression, to report errors
    pub expr: Expr,
    /// Labels (resolved) met by the expression with their sign when the expression is linear
    pub atoms: Vec<(String, i32)>,
    /// false when the expression contains something else than additions and subtractions
    pub linear: bool
}

#[derive(Clone, Debug, Default)]
pub struct RelocationState {
    /// true between RELOCATE_START and RELOCATE_END
    pub block_active: bool,
    /// true as soon as a RELOCATE_START has been met (it is kept from a pass to the next one,
    /// as it is useful to know it for the code before the first block)
    pub tracking: bool,
    /// Resolved names of the labels defined in a block (kept from a pass to the next one)
    relocatable_labels: HashSet<String>,
    /// Candidates of the current pass
    candidates: Vec<RelocationCandidate>,
    /// Number of entries of the table in the previous pass
    pub previous_count: Option<usize>
}

impl RelocationState {
    pub fn new_pass(&mut self) {
        self.block_active = false;
        self.candidates.clear();
    }

    pub fn add_relocatable_label(&mut self, resolved: String) {
        self.relocatable_labels.insert(resolved);
    }

    pub fn add_candidate(&mut self, candidate: RelocationCandidate) {
        self.candidates.push(candidate);
    }

    fn is_relocatable(&self, atom: &str) -> bool {
        atom == DOLLAR || self.relocatable_labels.contains(atom)
    }

    /// Addresses of the values to relocate, in the order they have been assembled.
    /// Fails with the faulty expression when it mixes relocatable labels in a way
    /// that cannot be fixed by adding one delta.
    pub fn addresses_to_relocate(&self) -> Result<Vec<u16>, String> {
        let mut addresses = Vec::new();
        for candidate in &self.candidates {
            let relocatable = candidate
                .atoms
                .iter()
                .filter(|(atom, _)| self.is_relocatable(atom));

            let delta = if candidate.linear {
                relocatable.map(|(_, sign)| sign).sum::<i32>()
            }
            else {
                // not an addition: only one label is allowed
                relocatable.count() as i32
            };

            let any = candidate
                .atoms
                .iter()
                .any(|(atom, _)| self.is_relocatable(atom));
            match (delta, any) {
                (_, false) => {},
                (1, _) => addresses.push(candidate.address),
                // differences of relocatable labels do not depend on the relocation
                (0, _) if candidate.linear => {},
                _ => {
                    return Err(format!(
                        "RELOCATE_TABLE: expression [{}] combines relocatable addresses in a way that cannot be relocated (it would need more than one fixup)",
                        candidate.expr
                    ));
                }
            }
        }
        Ok(addresses)
    }
}

/// Collect the labels of `expr`. `resolve` gives the name of a label in the symbols table.
/// Returns false if the expression is not an addition/subtraction of labels and constants.
pub fn collect_atoms(
    expr: &Expr,
    sign: i32,
    block_active: bool,
    resolve: &dyn Fn(&str) -> String,
    atoms: &mut Vec<(String, i32)>
) -> bool {
    match expr {
        Expr::Label(label) => {
            if label.as_str() == DOLLAR {
                if block_active {
                    atoms.push((DOLLAR.to_owned(), sign));
                }
            }
            else {
                atoms.push((resolve(label.as_str()), sign));
            }
            true
        },
        Expr::Paren(inner) => collect_atoms(inner, sign, block_active, resolve, atoms),
        Expr::UnaryOperation(UnaryOperation::Neg, inner) => {
            collect_atoms(inner, -sign, block_active, resolve, atoms)
        },
        Expr::BinaryOperation(BinaryOperation::Add, left, right) => {
            let left = collect_atoms(left, sign, block_active, resolve, atoms);
            let right = collect_atoms(right, sign, block_active, resolve, atoms);
            left && right
        },
        Expr::BinaryOperation(BinaryOperation::Sub, left, right) => {
            let left = collect_atoms(left, sign, block_active, resolve, atoms);
            let right = collect_atoms(right, -sign, block_active, resolve, atoms);
            left && right
        },
        Expr::UnaryOperation(_, inner) => {
            collect_atoms(inner, sign, block_active, resolve, atoms);
            false
        },
        Expr::BinaryOperation(_, left, right) => {
            collect_atoms(left, sign, block_active, resolve, atoms);
            collect_atoms(right, sign, block_active, resolve, atoms);
            false
        },
        Expr::Value(_) | Expr::Char(_) | Expr::Bool(_) | Expr::Float(_) | Expr::RelativeDelta(_) => {
            true
        },
        other => {
            // functions, ternary, strings... : the labels are only counted
            use cpclib_tokens::ExprElement;
            let mut symbols: Vec<String> = other.symbols().into_iter().collect();
            symbols.sort();
            atoms.extend(symbols.into_iter().map(|s| (resolve(&s), sign)));
            false
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn candidate(address: u16, expr: Expr, state: &RelocationState) -> RelocationCandidate {
        let mut atoms = Vec::new();
        let linear = collect_atoms(&expr, 1, true, &|s| s.to_uppercase(), &mut atoms);
        let _ = state;
        RelocationCandidate {
            address,
            expr,
            atoms,
            linear
        }
    }

    fn label(name: &str) -> Expr {
        Expr::Label(name.into())
    }

    #[test]
    fn single_label_is_relocated_but_not_a_constant_or_a_foreign_label() {
        let mut state = RelocationState::default();
        state.add_relocatable_label("INSIDE".into());
        for (address, expr) in [
            (1, label("inside")),
            (3, label("outside")),
            (5, Expr::Value(0x1234)),
            (7, Expr::BinaryOperation(BinaryOperation::Add, label("inside").into(), Expr::Value(5).into()))
        ] {
            let c = candidate(address, expr, &state);
            state.add_candidate(c);
        }
        assert_eq!(state.addresses_to_relocate().unwrap(), vec![1, 7]);
    }

    #[test]
    fn difference_of_relocatable_labels_is_not_relocated() {
        let mut state = RelocationState::default();
        state.add_relocatable_label("A".into());
        state.add_relocatable_label("B".into());
        let c = candidate(
            0,
            Expr::BinaryOperation(BinaryOperation::Sub, label("b").into(), label("a").into()),
            &state
        );
        state.add_candidate(c);
        assert!(state.addresses_to_relocate().unwrap().is_empty());
    }

    #[test]
    fn sum_of_relocatable_labels_is_an_error() {
        let mut state = RelocationState::default();
        state.add_relocatable_label("A".into());
        state.add_relocatable_label("B".into());
        let c = candidate(
            0,
            Expr::BinaryOperation(BinaryOperation::Add, label("b").into(), label("a").into()),
            &state
        );
        state.add_candidate(c);
        assert!(state.addresses_to_relocate().is_err());
    }
}
