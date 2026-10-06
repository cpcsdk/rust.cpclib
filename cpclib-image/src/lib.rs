pub mod asic;
pub mod color;
pub mod ga;
pub mod image;
pub mod ink;
pub mod kit;
pub mod ocp;
pub mod palette;
pub mod pen;
pub mod pixels;
pub mod screen;

/// True-color-to-CPC conversion: resize + automatic palette selection +
/// dithering, feeding into `transfer` for the actual byte encoding.
pub mod convert;

/// PC to CPC image transfer: exact resolution, exact/near hardware colors already. WIP
pub mod transfer;
