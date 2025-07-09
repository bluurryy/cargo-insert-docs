#![allow(rustdoc::redundant_explicit_links, rustdoc::broken_intra_doc_links)]
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
