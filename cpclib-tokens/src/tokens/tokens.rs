use std::borrow::Cow;

use cpclib_common::smallvec::SmallVec;
use either::Either;

use crate::tokens::expression::*;
use crate::tokens::instructions::*;
use crate::tokens::listing::*;
use crate::DataAccess;

impl ListingElement for Token {
    type AssemblerControlCommand = StandardAssemblerControlCommand;
    // type Element = Token;
    type DataAccess = DataAccess;
    type Expr = Expr;
    type MacroParam = MacroParam;
    type TestKind = TestKind;

    //    type Listing = BaseListing<Token>;

    fn to_token(&self) -> Cow<Token> {
        Cow::Borrowed(self)
    }

    fn is_equ(&self) -> bool {
        match self {
            Token::Equ { .. } => true,
            _ => false
        }
    }

    fn equ_symbol(&self) -> &str {
        match self {
            Token::Equ { label, .. } => label.as_str(),
            _ => unreachable!()
        }
    }

    fn equ_value(&self) -> &Self::Expr {
        match self {
            Token::Equ { expr, .. } => expr,
            _ => unreachable!()
        }
    }

    fn is_warning(&self) -> bool {
        false
    }

    fn warning_token(&self) -> &Self {
        unreachable!()
    }

    fn warning_message(&self) -> &str {
        unreachable!()
    }

    fn is_module(&self) -> bool {
        match self {
            Token::Module(..) => true,
            _ => false
        }
    }

    fn module_listing(&self) -> &[Self] {
        match self {
            Token::Module(_, lst, ..) => lst,
            _ => unreachable!()
        }
    }

    fn module_name(&self) -> &str {
        match self {
            Token::Module(name, ..) => name.as_str(),
            _ => unreachable!()
        }
    }

    fn is_while(&self) -> bool {
        match self {
            Token::While(..) => true,
            _ => false
        }
    }

    fn while_expr(&self) -> &Self::Expr {
        match self {
            Token::While(expr, ..) => expr,
            _ => unreachable!()
        }
    }

    fn while_listing(&self) -> &[Self] {
        match self {
            Token::While(_, lst, ..) => lst,
            _ => unreachable!()
        }
    }

    fn mnemonic(&self) -> Option<&Mnemonic> {
        match self {
            Token::OpCode(ref mnemonic, ..) => Some(mnemonic),
            _ => None
        }
    }

    fn mnemonic_arg1(&self) -> Option<&Self::DataAccess> {
        match self {
            Token::OpCode(_, ref arg1, ..) => arg1.as_ref(),
            _ => None
        }
    }

    fn mnemonic_arg2(&self) -> Option<&Self::DataAccess> {
        match self {
            Token::OpCode(_, _, ref arg2, _) => arg2.as_ref(),
            _ => None
        }
    }

    fn mnemonic_arg1_mut(&mut self) -> Option<&mut Self::DataAccess> {
        match self {
            Token::OpCode(_, ref mut arg1, ..) => arg1.as_mut(),
            _ => None
        }
    }

    fn mnemonic_arg2_mut(&mut self) -> Option<&mut Self::DataAccess> {
        match self {
            Token::OpCode(_, _, ref mut arg2, _) => arg2.as_mut(),
            _ => None
        }
    }

    fn is_directive(&self) -> bool {
        match self {
            Self::OpCode(..) => false,
            _ => true
        }
    }

    fn is_iterate(&self) -> bool {
        match self {
            Self::Iterate(..) => true,
            _ => false
        }
    }

    fn iterate_listing(&self) -> &[Self] {
        match self {
            Self::Iterate(_, _, listing, ..) => listing,
            _ => unreachable!()
        }
    }

    fn iterate_counter_name(&self) -> &str {
        match self {
            Self::Iterate(name, ..) => name.as_str(),
            _ => unreachable!()
        }
    }

    fn iterate_values(&self) -> either::Either<&Vec<Self::Expr>, &Self::Expr> {
        match self {
            Self::Iterate(_, values, ..) => Either::Left(values),
            _ => unreachable!()
        }
    }

    fn is_for(&self) -> bool {
        match self {
            Self::For { .. } => true,
            _ => false
        }
    }

    fn for_listing(&self) -> &[Self] {
        match self {
            Self::For { listing, .. } => listing,
            _ => unreachable!()
        }
    }

    fn for_label(&self) -> &str {
        match self {
            Self::For { label, .. } => label.as_ref(),
            _ => unreachable!()
        }
    }

    fn for_start(&self) -> &Self::Expr {
        match self {
            Self::For { start, .. } => start,
            _ => unreachable!()
        }
    }

    fn for_stop(&self) -> &Self::Expr {
        match self {
            Self::For { stop, .. } => stop,
            _ => unreachable!()
        }
    }

    fn for_step(&self) -> Option<&Self::Expr> {
        match self {
            Self::For { step, .. } => step.as_ref(),
            _ => unreachable!()
        }
    }

    fn is_repeat_until(&self) -> bool {
        match self {
            Self::RepeatUntil(..) => true,
            _ => false
        }
    }

    fn repeat_until_listing(&self) -> &[Self] {
        match self {
            Self::RepeatUntil(_, code, ..) => code,
            _ => unreachable!()
        }
    }

    fn repeat_until_condition(&self) -> &Self::Expr {
        match self {
            Self::RepeatUntil(cond, ..) => cond,
            _ => unreachable!()
        }
    }

    fn is_repeat(&self) -> bool {
        match self {
            Self::Repeat(..) => true,
            _ => false
        }
    }

    fn repeat_listing(&self) -> &[Self] {
        match self {
            Self::Repeat(_, listing, ..) => listing,
            _ => unreachable!()
        }
    }

    fn repeat_count(&self) -> &Self::Expr {
        match self {
            Self::Repeat(e, ..) => e,
            _ => unreachable!()
        }
    }

    fn repeat_counter_name(&self) -> Option<&str> {
        match self {
            Self::Repeat(_, _, counter_name, ..) => counter_name.as_ref().map(|c| c.as_str()),
            _ => unreachable!()
        }
    }

    fn repeat_counter_start(&self) -> Option<&Self::Expr> {
        match self {
            Self::Repeat(_, _, _, start) => start.as_ref(),
            _ => unreachable!()
        }
    }

    fn is_macro_definition(&self) -> bool {
        match self {
            Self::Macro { .. } => true,
            _ => false
        }
    }

    fn macro_definition_name(&self) -> &str {
        match self {
            Self::Macro { name, .. } => name.as_str(),
            _ => unreachable!()
        }
    }

    fn macro_definition_arguments(&self) -> SmallVec<[&str; 4]> {
        match self {
            Self::Macro { params, .. } => params.iter().map(|a| a.as_str()).collect(),
            _ => unreachable!()
        }
    }

    fn macro_definition_code(&self) -> &str {
        match self {
            Self::Macro { content, .. } => content.as_str(),
            _ => unreachable!()
        }
    }

    fn macro_call_name(&self) -> &str {
        match self {
            Token::MacroCall(name, _) => name.as_str(),
            _ => panic!()
        }
    }

    fn macro_call_arguments(&self) -> &[Self::MacroParam] {
        match self {
            Token::MacroCall(_, args) => args,
            _ => panic!()
        }
    }

    fn is_if(&self) -> bool {
        match self {
            Token::If(..) => true,
            _ => false
        }
    }

    fn if_nb_tests(&self) -> usize {
        match self {
            Self::If(tests, ..) => tests.len(),
            _ => panic!()
        }
    }

    fn if_test(&self, idx: usize) -> (&Self::TestKind, &[Self]) {
        match self {
            Self::If(tests, ..) => {
                let data = &tests[idx];
                (&data.0, &data.1)
            },
            _ => panic!()
        }
    }

    fn if_else(&self) -> Option<&[Self]> {
        match self {
            Self::If(_, r#else) => r#else.as_ref().map(|l| l.as_ref()),
            _ => panic!()
        }
    }

    fn is_include(&self) -> bool {
        match self {
            Token::Include(..) => true,
            _ => false
        }
    }

    fn is_incbin(&self) -> bool {
        match self {
            Token::Incbin { .. } => true,
            _ => false
        }
    }

    fn incbin_fname(&self) -> &str {
        match self {
            Self::Incbin { fname, .. } => fname.as_ref(),
            _ => unreachable!()
        }
    }

    fn incbin_offset(&self) -> Option<&Self::Expr> {
        match self {
            Self::Incbin { offset, .. } => offset.as_ref(),
            _ => unreachable!()
        }
    }

    fn incbin_length(&self) -> Option<&Self::Expr> {
        match self {
            Self::Incbin { length, .. } => length.as_ref(),
            _ => unreachable!()
        }
    }

    fn incbin_transformation(&self) -> &BinaryTransformation {
        match self {
            Self::Incbin { transformation, .. } => transformation,
            _ => unreachable!()
        }
    }

    fn include_fname(&self) -> &str {
        match self {
            Self::Include(fname, ..) => fname.as_ref(),
            _ => unreachable!()
        }
    }

    fn include_namespace(&self) -> Option<&str> {
        match self {
            Self::Include(_, module, _) => module.as_ref().map(|s| s.as_str()),
            _ => unreachable!()
        }
    }

    fn include_once(&self) -> bool {
        match self {
            Self::Include(_, _, once) => *once,
            _ => unreachable!()
        }
    }

    fn is_call_macro_or_build_struct(&self) -> bool {
        match self {
            Self::MacroCall(..) => true,
            _ => false
        }
    }

    fn is_function_definition(&self) -> bool {
        match self {
            Self::Function(..) => true,
            _ => false
        }
    }

    fn function_definition_name(&self) -> &str {
        match self {
            Self::Function(name, ..) => name.as_str(),
            _ => unreachable!()
        }
    }

    fn function_definition_params(&self) -> SmallVec<[&str; 4]> {
        match self {
            Self::Function(_, params, _) => params.iter().map(|v| v.as_str()).collect(),
            _ => unreachable!()
        }
    }

    fn function_definition_inner(&self) -> &[Self] {
        match self {
            Self::Function(_, _, inner) => inner,
            _ => unreachable!()
        }
    }

    fn is_crunched_section(&self) -> bool {
        match self {
            Self::CrunchedSection(..) => true,
            _ => false
        }
    }

    fn crunched_section_listing(&self) -> &[Self] {
        match self {
            Self::CrunchedSection(_, lst) => lst,
            _ => unreachable!()
        }
    }

    fn crunched_section_kind(&self) -> &CrunchType {
        match self {
            Self::CrunchedSection(kind, _) => kind,
            _ => unreachable!()
        }
    }

    fn is_rorg(&self) -> bool {
        match self {
            Self::Rorg(..) => true,
            _ => false
        }
    }

    fn rorg_listing(&self) -> &[Self] {
        match self {
            Self::Rorg(_, lst) => lst,
            _ => unreachable!()
        }
    }

    fn rorg_expr(&self) -> &Self::Expr {
        match self {
            Self::Rorg(exp, _) => exp,
            _ => unreachable!()
        }
    }

    fn is_confined(&self) -> bool {
        false // TODO implement properly
    }

    fn confined_listing(&self) -> &[Self] {
        todo!()
    }

    fn is_switch(&self) -> bool {
        match self {
            Self::Switch(..) => true,
            _ => false
        }
    }

    fn switch_expr(&self) -> &Self::Expr {
        match self {
            Self::Switch(expr, ..) => expr,
            _ => unreachable!()
        }
    }

    fn switch_cases(&self) -> Box<dyn Iterator<Item = (&Self::Expr, &[Self], bool)> + '_> {
        match self {
            Self::Switch(_, cases, ..) => {
                Box::new(cases.iter().map(|c| (&c.0, c.1.as_slice(), c.2)))
            },
            _ => unreachable!()
        }
    }

    fn switch_default(&self) -> Option<&[Self]> {
        match self {
            Self::Switch(_, _, default, ..) => default.as_ref().map(|l| l.as_slice()),
            _ => unreachable!()
        }
    }

    fn is_db(&self) -> bool {
        match self {
            Self::Defb(..) => true,
            _ => false
        }
    }

    fn is_dw(&self) -> bool {
        match self {
            Self::Defw(..) => true,
            _ => false
        }
    }

    fn is_str(&self) -> bool {
        match self {
            Self::Str(..) => true,
            _ => false
        }
    }

    fn data_exprs(&self) -> &[Self::Expr] {
        match self {
            Self::Defb(e) | Self::Defw(e) | Self::Str(e) => e,
            _ => unreachable!()
        }
    }

    fn is_set(&self) -> bool {
        match self {
            Self::Assign { .. } => true,
            _ => false
        }
    }

    fn is_comment(&self) -> bool {
        match self {
            Self::Comment(..) => true,
            _ => false
        }
    }

    fn is_assign(&self) -> bool {
        match self {
            Self::Assign { .. } => true,
            _ => false
        }
    }

    fn is_org(&self) -> bool {
        match self {
            Self::Org { .. } => true,
            _ => false
        }
    }

    fn repeat_counter_step(&self) -> Option<&Self::Expr> {
        todo!()
    }

    fn is_assembler_control(&self) -> bool {
        match self {
            Self::AssemblerControl(..) => true,
            _ => false
        }
    }

    fn assembler_control_command(&self) -> &Self::AssemblerControlCommand {
        todo!()
    }

    fn assembler_control_get_max_passes(&self) -> Option<&Self::Expr> {
        todo!()
    }

    fn assembler_control_get_listing(&self) -> &[Self] {
        todo!()
    }

    fn macro_flavor(&self) -> AssemblerFlavor {
        todo!()
    }

    fn is_buildcpr(&self) -> bool {
        match self {
            Self::BuildCpr => true,
            _ => false
        }
    }
}

/// Standard listing is a specific implementation
pub type Listing = BaseListing<Token>;

// Set of methods that do not have additional dependencies
impl Listing {
    /// Add a new label to the listing
    pub fn add_label(&mut self, label: &str) {
        self.listing_mut().push(Token::Label(label.into()));
    }

    /// Add a new comment to the listing
    pub fn add_comment(&mut self, comment: &str) {
        self.listing_mut()
            .push(Token::Comment(String::from(comment)));
    }

    /// Add a list of bytes to the listing
    pub fn add_bytes(&mut self, bytes: &[u8]) {
        let exp = bytes
            .iter()
            .map(|pu8| Expr::Value(i32::from(*pu8)))
            .collect::<Vec<_>>();
        let tok = Token::Defb(exp);
        self.push(tok);
    }

    // Macro can have labels like @stuff.
    // They must be replaced by unique values to be sure they can be called several times
    // pub fn fix_local_macro_labels_with_seed(&mut self, seed: usize) {
    // self.iter_mut()
    // .for_each(|e| e.fix_local_macro_labels_with_seed(seed));
    //
    //     dbg!(&self);
    // }

}

impl From<&[u8]> for Listing {
    fn from(src: &[u8]) -> Listing {
        let mut new = Listing::default();
        new.add_bytes(src);
        new
    }
}
