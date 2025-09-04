//! Adapted from `rust-lang/rust`'s `src/librustdoc/html/markdown.rs`

use core::{
    cell::RefCell,
    fmt::Display,
    iter::Peekable,
    marker::PhantomData,
    str::{CharIndices, FromStr},
};

pub fn is_rust(lang: &str) -> Result<bool, Vec<String>> {
    let extra = ExtraInfo::new();
    let parsed = LangString::parse(lang, Some(&extra));
    let errors = extra.into_errors();
    if errors.is_empty() { Ok(parsed.rust) } else { Err(errors) }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Edition;

impl FromStr for Edition {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "future" || s.parse::<u16>().is_ok() { Ok(Edition) } else { Err(()) }
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
struct LangString {
    original: String,
    should_panic: bool,
    no_run: bool,
    ignore: Ignore,
    rust: bool,
    test_harness: bool,
    compile_fail: bool,
    standalone_crate: bool,
    error_codes: Vec<String>,
    edition: Option<Edition>,
    added_classes: Vec<String>,
    unknown: Vec<String>,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub(crate) enum Ignore {
    All,
    None,
    Some(Vec<String>),
}

pub(crate) struct ExtraInfo<'tcx> {
    errors: RefCell<Vec<String>>,
    marker: PhantomData<&'tcx u8>,
}

impl<'tcx> ExtraInfo<'tcx> {
    pub(crate) fn new() -> ExtraInfo<'tcx> {
        ExtraInfo { errors: Default::default(), marker: PhantomData }
    }

    fn into_errors(self) -> Vec<String> {
        self.errors.into_inner()
    }

    fn message(&self, msg: impl Display) {
        self.errors.borrow_mut().push(msg.to_string())
    }

    fn error_invalid_codeblock_attr(&self, msg: impl Display) {
        self.message(format_args!("invalid codeblock attribute: {msg}"));
    }
}

/// This is the parser for fenced codeblocks attributes. It implements the following eBNF:
///
/// ```eBNF
/// lang-string = *(token-list / delimited-attribute-list / comment)
///
/// bareword = LEADINGCHAR *(CHAR)
/// bareword-without-leading-char = CHAR *(CHAR)
/// quoted-string = QUOTE *(NONQUOTE) QUOTE
/// token = bareword / quoted-string
/// token-without-leading-char = bareword-without-leading-char / quoted-string
/// sep = COMMA/WS *(COMMA/WS)
/// attribute = (DOT token)/(token EQUAL token-without-leading-char)
/// attribute-list = [sep] attribute *(sep attribute) [sep]
/// delimited-attribute-list = OPEN-CURLY-BRACKET attribute-list CLOSE-CURLY-BRACKET
/// token-list = [sep] token *(sep token) [sep]
/// comment = OPEN_PAREN *(all characters) CLOSE_PAREN
///
/// OPEN_PAREN = "("
/// CLOSE_PARENT = ")"
/// OPEN-CURLY-BRACKET = "{"
/// CLOSE-CURLY-BRACKET = "}"
/// LEADINGCHAR = ALPHA | DIGIT | "_" | "-" | ":"
/// ; All ASCII punctuation except comma, quote, equals, backslash, grave (backquote) and braces.
/// ; Comma is used to separate language tokens, so it can't be used in one.
/// ; Quote is used to allow otherwise-disallowed characters in language tokens.
/// ; Equals is used to make key=value pairs in attribute blocks.
/// ; Backslash and grave are special Markdown characters.
/// ; Braces are used to start an attribute block.
/// CHAR = ALPHA | DIGIT | "_" | "-" | ":" | "." | "!" | "#" | "$" | "%" | "&" | "*" | "+" | "/" |
///        ";" | "<" | ">" | "?" | "@" | "^" | "|" | "~"
/// NONQUOTE = %x09 / %x20 / %x21 / %x23-7E ; TAB / SPACE / all printable characters except `"`
/// COMMA = ","
/// DOT = "."
/// EQUAL = "="
///
/// ALPHA = %x41-5A / %x61-7A ; A-Z / a-z
/// DIGIT = %x30-39
/// WS = %x09 / " "
/// ```
struct TagIterator<'a, 'tcx> {
    inner: Peekable<CharIndices<'a>>,
    data: &'a str,
    is_in_attribute_block: bool,
    extra: Option<&'a ExtraInfo<'tcx>>,
    is_error: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum LangStringToken<'a> {
    LangToken(&'a str),
    ClassAttribute(&'a str),
    KeyValueAttribute(&'a str, &'a str),
}

fn is_leading_char(c: char) -> bool {
    c == '_' || c == '-' || c == ':' || c.is_ascii_alphabetic() || c.is_ascii_digit()
}
fn is_bareword_char(c: char) -> bool {
    is_leading_char(c) || ".!#$%&*+/;<>?@^|~".contains(c)
}
fn is_separator(c: char) -> bool {
    c == ' ' || c == ',' || c == '\t'
}

struct Indices {
    start: usize,
    end: usize,
}

impl<'a, 'tcx> TagIterator<'a, 'tcx> {
    fn new(data: &'a str, extra: Option<&'a ExtraInfo<'tcx>>) -> Self {
        Self {
            inner: data.char_indices().peekable(),
            data,
            is_in_attribute_block: false,
            extra,
            is_error: false,
        }
    }

    fn emit_error(&mut self, err: impl Display) {
        if let Some(extra) = self.extra {
            extra.error_invalid_codeblock_attr(err);
        }
        self.is_error = true;
    }

    fn skip_separators(&mut self) -> Option<usize> {
        while let Some((pos, c)) = self.inner.peek() {
            if !is_separator(*c) {
                return Some(*pos);
            }
            self.inner.next();
        }
        None
    }

    fn parse_string(&mut self, start: usize) -> Option<Indices> {
        for (pos, c) in self.inner.by_ref() {
            if c == '"' {
                return Some(Indices { start: start + 1, end: pos });
            }
        }
        self.emit_error("unclosed quote string `\"`");
        None
    }

    fn parse_class(&mut self, start: usize) -> Option<LangStringToken<'a>> {
        while let Some((pos, c)) = self.inner.peek().copied() {
            if is_bareword_char(c) {
                self.inner.next();
            } else {
                let class = &self.data[start + 1..pos];
                if class.is_empty() {
                    self.emit_error(format!("unexpected `{c}` character after `.`"));
                    return None;
                } else if self.check_after_token() {
                    return Some(LangStringToken::ClassAttribute(class));
                } else {
                    return None;
                }
            }
        }
        let class = &self.data[start + 1..];
        if class.is_empty() {
            self.emit_error("missing character after `.`");
            None
        } else if self.check_after_token() {
            Some(LangStringToken::ClassAttribute(class))
        } else {
            None
        }
    }

    fn parse_token(&mut self, start: usize) -> Option<Indices> {
        while let Some((pos, c)) = self.inner.peek() {
            if !is_bareword_char(*c) {
                return Some(Indices { start, end: *pos });
            }
            self.inner.next();
        }
        self.emit_error("unexpected end");
        None
    }

    fn parse_key_value(&mut self, c: char, start: usize) -> Option<LangStringToken<'a>> {
        let key_indices =
            if c == '"' { self.parse_string(start)? } else { self.parse_token(start)? };
        if key_indices.start == key_indices.end {
            self.emit_error("unexpected empty string as key");
            return None;
        }

        if let Some((_, c)) = self.inner.next() {
            if c != '=' {
                self.emit_error(format!("expected `=`, found `{}`", c));
                return None;
            }
        } else {
            self.emit_error("unexpected end");
            return None;
        }
        let value_indices = match self.inner.next() {
            Some((pos, '"')) => self.parse_string(pos)?,
            Some((pos, c)) if is_bareword_char(c) => self.parse_token(pos)?,
            Some((_, c)) => {
                self.emit_error(format!("unexpected `{c}` character after `=`"));
                return None;
            }
            None => {
                self.emit_error("expected value after `=`");
                return None;
            }
        };
        if value_indices.start == value_indices.end {
            self.emit_error("unexpected empty string as value");
            None
        } else if self.check_after_token() {
            Some(LangStringToken::KeyValueAttribute(
                &self.data[key_indices.start..key_indices.end],
                &self.data[value_indices.start..value_indices.end],
            ))
        } else {
            None
        }
    }

    /// Returns `false` if an error was emitted.
    fn check_after_token(&mut self) -> bool {
        if let Some((_, c)) = self.inner.peek().copied() {
            if c == '}' || is_separator(c) || c == '(' {
                true
            } else {
                self.emit_error(format!("unexpected `{c}` character"));
                false
            }
        } else {
            // The error will be caught on the next iteration.
            true
        }
    }

    fn parse_in_attribute_block(&mut self) -> Option<LangStringToken<'a>> {
        if let Some((pos, c)) = self.inner.next() {
            if c == '}' {
                self.is_in_attribute_block = false;
                return self.next();
            } else if c == '.' {
                return self.parse_class(pos);
            } else if c == '"' || is_leading_char(c) {
                return self.parse_key_value(c, pos);
            } else {
                self.emit_error(format!("unexpected character `{c}`"));
                return None;
            }
        }
        self.emit_error("unclosed attribute block (`{}`): missing `}` at the end");
        None
    }

    /// Returns `false` if an error was emitted.
    fn skip_paren_block(&mut self) -> bool {
        for (_, c) in self.inner.by_ref() {
            if c == ')' {
                return true;
            }
        }
        self.emit_error("unclosed comment: missing `)` at the end");
        false
    }

    fn parse_outside_attribute_block(&mut self, start: usize) -> Option<LangStringToken<'a>> {
        while let Some((pos, c)) = self.inner.next() {
            if c == '"' {
                if pos != start {
                    self.emit_error("expected ` `, `{` or `,` found `\"`");
                    return None;
                }
                let indices = self.parse_string(pos)?;
                if let Some((_, c)) = self.inner.peek().copied()
                    && c != '{'
                    && !is_separator(c)
                    && c != '('
                {
                    self.emit_error(format!("expected ` `, `{{` or `,` after `\"`, found `{c}`"));
                    return None;
                }
                return Some(LangStringToken::LangToken(&self.data[indices.start..indices.end]));
            } else if c == '{' {
                self.is_in_attribute_block = true;
                return self.next();
            } else if is_separator(c) {
                if pos != start {
                    return Some(LangStringToken::LangToken(&self.data[start..pos]));
                }
                return self.next();
            } else if c == '(' {
                if !self.skip_paren_block() {
                    return None;
                }
                if pos != start {
                    return Some(LangStringToken::LangToken(&self.data[start..pos]));
                }
                return self.next();
            } else if (pos == start && is_leading_char(c)) || (pos != start && is_bareword_char(c))
            {
                continue;
            } else {
                self.emit_error(format!("unexpected character `{c}`"));
                return None;
            }
        }
        let token = &self.data[start..];
        if token.is_empty() { None } else { Some(LangStringToken::LangToken(&self.data[start..])) }
    }
}

impl<'a> Iterator for TagIterator<'a, '_> {
    type Item = LangStringToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_error {
            return None;
        }
        let Some(start) = self.skip_separators() else {
            if self.is_in_attribute_block {
                self.emit_error("unclosed attribute block (`{}`): missing `}` at the end");
            }
            return None;
        };
        if self.is_in_attribute_block {
            self.parse_in_attribute_block()
        } else {
            self.parse_outside_attribute_block(start)
        }
    }
}

impl Default for LangString {
    fn default() -> Self {
        Self {
            original: String::new(),
            should_panic: false,
            no_run: false,
            ignore: Ignore::None,
            rust: true,
            test_harness: false,
            compile_fail: false,
            standalone_crate: false,
            error_codes: Vec::new(),
            edition: None,
            added_classes: Vec::new(),
            unknown: Vec::new(),
        }
    }
}

impl LangString {
    fn parse(string: &str, extra: Option<&ExtraInfo<'_>>) -> Self {
        let mut seen_rust_tags = false;
        let mut seen_other_tags = false;
        let mut seen_custom_tag = false;
        let mut data = LangString::default();
        let mut ignores = vec![];

        data.original = string.to_owned();

        let mut call = |tokens: &mut dyn Iterator<Item = LangStringToken<'_>>| {
            for token in tokens {
                match token {
                    LangStringToken::LangToken("should_panic") => {
                        data.should_panic = true;
                        seen_rust_tags = !seen_other_tags;
                    }
                    LangStringToken::LangToken("no_run") => {
                        data.no_run = true;
                        seen_rust_tags = !seen_other_tags;
                    }
                    LangStringToken::LangToken("ignore") => {
                        data.ignore = Ignore::All;
                        seen_rust_tags = !seen_other_tags;
                    }
                    LangStringToken::LangToken(x) if x.strip_prefix("ignore-").is_some() => {
                        ignores.push(x.strip_prefix("ignore-").unwrap().to_owned());
                        seen_rust_tags = !seen_other_tags;
                    }
                    LangStringToken::LangToken("rust") => {
                        data.rust = true;
                        seen_rust_tags = true;
                    }
                    LangStringToken::LangToken("custom") => {
                        seen_custom_tag = true;
                    }
                    LangStringToken::LangToken("test_harness") => {
                        data.test_harness = true;
                        seen_rust_tags = !seen_other_tags || seen_rust_tags;
                    }
                    LangStringToken::LangToken("compile_fail") => {
                        data.compile_fail = true;
                        seen_rust_tags = !seen_other_tags || seen_rust_tags;
                        data.no_run = true;
                    }
                    LangStringToken::LangToken("standalone_crate") => {
                        data.standalone_crate = true;
                        seen_rust_tags = !seen_other_tags || seen_rust_tags;
                    }
                    LangStringToken::LangToken(x) if x.strip_prefix("edition").is_some() => {
                        data.edition = x.strip_prefix("edition").unwrap().parse::<Edition>().ok();
                    }
                    LangStringToken::LangToken(x)
                        if x.strip_prefix('E').is_some()
                            && x.strip_prefix('E').unwrap().len() == 4 =>
                    {
                        if x.strip_prefix('E').unwrap().parse::<u32>().is_ok() {
                            data.error_codes.push(x.to_owned());
                            seen_rust_tags = !seen_other_tags || seen_rust_tags;
                        } else {
                            seen_other_tags = true;
                        }
                    }
                    LangStringToken::LangToken(x) => {
                        seen_other_tags = true;
                        data.unknown.push(x.to_owned());
                    }
                    LangStringToken::KeyValueAttribute("class", value) => {
                        data.added_classes.push(value.to_owned());
                    }
                    LangStringToken::ClassAttribute(class) => {
                        data.added_classes.push(class.to_owned());
                    }
                    _ => {}
                }
            }
        };

        let mut tag_iter = TagIterator::new(string, extra);
        call(&mut tag_iter);

        // ignore-foo overrides ignore
        if !ignores.is_empty() {
            data.ignore = Ignore::Some(ignores);
        }

        data.rust &= !seen_custom_tag && (!seen_other_tags || seen_rust_tags) && !tag_iter.is_error;

        data
    }
}
