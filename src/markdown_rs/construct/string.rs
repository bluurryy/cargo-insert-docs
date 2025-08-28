//! The string content type.
//!
//! **String** is a limited [text][] like content type which only allows
//! character escapes and character references.
//! It exists in things such as identifiers (media references, definitions),
//! titles, URLs, code (fenced) info and meta parts.
//!
//! The constructs found in string are:
//!
//! * [Character escape][crate::markdown_rs::construct::character_escape]
//! * [Character reference][crate::markdown_rs::construct::character_reference]
//!
//! [text]: crate::markdown_rs::construct::text

use crate::markdown_rs::construct::partial_whitespace::resolve_whitespace;
use crate::markdown_rs::resolve::Name as ResolveName;
use crate::markdown_rs::state::{Name as StateName, State};
use crate::markdown_rs::subtokenize::Subresult;
use crate::markdown_rs::tokenizer::Tokenizer;

/// Characters that can start something in string.
const MARKERS: [u8; 2] = [b'&', b'\\'];

/// Start of string.
///
/// ````markdown
/// > | ```js
///        ^
/// ````
pub fn start(tokenizer: &mut Tokenizer) -> State {
    tokenizer.tokenize_state.markers = &MARKERS;
    State::Retry(StateName::StringBefore)
}

/// Before string.
///
/// ````markdown
/// > | ```js
///        ^
/// ````
pub fn before(tokenizer: &mut Tokenizer) -> State {
    match tokenizer.current {
        None => {
            tokenizer.register_resolver(ResolveName::Data);
            tokenizer.register_resolver(ResolveName::String);
            State::Ok
        }
        Some(b'&') => {
            tokenizer.attempt(
                State::Next(StateName::StringBefore),
                State::Next(StateName::StringBeforeData),
            );
            State::Retry(StateName::CharacterReferenceStart)
        }
        Some(b'\\') => {
            tokenizer.attempt(
                State::Next(StateName::StringBefore),
                State::Next(StateName::StringBeforeData),
            );
            State::Retry(StateName::CharacterEscapeStart)
        }
        _ => State::Retry(StateName::StringBeforeData),
    }
}

/// At data.
///
/// ````markdown
/// > | ```js
///        ^
/// ````
pub fn before_data(tokenizer: &mut Tokenizer) -> State {
    tokenizer.attempt(State::Next(StateName::StringBefore), State::Nok);
    State::Retry(StateName::DataStart)
}

/// Resolve whitespace in string.
pub fn resolve(tokenizer: &mut Tokenizer) -> Option<Subresult> {
    resolve_whitespace(tokenizer, false, false);
    None
}
