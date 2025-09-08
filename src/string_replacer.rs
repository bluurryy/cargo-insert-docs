#[cfg(test)]
mod tests;

use core::ops::Range;
use std::borrow::Cow;

macro_rules! assert_le {
    ($left:expr, $right:expr) => {
        match (&$left, &$right) {
            (left, right) => {
                if !(left <= right) {
                    panic!(
                        "assertion `{left_expr} <= {right_expr}` failed\n  left: {left:?}\n right: {right:?}",
                        left_expr = stringify!($left),
                        right_expr = stringify!($right),
                    )
                }
            }
        }
    };
}

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
        assert_le!(range.start, range.end);
        assert_le!(range.end, self.string.len());

        let end_chunk = &self.string[range.end..];

        if !end_chunk.is_empty() {
            self.chunks.push(Cow::Borrowed(end_chunk));
        }

        if !with.is_empty() {
            self.chunks.push(with);
        }

        self.string = &self.string[..range.start];
    }

    pub fn rest(&self) -> &str {
        self.string
    }

    pub fn finish(&self) -> String {
        let cap = self.string.len() + self.chunks.iter().map(|c| c.len()).sum::<usize>();
        let mut out = String::with_capacity(cap);

        out.push_str(self.string);
        self.chunks.iter().rev().for_each(|c| out.push_str(c));
        out
    }
}
