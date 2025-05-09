#![feature(let_chains)]

use std::{collections::{BTreeMap, BTreeSet, HashMap, HashSet}, fmt::write, ops::{Deref, DerefMut}};

use cpclib_asm::{dec_l, inc_l, BaseListing, Expr, IfBuilder, Listing, ListingBuilder, ListingExt, ListingFromStr, ListingSelector, Register8, TestKind};
use cpclib_image::convert::{SpriteEncoding, SpriteOutput};
use bon::Builder;
use smol_str::SmolStr;
use itertools::Itertools;

/// The action handled by the code
#[derive(Default)]
pub enum RoutineAction {
    #[default]
    DrawPixelMaskedSprite,
    SaveBackgroundAndDrawPixelMaskedSprite,
    RestoreBackground
}   


#[derive(Default)]
pub struct RegistersStore {
    regs: HashMap<Register8, u8>,
    available_regs: Vec<Register8>
}

impl RegistersStore {
    pub fn new(regs: &[Register8]) -> Self {
        Self {
            regs: Default::default(),
            available_regs: regs.to_vec()
        }
    }

    pub fn listing(&self) -> Listing {
        let mut lst = ListingBuilder::default();
        for (r, v) in self.regs.iter() {
            lst = lst.ld_r8_expr(*r, *v);
        }
        lst.build()
    }

    pub fn register_for(&self, val: u8) -> Option<Register8> {
        self.regs.iter()
            .find(|(r, v)| **v == val)
            .map(|(r, v)| r.clone())
    }

    pub fn set(&mut self, r: Register8, v: u8) -> &mut Self {
        assert!(!self.available_regs.contains(&r));
        self.regs.insert(r, v);
        self
    }

    pub fn next_available_regs(&mut self) -> Option<Register8> {
        self.available_regs.pop()
    }

    pub fn has_available_regs(&self) -> bool {
        !self.available_regs.is_empty()
    }
}



#[derive(Builder)]
//#[builder(start_fn = with_header)]
#[builder(on(String, into))]                                                
#[builder(on(SmolStr, into))]                                                
pub struct Compiler {
  //  #[builder(start_fn)]
    header_comment: Option<String>,
   // #[builder(start_fn)]
    header_label: Option<SmolStr>,

    /// The listing of the final code
    #[builder(default)]
    lst: Listing,

    /// Utility to handle line changes
    bc26: Bc26,

    #[builder(default)]
    regs: RegistersStore
}


impl Deref for Compiler {
    type Target = Listing;
    fn deref(&self) -> &Self::Target {
        &self.lst
    }
}

impl DerefMut for Compiler {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.lst
    }
}

impl Compiler {
    pub fn build_stats(spr: &SpriteOutput, msk: &SpriteOutput) -> BTreeMap<u8, usize> {
        let mut set: BTreeMap<u8, usize> = Default::default();
        for b in spr.data().iter().zip(msk.data().iter())
            .filter_map(|(&b, &m)| if m!= 0xff && m!= 0x00 {
                Some(b)
            } else {
                None
            }) {
            if set.contains_key(&b) {
                *set.get_mut(&b).unwrap() += 1;
            } else {
                set.insert(b, 1);
            }
        }

        set

    }

    pub fn compile(mut self, spr: &SpriteOutput, msk: &SpriteOutput) -> Listing {
        assert_eq!(spr.bytes_width(), msk.bytes_width());
        assert_eq!(spr.height(), msk.height());
        assert_eq!(spr.encoding(), msk.encoding());


        let stats = Self::build_stats(spr, msk);
        dbg!(&stats);

        let mut retained = stats.into_iter().filter(|(k,v)| *v > 3)
                .sorted_by_key(|(k,v)| *v)
                .collect_vec();
        dbg!(&retained);

        // prefetch registers with most used values
        self.regs = RegistersStore::new(&[Register8::D, Register8::E, Register8::B, Register8::C]);
        while let Some((v, _)) = retained.pop() && self.regs.has_available_regs() {
            let r = self.regs.next_available_regs().unwrap();
            self.regs.set(r, v);
        }

        let width = spr.bytes_width();

        self.emit_header();
        let mut write_cursor = 0;
        for (idx, line) in spr.data().iter().cloned().zip(msk.data().iter().cloned())
            .chunks(width)
            .into_iter()
            .enumerate() {
            write_cursor = self.emit_line(idx, line, write_cursor)
        }

        self.emit_footer();
        self.lst
    } 

    fn emit_line(&mut self, line_idx: usize, line: impl Iterator<Item=(u8, u8)>, mut write_cursor: u16) -> u16 {
        let mut nb_moves = 0;
        self.emit_line_header(line_idx);

        let mut lst = ListingBuilder::default();

        let line = line.collect_vec();
        let nb_steps = line.len() as u16;
        for (read_idx, (pixs, mask)) in line.into_iter().enumerate() {

            let read_cursor = read_idx as u16;
            let will_write_on_screen = mask != 0xff;

            // we need to lazily  move the write buffer when some modifications have to be done
            if will_write_on_screen {
                assert!(self.use_8bits_addresses_handling());

                // get the number of displacement since last screen update
                let local_moves = (read_cursor as i32) - (write_cursor as i32);

                // select the fastest way to do it
                let chosen: Listing = match local_moves {
                    0 => Listing::new(),
                    1 => inc_l().into(),
                    -1 => dec_l().into(),
                    local_moves => {
                        dbg!(local_moves);
                        let choice1: Listing = ListingBuilder::default()
                        .repeat(
                            local_moves.abs(), 
                            if local_moves > 0 {inc_l()} else {dec_l()}
                        )
                        .build();
                        let choice2: Listing = ListingBuilder::default()
                            .ld_a_expr(local_moves)
                            .add_l()
                            .ld_l_a()
                            .build();

                        ListingSelector::default()
                            .add(choice1)
                            .add(choice2)
                            .select()
                    }
                };
                
                               
                // add it to the generated code
                lst = lst
                    .comment(format!("Move of {} bytes", local_moves))
                    .extend(chosen); // inject the fastest one

                write_cursor = read_cursor;
            }

            // properly handle the byte
            match mask {
                0x00 => {
                    // no background pixels are kept
                    lst = lst
                        .comment("No masking here")
                        .ld_mem_hl_expr(pixs);
                },

                0xff => {
                    // all background pixels are kept
                    lst = lst.comment("Nothing shown here");
                },

                mask => {

                    // masking is necessary
                    let comment = if pixs != 0 {
                        "Masked byte"
                    } else {
                        "masked byte BUT no bit to set"
                    };

                    // mask screen byte
                    lst = lst
                        .comment(comment)
                        .ld_a_mem_hl()
                        .and_expr(mask);

                    // request additional bits
                    match pixs {
                        0 => {}, // nothing to draw
                        1 => {lst = lst.inc_a();} // faster/maller than OR 1
                        val => {
                            if let Some(reg) = self.regs.register_for(val) {
                                lst = lst.or_r8(reg);
                            } else {
                                lst = lst.or_expr(pixs);
                            }
                        }
                    }

                    // save
                    lst = lst.ld_mem_hl_a();

                },

            };

            if nb_steps-1 == read_cursor {
                lst = lst.comment("End of line");
            }  
        }

        self.inject_listing(&lst.build());

        self.emit_line_footer();
        write_cursor
    }

    fn emit_line_header(&mut self, idx: usize) {
        self.add_comment(format!("> Handle line {idx}"));
        if idx == 0 {
            self.add_comment("  HL already contains the destination address");
        } else {
            self.emit_compute_next_line_address();
        }

    }

    fn emit_line_footer(&mut self) {

    }
/* 
    fn emit_move_to_line_start(&mut self, step: u16) {
        let lst = if self.use_8bits_addresses_handling() {
            ListingBuilder::default()
                .comment("Move from end of line to beginning of the same line")
                .ld_a_expr(Expr::Value(step as i32).neg())
                .add_l()
                .ld_l_a()
                .build()
        } else {
            unimplemented!()
        };

        self.inject_listing(&lst);
    }
*/
    pub fn use_8bits_addresses_handling(&self) -> bool {
        self.bc26.use_8bits_addresses_handling().unwrap()
    }

    fn emit_compute_next_line_address(&mut self) {
        self.add_comment("Compute the address of the next line");
        let execute = self.bc26.clone().execute();
        self.inject_listing(&execute);
    }

    fn emit_header(&mut self) {
        if let Some(comment) = self.header_comment.take() {
            self.add_comment(comment);
        }
        if let Some(label) = self.header_label.take() {
            self.add_label(label);
        }
        self.lst.inject_listing(&self.regs.listing());

    }

    fn emit_footer(&mut self) {
        self.add(cpclib_asm::ret());


        if let Some(bc26_routine) = self.bc26.routine() {
            let bc26_label = self.bc26.label();
            let r#if = IfBuilder::default()
            .condition(
                TestKind::ifndef(bc26_label),
                bc26_routine
            ).build();
            self.add(r#if);
        }

    }
}


// https://roudoudou.com/AmstradCPC/programmationAssembleurZ80Ecran.html
#[derive(Clone, Copy)]
pub enum Bc26 {
    Compute16KbC000{r1: u8},
    Compute16KbUniversal{r1: u8},
}

impl Bc26 {
    pub fn new_universal_16k(r1: u8) -> Self {
        Self::Compute16KbUniversal { r1 }
    }

    pub fn new_c000_16k(r1: u8) -> Self {
        Self::Compute16KbC000 { r1 }
    }
}

impl Bc26 {
    pub fn label(&self) -> String {
        match self {
            Bc26::Compute16KbC000 { r1 } => Self::c000_routine_label(*r1),
            Bc26::Compute16KbUniversal { r1 } => Self::universal_routine_label(*r1),
        }
    }

    pub fn routine(&self) -> Option<Listing> {
        match self {
            Bc26::Compute16KbC000 { r1 } => Some(Self::c000_routine(*r1)),
            Bc26::Compute16KbUniversal { r1 } => Some(Self::universal_routine(*r1)),
        }
    }

    pub fn execute(&self) -> Listing {
        match self {
            Bc26::Compute16KbC000 { r1 } | Bc26::Compute16KbUniversal { r1 } => {
                ListingBuilder::default()
                    .call(self.label())
                    .build()
            }
        }
    }

    pub fn r1(&self) -> Option<u8> {
        match self {
            Self::Compute16KbC000 { r1 } | Self::Compute16KbUniversal { r1 } => Some(*r1)
        }
    }
    pub fn use_8bits_addresses_handling(&self) -> Option<bool> {
        self.r1()
            .map(|r1| r1 == 32)
    }


    fn universal_routine_label(r1: u8) -> String {
        format!("universal_bc26_r1_{}", r1)
    }

    fn c000_routine_label(r1: u8) -> String {
        format!("c000_bc26_r1_{}", r1)
    }

    fn universal_routine(r1: u8) -> Listing {
        let label = Self::universal_routine_label(r1);
        Listing::from_str(&format!("
{}
            ld a,h : add 8 : ld h,a : and #38 : ret nz
            ld a,{} : add l : ld l,a : ld a,#C0 : adc h : ld h,a : res 3,h
            ret
        ", label, r1*2)).unwrap()
    }

    fn c000_routine(r1: u8) -> Listing {
        let label = Self::c000_routine_label(r1);
        Listing::from_str(&format!("
    {}
            ld a,h : add 8 : ld h,a : ret nc
            ld a,{} : add l : ld l,a : ld a,#C0 : adc h : ld h,a : res 3,h
            ret
        ", label, r1*2)).unwrap()
    }


}

/// Dummy display routine
/// 
/// ; Input: HL = drawing address
/// ; ld (hl), val
pub fn linear_sprite_compiler(label: &str, spr: &SpriteOutput, msk: &SpriteOutput, r1: u8) -> Listing {
    let spr = spr.with_encoding(SpriteEncoding::Linear);
    let msk = msk.with_encoding(SpriteEncoding::Linear);

    let comp = Compiler::builder()
            .header_comment("Linear sprite display routine. There is no register optimisation yet. so execution time is not good")
            .header_label(label)
            .bc26(Bc26::new_universal_16k(r1))
            .build();
    comp.compile(&spr, &msk)
}