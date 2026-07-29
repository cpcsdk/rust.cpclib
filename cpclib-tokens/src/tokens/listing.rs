use core::fmt::Debug;
use std::borrow::Cow;
use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};

use cpclib_common::smallvec::SmallVec;

use crate::{
    AssemblerControlCommand, AssemblerFlavor, BinaryTransformation, CrunchType, DataAccess,
    DataAccessElem, ExprElement, MacroParamElement, Mnemonic, Register8, Register16,
    TestKindElement
};

//
/// The ListingElement trait contains the public method any member of a listing should contain
/// ATM there is nothing really usefull
pub trait ListingElement
where Self: Debug + Sized + Sync
{
    type MacroParam: MacroParamElement;
    type TestKind: TestKindElement;
    type Expr: ExprElement + Debug + Eq + Clone + std::fmt::Display;
    // type Element: ListingElement + Debug + Sync;
    type DataAccess: DataAccessElem<Expr = Self::Expr>;
    // type Listing: ListingTrait;
    type AssemblerControlCommand: AssemblerControlCommand;

    fn defer_listing_output(&self) -> bool {
        false // self.is_equ() | self.is_set()
    }

    fn is_opcode(&self) -> bool {
        !self.is_directive()
    }

    fn is_assert(&self) -> bool;

    fn is_buildcpr(&self) -> bool;
    fn is_assembler_control(&self) -> bool;
    fn assembler_control_command(&self) -> &Self::AssemblerControlCommand;
    fn assembler_control_get_max_passes(&self) -> Option<&Self::Expr>;
    fn assembler_control_get_listing(&self) -> &[Self];

    fn is_org(&self) -> bool;
    fn org_first(&self) -> &Self::Expr;
    fn org_second(&self) -> Option<&Self::Expr>;

    fn is_comment(&self) -> bool;
    fn comment(&self) -> &str;

    fn is_set(&self) -> bool;

    fn is_label(&self) -> bool;
    fn is_equ(&self) -> bool;
    fn is_assign(&self) -> bool;
    fn equ_symbol(&self) -> &str;
    fn equ_value(&self) -> &Self::Expr;
    fn label_symbol(&self) -> &str;
    fn assign_symbol(&self) -> &str;
    fn assign_value(&self) -> &Self::Expr;

    fn is_warning(&self) -> bool;
    fn warning_token(&self) -> &Self;
    fn warning_message(&self) -> &str;

    fn mnemonic(&self) -> Option<&Mnemonic>;
    fn mnemonic_arg1(&self) -> Option<&Self::DataAccess>;
    fn mnemonic_arg2(&self) -> Option<&Self::DataAccess>;
    fn mnemonic_arg1_mut(&mut self) -> Option<&mut Self::DataAccess>;
    fn mnemonic_arg2_mut(&mut self) -> Option<&mut Self::DataAccess>;

    /// Accessor for instruction where A can be written optionally as a first argument
    fn mnemonic_unique_arg(&self) -> Option<&Self::DataAccess> {
        self.mnemonic_arg2().or_else(|| self.mnemonic_arg1())
    }

    fn is_directive(&self) -> bool;

    fn is_module(&self) -> bool;
    // fn module_listing(&self) -> &[Self];
    fn module_listing(&self) -> &[Self];
    fn module_name(&self) -> &str;

    fn is_while(&self) -> bool;
    fn while_expr(&self) -> &Self::Expr;
    fn while_listing(&self) -> &[Self];

    fn is_switch(&self) -> bool;
    fn switch_expr(&self) -> &Self::Expr;
    fn switch_cases(&self) -> Box<dyn Iterator<Item = (&Self::Expr, &[Self], bool)> + '_>;
    fn switch_default(&self) -> Option<&[Self]>;

    fn is_iterate(&self) -> bool;
    fn iterate_listing(&self) -> &[Self];
    fn iterate_counter_name(&self) -> &str;
    fn iterate_values(&self) -> either::Either<&Vec<Self::Expr>, &Self::Expr>;

    fn is_for(&self) -> bool;
    fn for_listing(&self) -> &[Self];
    fn for_label(&self) -> &str;
    fn for_start(&self) -> &Self::Expr;
    fn for_stop(&self) -> &Self::Expr;
    fn for_step(&self) -> Option<&Self::Expr>;

    fn is_repeat_token(&self) -> bool;
    fn repeat_token(&self) -> &Self;

    fn is_repeat_until(&self) -> bool;
    fn repeat_until_listing(&self) -> &[Self];
    fn repeat_until_condition(&self) -> &Self::Expr;

    fn is_rorg(&self) -> bool;
    fn rorg_listing(&self) -> &[Self];
    fn rorg_expr(&self) -> &Self::Expr;

    fn is_repeat(&self) -> bool;
    fn repeat_listing(&self) -> &[Self];
    fn repeat_count(&self) -> &Self::Expr;
    fn repeat_counter_name(&self) -> Option<&str>;
    fn repeat_counter_start(&self) -> Option<&Self::Expr>;
    fn repeat_counter_step(&self) -> Option<&Self::Expr>;

    fn is_crunched_section(&self) -> bool;
    fn crunched_section_listing(&self) -> &[Self];
    fn crunched_section_kind(&self) -> &CrunchType;

    fn is_macro_definition(&self) -> bool;
    fn macro_definition_name(&self) -> &str;
    fn macro_definition_arguments(&self) -> SmallVec<[&str; 4]>;
    fn macro_definition_code(&self) -> &str;
    fn macro_flavor(&self) -> AssemblerFlavor;

    fn is_call_macro_or_build_struct(&self) -> bool;
    fn macro_call_name(&self) -> &str;
    fn macro_call_arguments(&self) -> &[Self::MacroParam];

    fn is_if(&self) -> bool;
    fn if_nb_tests(&self) -> usize;
    fn if_test(&self, idx: usize) -> (&Self::TestKind, &[Self]);
    fn if_else(&self) -> Option<&[Self]>;

    fn is_incbin(&self) -> bool;
    fn incbin_fname(&self) -> &Self::Expr;
    fn incbin_offset(&self) -> Option<&Self::Expr>;
    fn incbin_length(&self) -> Option<&Self::Expr>;
    fn incbin_transformation(&self) -> &BinaryTransformation;

    fn is_include(&self) -> bool;
    fn include_fname(&self) -> &Self::Expr;
    fn include_namespace(&self) -> Option<&str>;
    fn include_once(&self) -> bool;
    fn include_is_standard_include(&self) -> bool {
        //   let has_bracket = self.incbin_fname().to_string().contains('{');

        self.is_include() &&
       /* !self.include_fname().contains('{') &&*/ // no expansion
        !self.include_once()
    }

    fn is_function_definition(&self) -> bool;
    fn function_definition_name(&self) -> &str;
    fn function_definition_params(&self) -> SmallVec<[&str; 4]>;
    fn function_definition_inner(&self) -> &[Self];

    fn is_confined(&self) -> bool;
    fn confined_listing(&self) -> &[Self];

    fn is_db(&self) -> bool;
    fn is_dw(&self) -> bool;
    fn is_str(&self) -> bool;
    fn data_exprs(&self) -> &[Self::Expr];

    fn is_run(&self) -> bool;
    fn run_expr(&self) -> &Self::Expr;

    fn is_return(&self) -> bool;
    fn return_value(&self) -> &Self::Expr;

    fn is_print(&self) -> bool;
    fn is_breakpoint(&self) -> bool;
    fn is_save(&self) -> bool;

    fn to_token(&self) -> Cow<'_, crate::Token>;
    fn starts_with_label(&self) -> bool {
        self.is_label() || self.is_assign() || self.is_equ() || self.is_set()
    }

    #[inline]
    fn fake_to_listing_from_access<DA: DataAccessElem>(
        mnemonic: Mnemonic,
        arg1: Option<&DA>,
        arg2: Option<&DA>,
        arg3: Option<Register8>
    ) -> Option<Vec<(Mnemonic, Option<DataAccess>, Option<DataAccess>)>> {
        if arg3.is_some() {
            return None;
        }

        let mut listing = Vec::new();

        match mnemonic {
            Mnemonic::Add | Mnemonic::Adc
                if arg1.as_ref().map(|a| a.is_register_de()).unwrap_or(false)
                    && arg2.as_ref().map(|a| a.is_register16()).unwrap_or(false) =>
            {
                let rhs = arg2.unwrap().get_register16().unwrap();
                let mapped_rhs = rhs.swap_de_hl();
                listing.push((Mnemonic::ExHlDe, None, None));
                listing.push((
                    mnemonic,
                    Some(DataAccess::Register16(Register16::Hl)),
                    Some(DataAccess::Register16(mapped_rhs))
                ));
                listing.push((Mnemonic::ExHlDe, None, None));
                Some(listing)
            },

            Mnemonic::Sbc
                if arg1.as_ref().map(|a| a.is_register_de()).unwrap_or(false)
                    && arg2.as_ref().map(|a| a.is_register16()).unwrap_or(false) =>
            {
                let rhs = arg2.unwrap().get_register16().unwrap();
                let mapped_rhs = rhs.swap_de_hl();
                listing.push((Mnemonic::ExHlDe, None, None));
                listing.push((
                    Mnemonic::Sbc,
                    Some(DataAccess::Register16(Register16::Hl)),
                    Some(DataAccess::Register16(mapped_rhs))
                ));
                listing.push((Mnemonic::ExHlDe, None, None));
                Some(listing)
            },

            Mnemonic::Sub
                if arg1
                    .as_ref()
                    .map(|a| a.is_register_de() || a.is_register_hl())
                    .unwrap_or(false)
                    && arg2.as_ref().map(|a| a.is_register16()).unwrap_or(false) =>
            {
                let lhs = arg1.unwrap();
                let rhs = arg2.unwrap().get_register16().unwrap();
                listing.push((
                    Mnemonic::Or,
                    Some(DataAccess::Register8(Register8::A)),
                    None
                ));

                if lhs.is_register_de() {
                    let mapped_rhs = rhs.swap_de_hl();
                    listing.push((Mnemonic::ExHlDe, None, None));
                    listing.push((
                        Mnemonic::Sbc,
                        Some(DataAccess::Register16(Register16::Hl)),
                        Some(DataAccess::Register16(mapped_rhs))
                    ));
                    listing.push((Mnemonic::ExHlDe, None, None));
                }
                else {
                    listing.push((
                        Mnemonic::Sbc,
                        Some(DataAccess::Register16(Register16::Hl)),
                        Some(DataAccess::Register16(rhs))
                    ));
                }

                Some(listing)
            },

            Mnemonic::Ld
                if arg1.as_ref().map(|a| a.is_register_hl()).unwrap_or(false)
                    && arg2.as_ref().map(|a| a.is_register_sp()).unwrap_or(false) =>
            {
                listing.push((
                    Mnemonic::Ld,
                    Some(DataAccess::Register16(Register16::Hl)),
                    Some(DataAccess::Expression(0u8.into()))
                ));
                listing.push((
                    Mnemonic::Add,
                    Some(DataAccess::Register16(Register16::Hl)),
                    Some(DataAccess::Register16(Register16::Sp))
                ));
                Some(listing)
            },

            Mnemonic::Ld
                if arg1
                    .as_ref()
                    .map(|a| a.is_register_bc() || a.is_register_de() || a.is_register_hl())
                    .unwrap_or(false)
                    && arg2
                        .as_ref()
                        .map(|a| a.is_register_bc() || a.is_register_de() || a.is_register_hl())
                        .unwrap_or(false) =>
            {
                let lhs = arg1.unwrap().get_register16().unwrap();
                let rhs = arg2.unwrap().get_register16().unwrap();
                listing.push((
                    Mnemonic::Ld,
                    Some(DataAccess::Register8(lhs.low().unwrap())),
                    Some(DataAccess::Register8(rhs.low().unwrap()))
                ));
                listing.push((
                    Mnemonic::Ld,
                    Some(DataAccess::Register8(lhs.high().unwrap())),
                    Some(DataAccess::Register8(rhs.high().unwrap()))
                ));
                Some(listing)
            },

            Mnemonic::Ld
                if arg1
                    .as_ref()
                    .map(|a| a.is_register_bc() || a.is_register_de() || a.is_register_hl())
                    .unwrap_or(false)
                    && arg2
                        .as_ref()
                        .map(|a| a.is_indexregister_with_index())
                        .unwrap_or(false) =>
            {
                let dst = arg1.unwrap().get_register16().unwrap();
                let src = arg2.unwrap().get_indexregister16().unwrap();
                let idx = arg2.unwrap().get_index().unwrap();

                listing.push((
                    Mnemonic::Ld,
                    Some(DataAccess::Register8(dst.low().unwrap())),
                    Some(DataAccess::IndexRegister16WithIndex(
                        src,
                        idx.0,
                        idx.1.to_expr().into_owned()
                    ))
                ));
                listing.push((
                    Mnemonic::Ld,
                    Some(DataAccess::Register8(dst.high().unwrap())),
                    Some(DataAccess::IndexRegister16WithIndex(
                        src,
                        idx.0,
                        idx.1.to_expr().into_owned().add(1)
                    ))
                ));
                Some(listing)
            },

            Mnemonic::Ld
                if arg1
                    .as_ref()
                    .map(|a| a.is_indexregister_with_index())
                    .unwrap_or(false)
                    && arg2
                        .as_ref()
                        .map(|a| a.is_register_bc() || a.is_register_de() || a.is_register_hl())
                        .unwrap_or(false) =>
            {
                let dst = arg1.unwrap().get_indexregister16().unwrap();
                let idx = arg1.unwrap().get_index().unwrap();
                let src = arg2.unwrap().get_register16().unwrap();

                listing.push((
                    Mnemonic::Ld,
                    Some(DataAccess::IndexRegister16WithIndex(
                        dst,
                        idx.0,
                        idx.1.to_expr().into_owned()
                    )),
                    Some(DataAccess::Register8(src.low().unwrap()))
                ));
                listing.push((
                    Mnemonic::Ld,
                    Some(DataAccess::IndexRegister16WithIndex(
                        dst,
                        idx.0,
                        idx.1.to_expr().into_owned().add(1)
                    )),
                    Some(DataAccess::Register8(src.high().unwrap()))
                ));
                Some(listing)
            },

            Mnemonic::Ld
                if (arg1.as_ref().map(|a| a.is_register_hl()).unwrap_or(false)
                    && arg2
                        .as_ref()
                        .map(|a| a.is_indexregister16())
                        .unwrap_or(false))
                    || (arg1
                        .as_ref()
                        .map(|a| a.is_indexregister16())
                        .unwrap_or(false)
                        && arg2.as_ref().map(|a| a.is_register_hl()).unwrap_or(false))
                    || (arg1
                        .as_ref()
                        .map(|a| a.is_indexregister16())
                        .unwrap_or(false)
                        && arg2
                            .as_ref()
                            .map(|a| a.is_indexregister16())
                            .unwrap_or(false)) =>
            {
                let dst = if arg1.unwrap().is_register16() {
                    DataAccess::Register16(arg1.unwrap().get_register16().unwrap())
                }
                else {
                    DataAccess::IndexRegister16(arg1.unwrap().get_indexregister16().unwrap())
                };

                let src = if arg2.unwrap().is_register16() {
                    DataAccess::Register16(arg2.unwrap().get_register16().unwrap())
                }
                else {
                    DataAccess::IndexRegister16(arg2.unwrap().get_indexregister16().unwrap())
                };

                listing.push((Mnemonic::Push, Some(src), None));
                listing.push((Mnemonic::Pop, Some(dst), None));
                Some(listing)
            },

            Mnemonic::Ld
                if arg1
                    .as_ref()
                    .map(|a| a.is_register_bc() || a.is_register_de() || a.is_register_hl())
                    .unwrap_or(false)
                    && arg2
                        .as_ref()
                        .map(|a| a.is_indexregister16())
                        .unwrap_or(false) =>
            {
                let dst = arg1.unwrap().get_register16().unwrap();
                let src = arg2.unwrap().get_indexregister16().unwrap();

                listing.push((
                    Mnemonic::Ld,
                    Some(DataAccess::Register8(dst.low().unwrap())),
                    Some(DataAccess::IndexRegister8(src.low()))
                ));
                listing.push((
                    Mnemonic::Ld,
                    Some(DataAccess::Register8(dst.high().unwrap())),
                    Some(DataAccess::IndexRegister8(src.high()))
                ));
                Some(listing)
            },

            Mnemonic::Ld
                if arg1
                    .as_ref()
                    .map(|a| a.is_indexregister16())
                    .unwrap_or(false)
                    && arg2
                        .as_ref()
                        .map(|a| a.is_register_bc() || a.is_register_de() || a.is_register_hl())
                        .unwrap_or(false) =>
            {
                let dst = arg1.unwrap().get_indexregister16().unwrap();
                let src = arg2.unwrap().get_register16().unwrap();

                listing.push((
                    Mnemonic::Ld,
                    Some(DataAccess::IndexRegister8(dst.low())),
                    Some(DataAccess::Register8(src.low().unwrap()))
                ));
                listing.push((
                    Mnemonic::Ld,
                    Some(DataAccess::IndexRegister8(dst.high())),
                    Some(DataAccess::Register8(src.high().unwrap()))
                ));
                Some(listing)
            },

            Mnemonic::Ld
                if arg1
                    .as_ref()
                    .map(|a| a.is_register_bc() || a.is_register_de() || a.is_register_hl())
                    .unwrap_or(false)
                    && arg2.as_ref().map(|a| a.is_address_in_hl()).unwrap_or(false) =>
            {
                let dst = arg1.unwrap().get_register16().unwrap();

                listing.push((
                    Mnemonic::Ld,
                    Some(DataAccess::Register8(dst.low().unwrap())),
                    Some(DataAccess::MemoryRegister16(Register16::Hl))
                ));
                listing.push((
                    Mnemonic::Inc,
                    Some(DataAccess::Register16(Register16::Hl)),
                    None
                ));
                listing.push((
                    Mnemonic::Ld,
                    Some(DataAccess::Register8(dst.high().unwrap())),
                    Some(DataAccess::MemoryRegister16(Register16::Hl))
                ));
                listing.push((
                    Mnemonic::Dec,
                    Some(DataAccess::Register16(Register16::Hl)),
                    None
                ));
                Some(listing)
            },

            Mnemonic::Ld
                if arg1.as_ref().map(|a| a.is_address_in_hl()).unwrap_or(false)
                    && arg2
                        .as_ref()
                        .map(|a| a.is_register_bc() || a.is_register_de() || a.is_register_hl())
                        .unwrap_or(false) =>
            {
                let src = arg2.unwrap().get_register16().unwrap();

                listing.push((
                    Mnemonic::Ld,
                    Some(DataAccess::MemoryRegister16(Register16::Hl)),
                    Some(DataAccess::Register8(src.low().unwrap()))
                ));
                listing.push((
                    Mnemonic::Inc,
                    Some(DataAccess::Register16(Register16::Hl)),
                    None
                ));
                listing.push((
                    Mnemonic::Ld,
                    Some(DataAccess::MemoryRegister16(Register16::Hl)),
                    Some(DataAccess::Register8(src.high().unwrap()))
                ));
                listing.push((
                    Mnemonic::Dec,
                    Some(DataAccess::Register16(Register16::Hl)),
                    None
                ));
                Some(listing)
            },

            Mnemonic::Srl8
                if arg1
                    .as_ref()
                    .map(|a| a.is_register16() || a.is_indexregister16())
                    .unwrap_or(false) =>
            {
                if arg1.unwrap().is_register16() {
                    let reg = arg1.unwrap().get_register16().unwrap();
                    listing.push((
                        Mnemonic::Ld,
                        Some(DataAccess::Register8(reg.low().unwrap())),
                        Some(DataAccess::Register8(reg.high().unwrap()))
                    ));
                    listing.push((
                        Mnemonic::Ld,
                        Some(DataAccess::Register8(reg.high().unwrap())),
                        Some(DataAccess::Expression(0u8.into()))
                    ));
                }
                else {
                    let reg = arg1.unwrap().get_indexregister16().unwrap();
                    listing.push((
                        Mnemonic::Ld,
                        Some(DataAccess::IndexRegister8(reg.low())),
                        Some(DataAccess::IndexRegister8(reg.high()))
                    ));
                    listing.push((
                        Mnemonic::Ld,
                        Some(DataAccess::IndexRegister8(reg.high())),
                        Some(DataAccess::Expression(0u8.into()))
                    ));
                }
                Some(listing)
            },

            Mnemonic::Srl
            | Mnemonic::Sra
            | Mnemonic::Sl1
            | Mnemonic::Sla
            | Mnemonic::Rl
            | Mnemonic::Rr
            | Mnemonic::Rlc
            | Mnemonic::Rrc
                if arg1.as_ref().map(|a| a.is_register16()).unwrap_or(false) =>
            {
                let reg16 = arg1.unwrap().get_register16().unwrap();
                let opcodes: &[(Mnemonic, Option<Register8>)] = match mnemonic {
                    Mnemonic::Srl => &[(Mnemonic::Srl, reg16.high()), (Mnemonic::Rr, reg16.low())],
                    Mnemonic::Sra => &[(Mnemonic::Sra, reg16.high()), (Mnemonic::Rr, reg16.low())],
                    Mnemonic::Sl1 => &[(Mnemonic::Sl1, reg16.low()), (Mnemonic::Rl, reg16.high())],
                    Mnemonic::Sla => &[(Mnemonic::Sla, reg16.low()), (Mnemonic::Rl, reg16.high())],
                    Mnemonic::Rr => &[(Mnemonic::Rr, reg16.high()), (Mnemonic::Rr, reg16.low())],
                    Mnemonic::Rl => &[(Mnemonic::Rl, reg16.low()), (Mnemonic::Rl, reg16.high())],
                    Mnemonic::Rlc => {
                        &[
                            (Mnemonic::Sla, reg16.high()),
                            (Mnemonic::Rl, reg16.low()),
                            (Mnemonic::Rr, reg16.high()),
                            (Mnemonic::Rlc, reg16.high())
                        ]
                    },
                    Mnemonic::Rrc => {
                        &[
                            (Mnemonic::Srl, reg16.high()),
                            (Mnemonic::Rr, reg16.low()),
                            (Mnemonic::Rl, reg16.high()),
                            (Mnemonic::Rrc, reg16.high())
                        ]
                    },
                    _ => unreachable!()
                };

                for (op, reg8) in opcodes {
                    listing.push((*op, reg8.map(DataAccess::Register8), None));
                }
                Some(listing)
            },

            _ => None
        }
    }

    #[inline]
    fn is_fake_instruction_from_access<DA: DataAccessElem>(
        mnemonic: Mnemonic,
        arg1: Option<&DA>,
        arg2: Option<&DA>,
        arg3: Option<Register8>
    ) -> bool {
        if arg3.is_some() {
            return false;
        }

        let arg1_is_de = arg1.as_ref().map(|a| a.is_register_de()).unwrap_or(false);
        let arg1_is_hl = arg1.as_ref().map(|a| a.is_register_hl()).unwrap_or(false);
        let arg1_is_reg16 = arg1.as_ref().map(|a| a.is_register16()).unwrap_or(false);
        let arg1_is_ix16 = arg1
            .as_ref()
            .map(|a| a.is_indexregister16())
            .unwrap_or(false);
        let arg1_is_reg16_or_ix16 = arg1_is_reg16 || arg1_is_ix16;

        let arg2_is_reg16 = arg2.as_ref().map(|a| a.is_register16()).unwrap_or(false);
        let arg2_is_sp = arg2.as_ref().map(|a| a.is_register_sp()).unwrap_or(false);
        let arg2_is_ix16 = arg2
            .as_ref()
            .map(|a| a.is_indexregister16())
            .unwrap_or(false);
        let arg2_is_hl_mem = arg2.as_ref().map(|a| a.is_address_in_hl()).unwrap_or(false);
        let _arg2_is_reg16_or_ix16 = arg2_is_reg16 || arg2_is_ix16;

        match mnemonic {
            Mnemonic::Add | Mnemonic::Adc | Mnemonic::Sbc => arg1_is_de && arg2_is_reg16,

            Mnemonic::Sub => (arg1_is_de || arg1_is_hl) && arg2_is_reg16,

            Mnemonic::Ld => {
                let fake_hl_sp = arg1_is_hl && arg2_is_sp;

                let fake_reg16_pair = arg1
                    .as_ref()
                    .map(|a| a.is_register_bc() || a.is_register_de() || a.is_register_hl())
                    .unwrap_or(false)
                    && arg2
                        .as_ref()
                        .map(|a| {
                            a.is_register_bc()
                                || a.is_register_de()
                                || a.is_register_hl()
                                || a.is_indexregister_with_index()
                        })
                        .unwrap_or(false);

                let fake_indexed_load = arg1
                    .as_ref()
                    .map(|a| a.is_indexregister_with_index())
                    .unwrap_or(false)
                    && arg2
                        .as_ref()
                        .map(|a| a.is_register_bc() || a.is_register_de() || a.is_register_hl())
                        .unwrap_or(false);

                let fake_indexed16_transfer = (arg1_is_hl && arg2_is_ix16)
                    || (arg1_is_ix16 && arg2_is_hl_mem)
                    || (arg1_is_ix16 && arg2_is_ix16)
                    || (arg1
                        .as_ref()
                        .map(|a| a.is_register_bc() || a.is_register_de() || a.is_register_hl())
                        .unwrap_or(false)
                        && arg2_is_ix16)
                    || (arg1_is_ix16
                        && arg2
                            .as_ref()
                            .map(|a| a.is_register_bc() || a.is_register_de() || a.is_register_hl())
                            .unwrap_or(false))
                    || (arg1
                        .as_ref()
                        .map(|a| a.is_register_bc() || a.is_register_de() || a.is_register_hl())
                        .unwrap_or(false)
                        && arg2_is_hl_mem)
                    || (arg1.as_ref().map(|a| a.is_address_in_hl()).unwrap_or(false)
                        && arg2
                            .as_ref()
                            .map(|a| a.is_register_bc() || a.is_register_de() || a.is_register_hl())
                            .unwrap_or(false));

                fake_hl_sp || fake_reg16_pair || fake_indexed_load || fake_indexed16_transfer
            },

            Mnemonic::Srl8 => arg1_is_reg16_or_ix16,

            Mnemonic::Srl
            | Mnemonic::Sra
            | Mnemonic::Sl1
            | Mnemonic::Sla
            | Mnemonic::Rl
            | Mnemonic::Rr
            | Mnemonic::Rlc
            | Mnemonic::Rrc => arg1_is_reg16,

            _ => false
        }
    }

    /// Returns all symbol names (labels) used in this token's expressions
    fn symbols(&self) -> std::collections::HashSet<String> {
        use std::collections::HashSet;

        let mut symbols = HashSet::new();

        // Extract from all expression fields - only call methods when appropriate
        // Use is_* checks before accessing fields to avoid panics

        if self.is_org() {
            symbols.extend(self.org_first().symbols());
            if let Some(expr) = self.org_second() {
                symbols.extend(expr.symbols());
            }
        }

        if self.is_equ() || self.is_set() {
            symbols.extend(self.equ_value().symbols());
        }

        if self.is_assign() {
            symbols.extend(self.assign_value().symbols());
        }

        // Only get mnemonic args if this is actually an opcode
        if self.is_opcode() {
            if let Some(arg1) = self.mnemonic_arg1()
                && let Some(expr) = arg1.get_expression()
            {
                symbols.extend(expr.symbols());
            }

            if let Some(arg2) = self.mnemonic_arg2()
                && let Some(expr) = arg2.get_expression()
            {
                symbols.extend(expr.symbols());
            }
        }

        if self.is_while() {
            symbols.extend(self.while_expr().symbols());
        }

        if self.is_switch() {
            symbols.extend(self.switch_expr().symbols());
            for (case_expr, ..) in self.switch_cases() {
                symbols.extend(case_expr.symbols());
            }
        }

        if self.is_iterate() {
            match self.iterate_values() {
                either::Either::Left(exprs) => {
                    for expr in exprs {
                        symbols.extend(expr.symbols());
                    }
                },
                either::Either::Right(expr) => {
                    symbols.extend(expr.symbols());
                }
            }
        }

        if self.is_for() {
            symbols.extend(self.for_start().symbols());
            symbols.extend(self.for_stop().symbols());
            if let Some(step) = self.for_step() {
                symbols.extend(step.symbols());
            }
        }

        if self.is_repeat_until() {
            symbols.extend(self.repeat_until_condition().symbols());
        }

        if self.is_rorg() {
            symbols.extend(self.rorg_expr().symbols());
        }

        if self.is_repeat() {
            symbols.extend(self.repeat_count().symbols());
            if let Some(start) = self.repeat_counter_start() {
                symbols.extend(start.symbols());
            }
            if let Some(step) = self.repeat_counter_step() {
                symbols.extend(step.symbols());
            }
        }

        if self.is_incbin() {
            symbols.extend(self.incbin_fname().symbols());
            if let Some(offset) = self.incbin_offset() {
                symbols.extend(offset.symbols());
            }
            if let Some(length) = self.incbin_length() {
                symbols.extend(length.symbols());
            }
        }

        if self.is_include() {
            symbols.extend(self.include_fname().symbols());
        }

        if self.is_db() || self.is_dw() || self.is_str() {
            for expr in self.data_exprs() {
                symbols.extend(expr.symbols());
            }
        }

        if self.is_run() {
            symbols.extend(self.run_expr().symbols());
        }

        if self.is_call_macro_or_build_struct() {
            // Macro name is a symbol reference
            symbols.insert(self.macro_call_name().to_string());
        }

        symbols
    }
}

#[cfg(test)]
mod tests {
    use super::ListingElement;
    use crate::{DataAccess, Mnemonic, Register8, Register16, Token};

    #[test]
    fn fake_predicate_and_expansion_are_aligned() {
        let fake_cases = vec![
            (
                Mnemonic::Add,
                Some(DataAccess::Register16(Register16::De)),
                Some(DataAccess::Register16(Register16::Bc)),
                None
            ),
            (
                Mnemonic::Adc,
                Some(DataAccess::Register16(Register16::De)),
                Some(DataAccess::Register16(Register16::Hl)),
                None
            ),
            (
                Mnemonic::Sbc,
                Some(DataAccess::Register16(Register16::De)),
                Some(DataAccess::Register16(Register16::Sp)),
                None
            ),
            (
                Mnemonic::Sub,
                Some(DataAccess::Register16(Register16::Hl)),
                Some(DataAccess::Register16(Register16::De)),
                None
            ),
            (
                Mnemonic::Ld,
                Some(DataAccess::Register16(Register16::De)),
                Some(DataAccess::Register16(Register16::Hl)),
                None
            ),
            (
                Mnemonic::Srl8,
                Some(DataAccess::Register16(Register16::De)),
                None,
                None
            ),
            (
                Mnemonic::Rlc,
                Some(DataAccess::Register16(Register16::Hl)),
                None,
                None
            ),
        ];

        let non_fake_cases = vec![
            (
                Mnemonic::Add,
                Some(DataAccess::Register16(Register16::Hl)),
                Some(DataAccess::Register16(Register16::De)),
                None
            ),
            (
                Mnemonic::Ld,
                Some(DataAccess::Register8(Register8::A)),
                Some(DataAccess::Register8(Register8::B)),
                None
            ),
            (
                Mnemonic::Srl8,
                Some(DataAccess::Register8(Register8::A)),
                None,
                None
            ),
            (
                Mnemonic::Rrc,
                Some(DataAccess::Register16(Register16::De)),
                None,
                Some(Register8::A)
            ),
        ];

        for (mnemonic, arg1, arg2, arg3) in fake_cases {
            let pred = <Token as ListingElement>::is_fake_instruction_from_access(
                mnemonic,
                arg1.as_ref(),
                arg2.as_ref(),
                arg3
            );
            let exp = <Token as ListingElement>::fake_to_listing_from_access(
                mnemonic,
                arg1.as_ref(),
                arg2.as_ref(),
                arg3
            )
            .is_some();

            assert!(pred, "expected fake predicate for {mnemonic:?}");
            assert!(exp, "expected fake expansion for {mnemonic:?}");
            assert_eq!(pred, exp, "predicate/expansion mismatch for {mnemonic:?}");
        }

        for (mnemonic, arg1, arg2, arg3) in non_fake_cases {
            let pred = <Token as ListingElement>::is_fake_instruction_from_access(
                mnemonic,
                arg1.as_ref(),
                arg2.as_ref(),
                arg3
            );
            let exp = <Token as ListingElement>::fake_to_listing_from_access(
                mnemonic,
                arg1.as_ref(),
                arg2.as_ref(),
                arg3
            )
            .is_some();

            assert!(!pred, "unexpected fake predicate for {mnemonic:?}");
            assert!(!exp, "unexpected fake expansion for {mnemonic:?}");
            assert_eq!(pred, exp, "predicate/expansion mismatch for {mnemonic:?}");
        }
    }
}

/// A listing is simply a list of things similar to token
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BaseListing<T: Clone + ListingElement> {
    /// Ordered list of the tokens
    pub(crate) listing: Vec<T>,
    /// Duration of the listing execution. Manually set by user
    pub(crate) duration: Option<usize>
}

impl<T: Clone + ListingElement> From<Vec<T>> for BaseListing<T> {
    fn from(listing: Vec<T>) -> Self {
        Self {
            listing,
            duration: None
        }
    }
}

impl<T: Clone + ListingElement> Deref for BaseListing<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.listing
    }
}

impl<T: Clone + ListingElement> DerefMut for BaseListing<T> {
    fn deref_mut(&mut self) -> &mut Vec<T> {
        &mut self.listing
    }
}

impl<T: Clone + ListingElement> Default for BaseListing<T> {
    fn default() -> Self {
        Self {
            listing: Vec::new(),
            duration: None
        }
    }
}

impl<T: Clone + Debug + ListingElement> From<T> for BaseListing<T> {
    fn from(token: T) -> Self {
        let mut lst = Self::default();
        lst.add(token);
        lst
    }
}

impl<T: Clone + ListingElement + Debug> FromIterator<T> for BaseListing<T> {
    fn from_iter<I: IntoIterator<Item = T>>(src: I) -> Self {
        Self::new_with(&src.into_iter().collect::<Vec<T>>())
    }
}

#[allow(missing_docs)]
impl<T: Clone + ListingElement + ::std::fmt::Debug> BaseListing<T> {
    /// Create an empty listing without duration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new  listing based on the provided Ts
    pub fn new_with(arg: &[T]) -> Self {
        Self {
            listing: arg.to_vec(),
            ..Default::default()
        }
    }

    /// Write access to listing. Should not exist but I do not know how to access to private firlds
    /// from trait implementation
    #[deprecated(note = "use listing_mut instead")]
    pub fn mut_listing(&mut self) -> &mut Vec<T> {
        &mut self.listing
    }

    pub fn listing_mut(&mut self) -> &mut Vec<T> {
        &mut self.listing
    }

    pub fn listing(&self) -> &[T] {
        &self.listing
    }

    /// Add a new token to the listing
    pub fn add(&mut self, token: T) {
        self.listing.push(token);
    }

    /// Consume another listing by injecting it
    pub fn inject_listing(&mut self, other: &Self) {
        self.listing.extend_from_slice(&other.listing);
    }

    /// Insert a copy of listing to the appropriate location
    pub fn insert_listing(&mut self, other: &Self, position: usize) {
        for (idx, token) in other.iter().enumerate() {
            self.listing.insert(idx + position, token.clone())
        }
    }

    pub fn set_duration(&mut self, duration: usize) {
        let duration = Some(duration);
        self.duration = duration;
    }

    pub fn duration(&self) -> Option<usize> {
        self.duration
    }

    /// Get the token at the required position
    pub fn get(&self, idx: usize) -> Option<&T> {
        self.listing.get(idx)
    }
}

// pub trait ListingTrait {
// type Element: ListingElement;
// fn as_slice(&self) -> &[Self::Element];
// }
//
// impl<T: ListingElement + Clone> ListingTrait for BaseListing<T> {
// type Element = T;
// fn as_slice(&self) -> &[Self::Element] {
// self.listing.as_ref()
// }
// }
