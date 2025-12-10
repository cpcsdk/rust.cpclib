use core::fmt::Debug;
use std::borrow::Cow;
use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};

use cpclib_common::smallvec::SmallVec;

use crate::{
    AssemblerControlCommand, AssemblerFlavor, BinaryTransformation, CrunchType, DataAccessElem,
    ExprElement, MacroParamElement, Mnemonic, TestKindElement
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

    fn is_print(&self) -> bool;
    fn is_breakpoint(&self) -> bool;
    fn is_save(&self) -> bool;

    fn to_token(&self) -> Cow<'_, crate::Token>;
    fn starts_with_label(&self) -> bool {
        self.is_label() || self.is_assign() || self.is_equ() || self.is_set()
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
        let mut new = Self::default();
        new.listing = arg.to_vec();
        new
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
