#![allow(rustdoc::redundant_explicit_links)]
#![allow(rustdoc::broken_intra_doc_links)]
#![allow(clippy::tabs_in_doc_comments)]
#![feature(trait_alias)]
#![feature(extern_types)]
//! <!-- docs intro start -->
//! This is kitchen-sink test crate for `cargo-insert-docs`.
//! <!-- docs intro end -->
//!
//! <!-- docs rest start -->
//! ### Links
//! - A shortcut reference: [`Vec`]
//! - A collapsed reference: [`Vec`][]
//! - A full reference: [`Vector`][`Vec`]
//!
//! - A shortcut reference with a definition: [`ThinRope`]
//! - A collapsed reference with a definition: [`ThinRope`][]
//! - A full reference with a definition: [`LeanCord`][`ThinRope`]
//!
//! - A link: [`String`](std::string::String)
//! - A link with title: [`str`](str "A String!")
//! - A http link: [rust](https://www.rust-lang.org/)
//! - A link with a hash: [`Vec` examples](Vec#examples).
//! - A broken reference: [goes nowhere]
//! - A broken link: [goes somewhere](i lied)
//! - A link with escaped characters: [Vec \[...\] tor](std::vec::Vec "does \"this\" work?")
//!
//! ### Inter-doc links
//! - A link to another crate: [`glob_match`](fast_glob::glob_match).
//! - A shortcut to another crate [`fast_glob::glob_match`].
//! - A link to a crate from github: [`indoc::indoc!`].
//!
//! ### Re-exports
//! - A link to a struct that is re-exported: [`Reexport`].
//! - A link to a struct that is re-exported with `#[doc(inline)]`: [`ReexportInline`].
//! - A link to a struct that is re-exported from a private module: [`ReexportPrivate`].
//!
//! ### Glob re-exports
//! Rustdoc's json glob uses put the burden of resolving exports on the user.
//! This is too hard: <https://github.com/rust-lang/rustdoc-types/issues/51#issuecomment-3071677482>
//! But we can have a naive implementation and try to not crash.
//!
//! - A link to types that are glob-imported: [`MyGlobImportedStruct`], [`my_glob_imported_fn`]
//! - A link to types that are glob-imported with `#[doc(inline)]`: [`MyInlineGlobImportedStruct`], [`my_inline_glob_imported_fn`]
//! - A link to types that are glob-imported from a private module: [`MyGlobImportedStructFromPrivateMod`], [`my_glob_imported_fn_from_private_mod`]
//! - A link to a struct from a mutually reexporting module: `Batman` (TODO)
//! - A link to structs from recursively glob-reexporting modules: `StructInGlobA`, `StructInGlobB`, `StructInGlobC` (TODO)
//!
//! ### Item variants
//! - A link to a module: [`my_module`] (foreign: [`std::mem`])
//! - A link to an extern crate: [`alloc`] (foreign: [`test_crate_dep::foreign_mod`])
//! - A link to a use: [`MyStructUse`] (foreign: [`test_crate_dep::foreign_extern_crate`])
//! - A link to a union: [`MyUnion`] (foreign: [`std::mem::MaybeUninit`])
//! - A link to a struct: [`MyStruct`] (foreign: [`std::mem::Discriminant`])
//! - A link to a struct field: [`MyStruct::my_field`] (foreign: [`std::ops::Range::start`])
//! - A link to an enum: [`MyEnum`] (foreign: [`std::cmp::Ordering`])
//! - A link to a variant: [`MyEnum::MyVariant`] (foreign: [`std::cmp::Ordering::Less`])
//! - A link to a function: [`my_function`] (foreign: [`std::mem::drop`])
//! - A link to a trait: [`MyTrait`] (foreign: [`std::iter::Iterator`])
//! - A link to a trait alias: [`IntoString`] (foreign: [`test_crate_dep::ForeignTraitAlias`])
//! - A link to an impl block is not possible
//! - A link to a type alias: [`MyStructAlias`] (foreign: [`test_crate_dep::ForeignTraitAlias`])
//! - A link to a constant: [`MY_CONSTANT`] (foreign: [`std::f32::consts::E`])
//! - A link to a static: [`MY_STATIC`] (foreign: [`test_crate_dep::FOREIGN_STATIC`])
//! - A link to an extern type: [`MyExternType`] (foreign: [`test_crate_dep::ForeignExternType`])
//! - A link to a macro: [`my_macro`] (foreign: [`std::format_args`])
//! - A link to a proc macro: [`phf_macros::phf_map`]
//! - A link to a primitive: [`i32`]
//! - A link to an associated constant: [`MyTrait::MY_ASSOCIATED_CONSTANT`] (foreign: [`test_crate_dep::ForeignTrait::FOREIGN_ASSOCIATED_CONSTANT`], [`f32::NAN`])
//! - A link to an associated type: [`MyTrait::MyAssociatedType`] (foreign: [`test_crate_dep::ForeignTrait::ForeignAssociatedType`])
//! - A link to a proc macro attribute is not possible?
//! - A link to a proc macro derive: [`Debug`]
//! - A link to a keyword is not possible
//! - A link to a builtin attribute: [`derive`]
//! - A link to a method: [`MyStruct::my_method`] (foreign: [`std::alloc::Layout::size`])
//! - A link to a required trait method: [`MyTrait::my_required_method`] (foreign: [`std::iter::Iterator::next`])
//! - A link to a provided trait method: [`MyTrait::my_provided_method`] (foreign: [`std::iter::Iterator::size_hint`])
//!
//! [`ThinRope`]: String
//!
//! # Features
//! <!-- features start -->
//! - **`std`** *(enabled by default)* — Some docs about std
//! - **`serde`** — Some docs about serde
//!
//!   Multiple lines work too
//! - **`something_undocumented`**
//! - **`recurse`** — Actually used feature, enables recursive imports that will cause errors.
//! - **`recurse-glob`** — Actually used feature, enables recursive glob imports that will cause errors.
//!
//! Here you can write documentation that goes
//! between the features
//!
//! - **`something_else`** — Wow
//! <!-- features end -->
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
//! ```compile_fail,E0369
//! // this is rust code as well
//! "hello" + "world"
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
//!
//! Test if ignoring lines work.
//! ```
//! # // ignore this line
//! #[derive(Debug)] // don't ignore this line
//! struct Foo {
//!    foo: i32
//! }
//!
//!   # // ignore this aswell
//!   #[derive(Debug)] // don't ignore this line
//! struct Bar;
//!
//! let s = "foo
//! ## bar # baz";
//! assert_eq!(s, "foo\n# bar # baz");
//!
//! let s = "foo
//! ### bar # baz";
//! assert_eq!(s, "foo\n## bar # baz");
//! ```
//!
//! Test if ignoring lines work for indented code blocks.
//!
//!     # // ignore this line
//!     #[derive(Debug)] // don't ignore this line
//!     struct Foo {
//!         foo: i32
//!     }
//!
//!       # // ignore this aswell
//!       #[derive(Debug)] // don't ignore this line
//!     struct Bar;
//!
//!     let s = "foo
//!     ## bar # baz";
//!     assert_eq!(s, "foo\n# bar # baz");
//!
//!     let s = "foo
//!     ### bar # baz";
//!     assert_eq!(s, "foo\n## bar # baz");
//!
//! Test if ignoring lines work in a quoted code block.
//!
//! > ```
//! > assert_eq!(1 + 1, 2);
//! > # // this is ignored
//! > ```
//!
//! Test if ignoring lines work in a listed code block.
//!
//! - ```
//!   assert_eq!(1 + 1, 2);
//!   # // this is ignored
//!   ```
//! - ```
//!   assert_eq!(1 + 1, 2);
//!   # // this is ignored
//!   ```
//!
//! <!-- docs rest end -->

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

#[cfg(feature = "recurse")]
pub mod a {
    pub use crate::n;
    pub struct Batman;
}

#[cfg(feature = "recurse")]
pub mod n {
    pub use crate::a;
}

#[cfg(feature = "recurse")]
pub use n::a::n::a::n::a::n::a::n::a::n::a::n::a::n::a::Batman;

#[cfg(feature = "recurse-glob")]
pub mod glob_a {
    pub use super::glob_c::*;
    pub struct StructInGlobA;
}

#[cfg(feature = "recurse-glob")]
pub mod glob_b {
    pub use super::glob_a::*;
    pub struct StructInGlobB;
}

#[cfg(feature = "recurse-glob")]
pub mod glob_c {
    pub use super::glob_b::*;
    pub struct StructInGlobC;
}

#[cfg(feature = "recurse-glob")]
pub use glob_a::*;

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
    fn my_required_method(&self);
    fn my_provided_method(&self) {}
}
pub trait MyTraitAlias = Into<String>;
pub type MyStructAlias = MyStruct;
pub const MY_CONSTANT: i32 = 0;
pub static MY_STATIC: i32 = 0;
unsafe extern "C" {
    pub type MyExternType;
}
