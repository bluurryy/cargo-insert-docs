//! Byte order mark occurs at the start of the document.
//!
//! ## Grammar
//!
//! Byte order mark forms with the following BNF
//! (<small>see [construct][crate::markdown_rs::construct] for character groups</small>):
//!
//! ```bnf
//! byte_order_mark ::= 0xEF 0xBB 0xBF
//! ```
//!
//! ## Recommendation
//!
//! Don’t use BOMs.
//!
//! ## Tokens
//!
//! * [`ByteOrderMark`][Name::ByteOrderMark]
//!
//! ## References
//!
//! * [`micromark/lib/preprocess.js` in `micromark`](https://github.com/micromark/micromark/blob/ed23453/packages/micromark/dev/lib/preprocess.js#L54-L60)

use crate::markdown_rs::event::Name;
use crate::markdown_rs::state::{Name as StateName, State};
use crate::markdown_rs::tokenizer::Tokenizer;

/// Bytes of a BOM.
const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// Before BOM.
///
/// ```text
/// > | 0xEF 0xBB 0xBF
///     ^^^^
/// ```
pub fn start(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(BOM[0]) {
        tokenizer.enter(Name::ByteOrderMark);
        State::Retry(StateName::BomInside)
    } else {
        State::Nok
    }
}

/// In BOM.
///
/// ```text
/// > | 0xEF 0xBB 0xBF
///     ^^^^ ^^^^ ^^^^
/// ```
pub fn inside(tokenizer: &mut Tokenizer) -> State {
    if tokenizer.current == Some(BOM[tokenizer.tokenize_state.size]) {
        tokenizer.tokenize_state.size += 1;
        tokenizer.consume();
        if tokenizer.tokenize_state.size == BOM.len() {
            tokenizer.exit(Name::ByteOrderMark);
            tokenizer.tokenize_state.size = 0;
            State::Ok
        } else {
            State::Next(StateName::BomInside)
        }
    } else {
        tokenizer.tokenize_state.size = 0;
        State::Nok
    }
}
