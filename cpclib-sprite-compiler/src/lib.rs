use std::ops::{Deref, DerefMut};

use cpclib_asm::{inc_l, BaseListing, Expr, IfBuilder, Listing, ListingBuilder, ListingExt, ListingFromStr, ListingSelector, TestKind};
use cpclib_image::convert::{SpriteEncoding, SpriteOutput};
use bon::Builder;
use smol_str::SmolStr;
use itertools::Itertools;

#[derive(Builder)]
//#[builder(start_fn = with_header)]
#[builder(on(String, into))]                                                
#[builder(on(SmolStr, into))]                                                
pub struct Compiler {
  //  #[builder(start_fn)]
    header_comment: Option<String>,
   // #[builder(start_fn)]
    header_label: Option<SmolStr>,

    #[builder(default)]
    lst: Listing,

    bc26: Bc26
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
    pub fn compile(mut self, spr: &SpriteOutput, msk: &SpriteOutput) -> Listing {
        assert_eq!(spr.bytes_width(), msk.bytes_width());
        assert_eq!(spr.height(), msk.height());
        assert_eq!(spr.encoding(), msk.encoding());

        let width = spr.bytes_width();







        self.emit_header();
        let mut previous_moves = 0;
        for (idx, line) in spr.data().iter().cloned().zip(msk.data().iter().cloned())
            .chunks(width)
            .into_iter()
            .enumerate() {
            previous_moves = self.emit_line(idx, line, previous_moves)
        }

        self.emit_footer();
        self.lst
    } 

    fn emit_line(&mut self, idx: usize, line: impl Iterator<Item=(u8, u8)>, previous_moves: u16) -> u16 {
        let mut nb_moves = 0;
        self.emit_line_header(idx, previous_moves);

        let mut lst = ListingBuilder::default();
        let mut not_drawned_count = 0;
        let mut drawned_but_not_moved = false;

        let line = line.collect_vec();
        let nb_steps = line.len();
        for (step, (pixs, mask)) in line.into_iter().enumerate() {


            // we need to lazyly  move the write buffer when some modifications have to be done
            if mask != 0xff && (not_drawned_count > 0 || drawned_but_not_moved) {
                assert!(self.use_8bits_addresses_handling());

                // get the number of displacement since last screen update
                let local_moves = not_drawned_count + if drawned_but_not_moved {
                    1
                } else {
                    0
                };

                // select the fastest way to do it
                let chosen: Listing = if local_moves == 1 {
                    inc_l().into()
                } else {
                    let choice1: Listing = ListingBuilder::default()
                        .repeat(local_moves, inc_l())
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
                };
                
                // add it to the generated code
                lst = lst
                    .comment(format!("Move of {} bytes", local_moves))
                    .extend(chosen); // inject the fastest one

                // store the real number of move (to go back to the beginning of the line)
                nb_moves += local_moves;

                // rest the counters
                not_drawned_count = 0;
                drawned_but_not_moved = false;
            }

            // properly handle the byte
            match mask {
                0x00 => {
                    // no background pixels are kept
                    lst = lst
                        .comment("No masking here")
                        .ld_mem_hl_expr(pixs);
                    drawned_but_not_moved = true;
                    
                },

                0xff => {
                    // all background pixels are kept
                    lst = lst.comment("Nothing shown here");
                    not_drawned_count += 1;
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
                    lst = if pixs != 0 {
                        lst.or_expr(pixs)
                    } else {
                        lst
                    };

                    // save
                    lst = lst.ld_mem_hl_a();

                    if nb_steps-1 == step {
                        lst = lst.comment("End of line");
                    }  
                    drawned_but_not_moved = true;
                },

            };
        }

        self.inject_listing(&lst.build());

        self.emit_line_footer();
        nb_moves as _
    }

    fn emit_line_header(&mut self, idx: usize, steps:u16) {
        self.add_comment(format!("Handle line {idx}"));
        if idx == 0 {
            self.add_comment("  HL already contains the destination address");
        } else {
            eprintln!("TODO do not call enymore move_to start_line => it is better to know the current x position and lazily move");
            self.emit_move_to_line_start(steps);
            self.emit_compute_next_line_address();
        }

    }

    fn emit_line_footer(&mut self) {

    }

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