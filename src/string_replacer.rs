use core::ops::Range;
use std::borrow::Cow;

/// A type to efficiently replace string ranges.
///
/// Ranges and indices must be removed in reverse order and must not overlap.
///
/// This type is more efficient than using `String::replace_range` repeatedly.
/// It also catches bugs by panicking when ranges are replaced in the wrong
/// order, or if they overlap.
pub struct StringReplacer<'a> {
    string: &'a str,
    chunks: Vec<Cow<'a, str>>,
}

impl<'a> StringReplacer<'a> {
    pub const fn new(string: &'a str) -> Self {
        Self { string, chunks: Vec::new() }
    }

    pub fn replace(&mut self, range: Range<usize>, with: impl Into<Cow<'a, str>>) {
        self.replace_inner(range, with.into())
    }

    pub fn insert(&mut self, idx: usize, with: impl Into<Cow<'a, str>>) {
        self.replace(idx..idx, with)
    }

    pub fn remove(&mut self, range: Range<usize>) {
        self.replace(range, "")
    }

    fn replace_inner(&mut self, range: Range<usize>, with: Cow<'a, str>) {
        assert!(range.end >= range.start);
        assert!(range.end <= self.string.len());

        let end_chunk = &self.string[range.end..];

        if !end_chunk.is_empty() {
            self.chunks.push(Cow::Borrowed(end_chunk));
        }

        if !with.is_empty() {
            self.chunks.push(with);
        }

        self.string = &self.string[..range.start];
    }

    pub fn finish(&self) -> String {
        let cap = self.string.len() + self.chunks.iter().map(|c| c.len()).sum::<usize>();
        let mut out = String::with_capacity(cap);

        out.push_str(self.string);
        self.chunks.iter().rev().for_each(|c| out.push_str(c));
        out
    }
}

#[test]
fn insert() {
    let str = "foobazqux";
    let mut replacer = StringReplacer::new(str);
    replacer.insert(3, "bar");
    assert_eq!(replacer.finish(), "foobarbazqux");
}

#[test]
fn test_simple() {
    let str = "foobarbaz";
    let mut replacer = StringReplacer::new(str);

    replacer.replace(6..8, "BA");
    replacer.replace(4..6, "AR");
    replacer.replace(1..3, "OO");

    assert_eq!(replacer.finish(), "fOObARBAz");
}

#[test]
fn test_edges() {
    let str = "foobarbaz";
    let mut replacer = StringReplacer::new(str);

    replacer.replace(6..9, "BAZ");
    replacer.replace(0..3, "FOO");

    assert_eq!(replacer.finish(), "FOObarBAZ");
}

#[test]
fn test_remove() {
    let str = "foobarbaz";
    let mut replacer = StringReplacer::new(str);

    replacer.replace(7..9, "");
    replacer.replace(4..5, "");
    replacer.replace(0..2, "");

    assert_eq!(replacer.finish(), "obrb");
}

#[test]
fn test_grow() {
    let str = "foobarbaz";

    let mut replacer = StringReplacer::new(str);
    replacer.replace(0..3, "FOOOOOO");
    assert_eq!(replacer.finish(), "FOOOOOObarbaz");

    let mut replacer = StringReplacer::new(str);
    replacer.replace(3..6, "BAAAAAAAAR");
    assert_eq!(replacer.finish(), "fooBAAAAAAAARbaz");

    let mut replacer = StringReplacer::new(str);
    replacer.replace(6..9, "BAAAAAAAAZ");
    assert_eq!(replacer.finish(), "foobarBAAAAAAAAZ");
}

#[test]
#[should_panic = "assertion failed: range.end >= range.start"]
#[expect(clippy::reversed_empty_ranges)]
fn test_panic_reverse_range() {
    let str = "foobarbaz";
    let mut replacer = StringReplacer::new(str);
    replacer.replace(1..0, "a");
}

#[test]
#[should_panic = "assertion failed: range.end <= self.string.len()"]
fn test_panic_out_of_bounds() {
    let str = "foobarbaz";
    let mut replacer = StringReplacer::new(str);
    replacer.replace(3..10, "a");
}

#[test]
#[should_panic = "assertion failed: range.end <= self.string.len()"]
fn test_panic_overlap() {
    let str = "foobarbaz";
    let mut replacer = StringReplacer::new(str);
    replacer.replace(6..9, "whatever");
    replacer.replace(5..7, "b");
}
