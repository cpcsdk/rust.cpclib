use std::collections::HashMap;

use cpclib_asm::{
    DataAccess, Expr, ExprEvaluationExt, Listing, Mnemonic, SymbolsTableTrait, Token, TokenExt,
    Value
};
use cpclib_common::event::EventObserver;
use cpclib_common::smol_str::SmolStr;

use crate::error::{BdAsmError, Result};

/// Resolves the absolute target address of a JR or DJNZ instruction.
///
/// The arithmetic itself now lives in `cpclib_asm::disass`, shared with
/// `cpclib-dap`'s live decoder - this is just this crate's own error
/// wrapping around it, kept local because `BdAsmError` is bdasm-specific.
fn resolve_jr_djnz_target(
    offset_expr: &Expr,
    current_address: Option<u16>,
    instruction: &Token
) -> Result<u16> {
    cpclib_asm::disass::resolve_jr_djnz_target(offset_expr, current_address).ok_or_else(|| {
        BdAsmError::UnknownAssemblerAddress {
            instruction: Box::new(instruction.clone()),
            bytes: 0
        }
    })
}

/// Collects all address references from jump/load instructions.
/// Only collects addresses within the valid_range (if provided) to avoid labeling external addresses like firmware routines.
/// TODO: move it in a public library
pub fn collect_addresses_from_expressions(
    listing: &Listing,
    valid_range: Option<std::ops::RangeInclusive<u16>>
) -> Result<Vec<u16>> {
    let mut labels: Vec<u16> = Default::default();

    let mut current_address: Option<u16> = None;
    let mut has_overflowed = false;
    for current_instruction in listing.iter() {
        assert!(
            !has_overflowed,
            "Address overflow occurred while processing instruction: {:?}",
            current_instruction
        );
        // Special handling for JR/DJNZ: they use PC-relative addressing
        if let Token::OpCode(Mnemonic::Djnz, Some(DataAccess::Expression(e)), ..)
        | Token::OpCode(Mnemonic::Jr, _, Some(DataAccess::Expression(e)), _) =
            current_instruction
        {
            let address = resolve_jr_djnz_target(e, current_address, current_instruction)?;
            // Only add label if it's within the valid range
            if valid_range.as_ref().is_none_or(|r| r.contains(&address)) {
                labels.push(address);
            }
        }
        // Generic handling: check all expressions in any OpCode for potential addresses
        else if let Token::OpCode(_, arg1, arg2, _) = current_instruction {
            // Helper closure to extract and check an expression
            let mut check_expression = |expr: &Expr, is_memory: bool| {
                if let Ok(value) = expr.eval()
                    && let Ok(address) = value.int_value()
                {
                    let address = address as u16;
                    // Always label memory references (they're 16-bit addresses)
                    // For direct expressions, only label values >= 256 (likely 16-bit addresses)
                    // Values < 256 are likely 8-bit immediate values, not addresses
                    let should_label = is_memory || address >= 256;

                    if should_label && valid_range.as_ref().is_none_or(|r| r.contains(&address)) {
                        labels.push(address);
                    }
                }
            };

            // Check first argument
            if let Some(arg) = arg1 {
                match arg {
                    DataAccess::Expression(e) => check_expression(e, false),
                    DataAccess::Memory(e) => check_expression(e, true),
                    _ => {}
                }
            }

            // Check second argument
            if let Some(arg) = arg2 {
                match arg {
                    DataAccess::Expression(e) => check_expression(e, false),
                    DataAccess::Memory(e) => check_expression(e, true),
                    _ => {}
                }
            }
        }

        let next_address = if let Token::Org { val1: address, .. } = current_instruction {
            let addr = address
                .eval()
                .map_err(|e| BdAsmError::ExprEvaluation(e.to_string()))?
                .int_value()
                .map_err(|e| BdAsmError::ExprEvaluation(e.to_string()))?
                as u16;
            current_address = Some(addr);
            current_address
        }
        else {
            let nb_bytes = current_instruction
                .number_of_bytes()
                .map_err(|e| BdAsmError::ExprEvaluation(e.to_string()))?;
            let res = match current_address {
                Some(address) => Some(address.overflowing_add(nb_bytes as u16)),
                None => {
                    if nb_bytes != 0 {
                        return Err(BdAsmError::UnknownAssemblerAddress {
                            instruction: Box::new(current_instruction.clone()),
                            bytes: nb_bytes
                        });
                    }
                    else {
                        None
                    }
                },
            };
            if let Some((next_addr, overflowed)) = res {
                if overflowed {
                    has_overflowed = true;
                }
                Some(next_addr)
            }
            else {
                None
            }
        };

        current_address = next_address;
    }

    Ok(labels)
}

/// Injects label names into expressions where possible.
pub fn inject_labels_into_expressions<O: EventObserver>(
    listing: &mut Listing,
    o: &O
) -> Result<()> {
    let (_bytes, table) = cpclib_asm::assemble_tokens_with_options(listing, Default::default())
        .map_err(|e| BdAsmError::AssemblyFailed(e.to_string()))?;

    let address_to_label = {
        let mut address_to_label = HashMap::<u16, &str>::default();
        for (s, v) in table.expression_symbol() {
            if s.value() == "$" || s.value() == "$$" {
                continue;
            }

            match v.value() {
                Value::Expr(expr) => {
                    if expr.is_int()
                        && let Some(int_val) = v.integer()
                    {
                        address_to_label.insert(int_val as u16, s.value());
                    }
                },
                Value::String(_) => {},
                Value::Address(a) => {
                    address_to_label.insert(a.address(), s.value());
                },
                // Skip unsupported types rather than panicking
                Value::Macro(_) | Value::Struct(_) | Value::Counter(_) => {
                    o.emit_stderr(&format!(
                        "Warning: Skipping label '{}' with unsupported value type",
                        s.value()
                    ));
                }
            }
        }
        address_to_label
    };

    let mut current_address: Option<u16> = None;
    for current_instruction in listing.iter_mut() {
        let next_address = if let Token::Org { val1: address, .. } = current_instruction {
            let addr = address
                .eval()
                .map_err(|e| BdAsmError::ExprEvaluation(e.to_string()))?
                .int_value()
                .map_err(|e| BdAsmError::ExprEvaluation(e.to_string()))?
                as u16;
            current_address = Some(addr);
            current_address
        }
        else {
            let nb_bytes = current_instruction
                .number_of_bytes()
                .map_err(|e| BdAsmError::ExprEvaluation(e.to_string()))?;
            match current_address {
                Some(address) => Some(address.wrapping_add(nb_bytes as u16)),
                None => {
                    if nb_bytes != 0 {
                        return Err(BdAsmError::UnknownAssemblerAddress {
                            instruction: Box::new(current_instruction.clone()),
                            bytes: nb_bytes
                        });
                    }
                    else {
                        None
                    }
                },
            }
        };

        // Clone before the mutable pattern match (borrow checker requirement)
        let instruction_snapshot = current_instruction.clone();

        // Special handling for JR/DJNZ: they use PC-relative addressing
        if let Token::OpCode(Mnemonic::Djnz, Some(DataAccess::Expression(e)), ..)
        | Token::OpCode(Mnemonic::Jr, _, Some(DataAccess::Expression(e)), _) =
            current_instruction
        {
            let address = resolve_jr_djnz_target(e, current_address, &instruction_snapshot)?;
            // Directly replace the expression with a label if one exists
            if let Some(label) = address_to_label.get(&address) {
                *e = Expr::Label(SmolStr::from(*label));
            }
        }
        // Generic handling: replace all expressions in any OpCode with labels if they match
        else if let Token::OpCode(_, arg1, arg2, _) = current_instruction {
            // Helper closure to replace an expression with a label if it matches
            let replace_if_label = |expr: &mut Expr| {
                if let Ok(value) = expr.eval()
                    && let Ok(address) = value.int_value()
                {
                    let address = address as u16;
                    if let Some(label) = address_to_label.get(&address) {
                        *expr = Expr::Label(SmolStr::from(*label));
                    }
                }
            };

            // Check and replace first argument
            if let Some(DataAccess::Expression(e) | DataAccess::Memory(e)) = arg1 {
                replace_if_label(e);
            }

            // Check and replace second argument
            if let Some(DataAccess::Expression(e) | DataAccess::Memory(e)) = arg2 {
                replace_if_label(e);
            }
        }

        current_address = next_address;
    }

    Ok(())
}
