#![allow(rustdoc::redundant_explicit_links)]
#![allow(rustdoc::broken_intra_doc_links)]
#![feature(trait_alias)]
#![feature(extern_types)]
//! - A shortcut link: [`Vec`]!
//! - An inline link: [`String`](std::string::String)!
//! - A reference: [`ThinRope`].
//!
//! - A link with title: [`str`](str "A String!")
//! - A http link: [rust](https://www.rust-lang.org/)
//! - A link with a hash: [`Vec` examples](Vec#examples).
//! - A broken reference: [goes nowhere]
//! - A broken link: [goes somewhere](i lied)
//!
//! - A link to another crate: [`glob_match`](fast_glob::glob_match).
//! - A shortcut to another crate [`fast_glob::glob_match`].
//! - A link to a crate from github: [`indoc::indoc!`].
//!
//! - A link to a struct that is re-exported: [`Reexport`].
//! - A link to a struct that is re-exported with `#[doc(inline)]`: [`ReexportInline`].
//! - A link to a struct that is re-exported from a private module: [`ReexportPrivate`].
//! - A link to types that are glob-imported: [`MyGlobImportedStruct`], [`my_glob_imported_fn`]
//! - A link to types that are glob-imported with `#[doc(inline)]`: [`MyInlineGlobImportedStruct`], [`my_inline_glob_imported_fn`]
//! - A link to types that are glob-imported from a private module: [`MyGlobImportedStructFromPrivateMod`], [`my_glob_imported_fn_from_private_mod`]
//! - A link to a struct from a mutually reexporting module: `Batman` (TODO)
//!
//! - A link to a module: [`my_module`]
//! - A link to an extern crate: [`alloc`]
//! - A link to a use: [`MyStructUse`]
//! - A link to a union: [`MyUnion`]
//! - A link to a struct: [`MyStruct`]
//! - A link to a struct field: [`MyStruct::my_field`]
//! - A link to an enum: [`MyEnum`]
//! - A link to a variant: [`MyEnum::MyVariant`]
//! - A link to a function: [`my_function`]
//! - A link to a trait: [`MyTrait`]
//! - A link to a trait alias: [`IntoString`]
//! - A link to an impl block is not possible
//! - A link to a type alias: [`MyStructAlias`]
//! - A link to a constant: [`MY_CONSTANT`]
//! - A link to a static: [`MY_STATIC`]
//! - A link to an extern type: [`MyExternType`]
//! - A link to a macro: [`my_macro`]
//! - A link to a proc macro: [`phf_macros::phf_map`]
//! - A link to a primitive: [`i32`]
//! - A link to an associated constant: [`MyTrait::MY_ASSOCIATED_CONSTANT`]
//! - A link to an associated type: [`MyTrait::MyAssociatedType`]
//! - A link to a proc macro attribute is not possible?
//! - A link to a proc macro derive: [`Debug`]
//! - A link to a keyword is not possible
//! - A link to a method: [`MyStruct::my_method`]
//!
//! [`ThinRope`]: String
//!
//! # Features
//! <!-- feature documentation start -->
//! - **`std`** *(enabled by default)* — Some docs about std
//! - **`serde`** — Some docs about serde
//!
//!   Multiple lines work too
//! - **`something_undocumented`**
//!
//! Here you can write documentation that goes
//! between the features
//!
//! - **`something_else`** — Wow
//! <!-- feature documentation end -->
//!
//! # Examples
//! ```
//! // this is rust code
//! let one = 1;
//! # println!("won't show up in readme");
//! let two = 2;
//! assert_eq!(one + two, 3);
//! ```
//!
//! ```compile_fail,E69420
//! // this is rust code as well
//! let one = 1;
//! # println!("won't show up in readme");
//! let two = 2;
//! assert_eq!(one + two, 3);
//! ```
//!
//!     // believe it or not: rust code
//!     let one = 1;
//!     # println!("won't show up in readme");
//!     let two = 2;
//!     assert_eq!(one + two, 3);
//!
//! ```python
//! # this most certainly isn't though
//! def square(n):
//!     n * n
//! ```
//!
//! ```custom,{.language-c}
//! // i don't see what this language could be
//! int main(void) { return 0; }
//! ```
// The docs should not link here because it's not inline.
pub use reexport::Reexport;

pub mod reexport {
    pub struct Reexport;
}

pub mod very {
    pub mod nested {
        pub mod module {
            // The docs should not link here.
            #[doc(inline)]
            pub use crate::reexport::Reexport;
        }
    }
}

// The docs should link here.
#[doc(inline)]
pub use reexport_inline::ReexportInline;

pub mod reexport_inline {
    pub struct ReexportInline;
}

pub use reexport_private::ReexportPrivate;

mod reexport_private {
    pub struct ReexportPrivate;
}

pub mod to_be_glob_imported {
    pub struct MyGlobImportedStruct;
    pub fn my_glob_imported_fn() {}

    #[expect(dead_code)]
    fn my_private_fn() {}
}

pub use to_be_glob_imported::*;

mod to_be_glob_imported_private {
    pub struct MyGlobImportedStructFromPrivateMod;
    pub fn my_glob_imported_fn_from_private_mod() {}

    #[expect(dead_code)]
    fn my_private_fn() {}
}

pub use to_be_glob_imported_private::*;

pub mod to_be_inline_glob_imported {
    pub struct MyInlineGlobImportedStruct;
    pub fn my_inline_glob_imported_fn() {}

    #[expect(dead_code)]
    fn my_private_fn() {}
}

#[doc(inline)]
pub use to_be_inline_glob_imported::*;

#[cfg(false)]
pub mod a {
    pub use crate::n;
    pub struct Batman;
}

#[cfg(false)]
pub mod n {
    pub use crate::a;
}

// TODO: don't let Batman overflow the stack
#[cfg(false)]
pub use n::a::n::a::n::a::n::a::n::a::n::a::n::a::n::a::Batman;

// here come tests to check that we can link to any item kind

pub mod my_module {}
pub extern crate alloc;
pub use MyStruct as MyStructUse;
pub union MyUnion {
    _x: u8,
}
pub struct MyStruct {
    pub my_field: i32,
}
impl MyStruct {
    pub fn my_method(&self) {}
}
pub enum MyEnum {
    MyVariant,
}
#[macro_export]
macro_rules! my_macro {
    () => {};
}
pub fn my_function() {}
pub trait MyTrait {
    const MY_ASSOCIATED_CONSTANT: i32 = 0;
    type MyAssociatedType;
}
pub trait MyTraitAlias = Into<String>;
pub type MyStructAlias = MyStruct;
pub const MY_CONSTANT: i32 = 0;
pub static MY_STATIC: i32 = 0;
unsafe extern "C" {
    pub type MyExternType;
}
