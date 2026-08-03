use std::borrow::Cow;

use cpclib_common::smallvec::SmallVec;
use cpclib_common::smol_str::SmolStr;

use crate::DataAccess;
use crate::tokens::expression::*;
use crate::tokens::instructions::*;
use crate::tokens::listing::*;

#[macro_export]
macro_rules! listing_element_impl_most_methods {
    () => {
        #[inline]
        fn is_run(&self) -> bool {
            match self.unwrapped() {
                Self::Run { .. } => true,
                _ => false
            }
        }

        #[inline]
        fn is_return(&self) -> bool {
            match self.unwrapped() {
                Self::Return(..) => true,
                _ => false
            }
        }

        #[inline]
        fn is_repeat_token(&self) -> bool {
            match self.unwrapped() {
                Self::RepeatToken { .. } => true,
                _ => false
            }
        }

        #[inline]
        fn repeat_token(&self) -> &Self {
            match self.unwrapped() {
                Self::RepeatToken { token, .. } => token,
                _ => unreachable!()
            }
        }

        #[inline]
        fn is_save(&self) -> bool {
            match self.unwrapped() {
                Self::Save { .. } => true,
                _ => false
            }
        }

        #[inline]
        fn is_breakpoint(&self) -> bool {
            match self.unwrapped() {
                Self::Breakpoint { .. } => true,
                _ => false
            }
        }

        #[inline]
        fn run_expr(&self) -> &Self::Expr {
            match self.unwrapped() {
                Self::Run(exp, _) => exp,
                _ => unreachable!()
            }
        }

        #[inline]
        fn return_value(&self) -> &Self::Expr {
            match self.unwrapped() {
                Self::Return(exp) => exp,
                _ => unreachable!()
            }
        }

        #[inline]
        fn equ_symbol(&self) -> &str {
            match self.unwrapped() {
                Self::Equ { label, .. } => label.as_str(),
                _ => unreachable!()
            }
        }

        #[inline]
        fn rorg_expr(&self) -> &Self::Expr {
            match self.unwrapped() {
                Self::Rorg(exp, _) => exp,
                _ => unreachable!()
            }
        }

        #[inline]
        fn equ_value(&self) -> &Self::Expr {
            match self.unwrapped() {
                Self::Equ { expr, .. } => expr,
                _ => unreachable!()
            }
        }
        #[inline]
        fn label_symbol(&self) -> &str {
            match self.unwrapped() {
                Self::Label(label, ..) => label.as_str(),
                _ => unreachable!()
            }
        }

        #[inline]
        fn assign_symbol(&self) -> &str {
            match self.unwrapped() {
                Self::Assign { label, .. } => label.as_str(),
                _ => unreachable!()
            }
        }

        #[inline]
        fn module_name(&self) -> &str {
            match self.unwrapped() {
                Self::Module(name, ..) => name.as_str(),
                _ => unreachable!()
            }
        }

        #[inline]
        fn mnemonic(&self) -> Option<&Mnemonic> {
            match self.unwrapped() {
                Self::OpCode(mnemonic, ..) => Some(mnemonic),
                _ => None
            }
        }

        #[inline]
        fn mnemonic_arg2(&self) -> Option<&Self::DataAccess> {
            match self.unwrapped() {
                Self::OpCode(_, _, arg2, _) => arg2.as_ref(),
                _ => None
            }
        }

        #[inline]
        fn mnemonic_arg1_mut(&mut self) -> Option<&mut Self::DataAccess> {
            match self.unwrapped_mut() {
                Self::OpCode(_, arg1, ..) => arg1.as_mut(),
                _ => None
            }
        }

        #[inline]
        fn iterate_counter_name(&self) -> &str {
            match self.unwrapped() {
                Self::Iterate(name, ..) => name.as_str(),
                _ => unreachable!()
            }
        }

        #[inline]
        fn iterate_values(&self) -> either::Either<&Vec<Self::Expr>, &Self::Expr> {
            match self.unwrapped() {
                Self::Iterate(_, values, ..) => values.as_ref(),
                _ => unreachable!()
            }
        }

        #[inline]
        fn mnemonic_arg2_mut(&mut self) -> Option<&mut Self::DataAccess> {
            match self.unwrapped_mut() {
                Self::OpCode(_, _, arg2, _) => arg2.as_mut(),
                _ => None
            }
        }

        #[inline]
        fn while_expr(&self) -> &Self::Expr {
            match self.unwrapped() {
                Self::While(expr, ..) => expr,
                _ => unreachable!()
            }
        }

        #[inline]
        fn assign_value(&self) -> &Self::Expr {
            match self.unwrapped() {
                Self::Assign { expr, .. } => expr,
                _ => unreachable!()
            }
        }

        #[inline]
        fn is_switch(&self) -> bool {
            match self.unwrapped() {
                Self::Switch(..) => true,
                _ => false
            }
        }

        #[inline]
        fn is_if(&self) -> bool {
            match self.unwrapped() {
                Self::If(..) => true,
                _ => false
            }
        }

        #[inline]
        fn is_repeat(&self) -> bool {
            match self.unwrapped() {
                Self::Repeat(..) => true,
                _ => false
            }
        }

        #[inline]
        fn is_for(&self) -> bool {
            match self.unwrapped() {
                Self::For { .. } => true,
                _ => false
            }
        }

        #[inline]
        fn is_directive(&self) -> bool {
            match self.unwrapped() {
                Self::OpCode(..) => false,
                _ => true
            }
        }

        #[inline]
        fn is_module(&self) -> bool {
            match self.unwrapped() {
                Self::Module(..) => true,
                _ => false
            }
        }

        #[inline]
        fn is_while(&self) -> bool {
            match self.unwrapped() {
                Self::While(..) => true,
                _ => false
            }
        }

        #[inline]
        fn is_iterate(&self) -> bool {
            match self.unwrapped() {
                Self::Iterate(..) => true,
                _ => false
            }
        }

        #[inline]
        fn is_repeat_until(&self) -> bool {
            match self.unwrapped() {
                Self::RepeatUntil(..) => true,
                _ => false
            }
        }

        #[inline]
        fn is_include(&self) -> bool {
            match self.unwrapped() {
                Self::Include(..) => true,
                _ => false
            }
        }

        #[inline]
        fn is_incbin(&self) -> bool {
            match self.unwrapped() {
                Self::Incbin { .. } => true,
                _ => false
            }
        }

        #[inline]
        fn is_call_macro_or_build_struct(&self) -> bool {
            match self.unwrapped() {
                Self::MacroCall(..) => true,
                _ => false
            }
        }

        #[inline]
        fn is_function_definition(&self) -> bool {
            match self.unwrapped() {
                Self::Function(..) => true,
                _ => false
            }
        }

        #[inline]
        fn is_crunched_section(&self) -> bool {
            match self.unwrapped() {
                Self::CrunchedSection(..) => true,
                _ => false
            }
        }

        #[inline]
        fn is_rorg(&self) -> bool {
            match self.unwrapped() {
                Self::Rorg(..) => true,
                _ => false
            }
        }

        #[inline]
        fn is_db(&self) -> bool {
            match self.unwrapped() {
                Self::Defb(..) => true,
                _ => false
            }
        }

        #[inline]
        fn is_dw(&self) -> bool {
            match self.unwrapped() {
                Self::Defw(..) => true,
                _ => false
            }
        }

        #[inline]
        fn is_str(&self) -> bool {
            match self.unwrapped() {
                Self::Str(..) => true,
                _ => false
            }
        }

        #[inline]

        fn is_set(&self) -> bool {
            self.is_assign()
        }

        #[inline]
        fn is_buildcpr(&self) -> bool {
            match self.unwrapped() {
                Self::BuildCpr { .. } => true,
                _ => false
            }
        }

        #[inline]
        fn is_assembler_control(&self) -> bool {
            match self.unwrapped() {
                Self::AssemblerControl(..) => true,
                _ => false
            }
        }

        #[inline]
        fn is_assert(&self) -> bool {
            match self.unwrapped() {
                Self::Assert(..) => true,
                _ => false
            }
        }

        #[inline]
        fn is_assign(&self) -> bool {
            match self.unwrapped() {
                Self::Assign { .. } => true,
                _ => false
            }
        }

        #[inline]
        fn is_comment(&self) -> bool {
            match self.unwrapped() {
                Self::Comment(..) => true,
                _ => false
            }
        }

        #[inline]
        fn comment(&self) -> &str {
            match self.unwrapped() {
                Self::Comment(content, ..) => content.as_str(),
                _ => unreachable!()
            }
        }

        #[inline]
        fn is_equ(&self) -> bool {
            match self.unwrapped() {
                Self::Equ { .. } => true,
                _ => false
            }
        }

        #[inline]
        fn is_label(&self) -> bool {
            match self.unwrapped() {
                Self::Label(..) => true,
                _ => false
            }
        }

        #[inline]
        fn is_org(&self) -> bool {
            match self.unwrapped() {
                Self::Org { .. } => true,
                _ => false
            }
        }

        #[inline]
        fn org_first(&self) -> &Self::Expr {
            match self.unwrapped() {
                Self::Org { val1, .. } => val1,
                _ => unreachable!()
            }
        }

        #[inline]
        fn org_second(&self) -> Option<&Self::Expr> {
            match self.unwrapped() {
                Self::Org { val2, .. } => val2.as_ref(),
                _ => unreachable!()
            }
        }

        #[inline]
        fn is_print(&self) -> bool {
            match self.unwrapped() {
                Self::Print(..) => true,
                _ => false
            }
        }

        #[inline]
        fn for_label(&self) -> &str {
            match self.unwrapped() {
                Self::For { label, .. } => label.as_ref(),
                _ => unreachable!()
            }
        }

        #[inline]
        fn for_start(&self) -> &Self::Expr {
            match self.unwrapped() {
                Self::For { start, .. } => start.as_ref(),
                _ => unreachable!()
            }
        }

        #[inline]
        fn for_stop(&self) -> &Self::Expr {
            match self.unwrapped() {
                Self::For { stop, .. } => stop.as_ref(),
                _ => unreachable!()
            }
        }

        #[inline]
        fn for_step(&self) -> Option<&Self::Expr> {
            match self.unwrapped() {
                Self::For { step, .. } => step.as_deref(),
                _ => unreachable!()
            }
        }

        #[inline]
        fn repeat_until_condition(&self) -> &Self::Expr {
            match self.unwrapped() {
                Self::RepeatUntil(cond, ..) => cond,
                _ => unreachable!()
            }
        }

        #[inline]
        fn repeat_count(&self) -> &Self::Expr {
            match self.unwrapped() {
                Self::Repeat(e, ..) => e,
                Self::RepeatToken { repeat, .. } => repeat,
                _ => unreachable!()
            }
        }

        #[inline]
        fn repeat_counter_name(&self) -> Option<&str> {
            match self.unwrapped() {
                Self::Repeat(_, _, counter_name, ..) => counter_name.as_ref().map(|c| c.as_str()),
                _ => unreachable!()
            }
        }

        #[inline]
        fn repeat_counter_start(&self) -> Option<&Self::Expr> {
            match self.unwrapped() {
                Self::Repeat(_, _, _, start, ..) => start.as_ref(),
                _ => unreachable!()
            }
        }

        #[inline]
        fn is_macro_definition(&self) -> bool {
            match self.unwrapped() {
                Self::Macro { .. } => true,
                _ => false
            }
        }

        #[inline]
        fn macro_definition_name(&self) -> &str {
            match self.unwrapped() {
                Self::Macro { name, .. } => name.as_str(),
                _ => unreachable!()
            }
        }

        #[inline]
        fn macro_definition_arguments(&self) -> SmallVec<[&str; 4]> {
            match self.unwrapped() {
                Self::Macro { params, .. } => params.iter().map(|a| a.as_str()).collect(),
                _ => unreachable!()
            }
        }

        #[inline]
        fn macro_definition_code(&self) -> &str {
            match self.unwrapped() {
                Self::Macro { content, .. } => content.as_str(),
                _ => unreachable!()
            }
        }

        #[inline]
        fn macro_definition_is_variadic(&self) -> bool {
            match self.unwrapped() {
                Self::Macro { has_variadic, .. } => *has_variadic,
                _ => unreachable!()
            }
        }

        #[inline]
        fn macro_call_name(&self) -> &str {
            match self.unwrapped() {
                Self::MacroCall(name, _) => name.as_str(),
                _ => panic!()
            }
        }

        #[inline]
        fn macro_call_arguments(&self) -> &[Self::MacroParam] {
            match self.unwrapped() {
                Self::MacroCall(_, args) => args,
                _ => panic!()
            }
        }

        #[inline]
        fn if_nb_tests(&self) -> usize {
            match self.unwrapped() {
                Self::If(tests, ..) => tests.len(),
                _ => panic!()
            }
        }

        #[inline]
        fn incbin_fname(&self) -> &Self::Expr {
            match self.unwrapped() {
                Self::Incbin { fname, .. } => fname,
                _ => unreachable!()
            }
        }

        #[inline]
        fn incbin_offset(&self) -> Option<&Self::Expr> {
            match self.unwrapped() {
                Self::Incbin { offset, .. } => offset.as_ref(),
                _ => unreachable!()
            }
        }

        #[inline]
        fn incbin_length(&self) -> Option<&Self::Expr> {
            match self.unwrapped() {
                Self::Incbin { length, .. } => length.as_ref(),
                _ => unreachable!()
            }
        }

        #[inline]
        fn incbin_transformation(&self) -> &BinaryTransformation {
            match self.unwrapped() {
                Self::Incbin { transformation, .. } => transformation,
                _ => unreachable!()
            }
        }

        #[inline]
        fn include_fname(&self) -> &Self::Expr {
            match self.unwrapped() {
                Self::Include(fname, ..) => fname,
                _ => unreachable!()
            }
        }

        #[inline]
        fn include_namespace(&self) -> Option<&str> {
            match self.unwrapped() {
                Self::Include(_, module, _) => module.as_ref().map(|s| s.as_str()),
                _ => unreachable!()
            }
        }

        #[inline]
        fn include_once(&self) -> bool {
            match self.unwrapped() {
                Self::Include(_, _, once) => *once,
                _ => unreachable!()
            }
        }

        fn function_definition_name(&self) -> &str {
            match self.unwrapped() {
                Self::Function(name, ..) => name.as_str(),
                _ => unreachable!()
            }
        }

        fn function_definition_params(&self) -> SmallVec<[&str; 4]> {
            match self.unwrapped() {
                Self::Function(_, params, _) => params.iter().map(|v| v.as_str()).collect(),
                _ => unreachable!()
            }
        }

        fn crunched_section_kind(&self) -> &CrunchType {
            match self.unwrapped() {
                Self::CrunchedSection(kind, ..) => kind,
                _ => unreachable!()
            }
        }

        fn switch_expr(&self) -> &Self::Expr {
            match self.unwrapped() {
                Self::Switch(expr, ..) => expr,
                _ => unreachable!()
            }
        }

        fn data_exprs(&self) -> &[Self::Expr] {
            match self.unwrapped() {
                Self::Defb(e, ..) | Self::Defw(e, ..) | Self::Str(e, ..) => e,
                _ => unreachable!()
            }
        }
    };
}

impl ListingElement for Token {
    type AssemblerControlCommand = StandardAssemblerControlCommand;
    // type Element = Token;
    type DataAccess = DataAccess;
    type Expr = Expr;
    type MacroParam = MacroParam;
    type TestKind = TestKind;

    listing_element_impl_most_methods!();

    //    type Listing = BaseListing<Token>;

    fn to_token(&self) -> Cow<'_, Token> {
        Cow::Borrowed(self)
    }

    fn is_warning(&self) -> bool {
        matches!(self, Self::WarningWrapper(..))
    }

    fn warning_token(&self) -> &Self {
        match self {
            Self::WarningWrapper(inner, _) => inner,
            _ => unreachable!()
        }
    }

    fn warning_message(&self) -> &str {
        match self {
            Self::WarningWrapper(_, msg) => msg.as_str(),
            _ => unreachable!()
        }
    }

    fn module_listing(&self) -> &[Self] {
        match self.unwrapped() {
            Token::Module(_, lst, ..) => lst,
            _ => unreachable!()
        }
    }

    fn while_listing(&self) -> &[Self] {
        match self.unwrapped() {
            Token::While(_, lst, ..) => lst,
            _ => unreachable!()
        }
    }

    fn mnemonic_arg1(&self) -> Option<&Self::DataAccess> {
        match self.unwrapped() {
            Token::OpCode(_, arg1, ..) => arg1.as_ref(),
            _ => None
        }
    }

    fn iterate_listing(&self) -> &[Self] {
        match self.unwrapped() {
            Self::Iterate(_, _, listing, ..) => listing,
            _ => unreachable!()
        }
    }

    fn for_listing(&self) -> &[Self] {
        match self.unwrapped() {
            Self::For { listing, .. } => listing.as_ref(),
            _ => unreachable!()
        }
    }

    fn repeat_until_listing(&self) -> &[Self] {
        match self.unwrapped() {
            Self::RepeatUntil(_, code, ..) => code,
            _ => unreachable!()
        }
    }

    fn repeat_listing(&self) -> &[Self] {
        match self.unwrapped() {
            Self::Repeat(_, listing, ..) => listing,
            _ => unreachable!()
        }
    }

    fn if_test(&self, idx: usize) -> (&Self::TestKind, &[Self]) {
        match self.unwrapped() {
            Self::If(tests, ..) => {
                let data = &tests[idx];
                (&data.0, &data.1)
            },
            _ => panic!()
        }
    }

    #[inline]
    fn if_else(&self) -> Option<&[Self]> {
        match self.unwrapped() {
            Self::If(_, r#else, ..) => r#else.as_ref().map(|l| l.as_ref()),
            _ => panic!()
        }
    }

    fn function_definition_inner(&self) -> &[Self] {
        match self.unwrapped() {
            Self::Function(_, _, inner) => inner,
            _ => unreachable!()
        }
    }

    fn crunched_section_listing(&self) -> &[Self] {
        match self.unwrapped() {
            Self::CrunchedSection(_, lst) => lst,
            _ => unreachable!()
        }
    }

    fn rorg_listing(&self) -> &[Self] {
        match self.unwrapped() {
            Self::Rorg(_, lst) => lst,
            _ => unreachable!()
        }
    }

    fn is_confined(&self) -> bool {
        false // TODO implement properly
    }

    fn confined_listing(&self) -> &[Self] {
        todo!()
    }

    fn switch_cases(&self) -> Box<dyn Iterator<Item = (&Self::Expr, &[Self], bool)> + '_> {
        match self.unwrapped() {
            Self::Switch(_, cases, ..) => {
                Box::new(cases.iter().map(|c| (&c.0, c.1.as_slice(), c.2)))
            },
            _ => unreachable!()
        }
    }

    fn switch_default(&self) -> Option<&[Self]> {
        match self.unwrapped() {
            Self::Switch(_, _, default, ..) => default.as_ref().map(|l| l.as_slice()),
            _ => unreachable!()
        }
    }

    fn repeat_counter_step(&self) -> Option<&Self::Expr> {
        match self.unwrapped() {
            Self::Repeat(_, _, _, step) => step.as_ref(),
            _ => unreachable!()
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
}

/// Standard listing is a specific implementation
pub type Listing = BaseListing<Token>;

// Set of methods that do not have additional dependencies
impl Listing {
    /// Add a new label to the listing
    pub fn add_label<S: Into<SmolStr>>(&mut self, label: S) {
        self.listing_mut().push(Token::Label(label.into()));
    }

    /// Add a new comment to the listing
    pub fn add_comment<S: Into<String>>(&mut self, comment: S) {
        self.listing_mut().push(Token::Comment(comment.into()));
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
