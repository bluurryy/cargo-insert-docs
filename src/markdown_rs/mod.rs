#![allow(clippy::implicit_saturating_sub)]
#![allow(clippy::new_without_default)]
#![allow(clippy::doc_link_with_quotes)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::result_large_err)]
#![allow(clippy::len_without_is_empty)]

extern crate alloc;
mod configuration;
pub mod construct;
pub mod event;
pub mod parser;
mod resolve;
mod state;
mod subtokenize;
mod tokenizer;
pub mod util;

pub mod message; // To do: externalize.
pub mod unist; // To do: externalize.

#[doc(hidden)]
pub use util::character_reference::{decode_named, decode_numeric};

#[doc(hidden)]
pub use util::identifier::{id_cont, id_start};

#[doc(hidden)]
pub use util::sanitize_uri::sanitize;

#[doc(hidden)]
pub use util::location::Location;

pub use util::line_ending::LineEnding;

pub use util::mdx::{
    EsmParse as MdxEsmParse, ExpressionKind as MdxExpressionKind,
    ExpressionParse as MdxExpressionParse, Signal as MdxSignal,
};

pub use configuration::{CompileOptions, Constructs, Options, ParseOptions};
