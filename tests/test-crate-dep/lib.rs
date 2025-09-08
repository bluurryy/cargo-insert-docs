#![feature(trait_alias)]
#![feature(extern_types)]

pub mod foreign_mod {}

pub extern crate alloc as foreign_extern_crate;

pub trait ForeignTraitAlias = core::iter::Iterator;

pub type ForeignStructAlias = core::alloc::Layout;

pub static FOREIGN_STATIC: i32 = 0;

unsafe extern "C" {
    pub type ForeignExternType;
}

pub trait ForeignTrait {
    const FOREIGN_ASSOCIATED_CONSTANT: i32;

    type ForeignAssociatedType;
}
