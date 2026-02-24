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
/// Ranges and indices must be removed in order and must not overlap.
#[derive(Debug)]
pub struct StringReplacer<'a> {
    start: usize,
    string: &'a str,
    chunks: Vec<Cow<'a, str>>,
}

impl<'a> StringReplacer<'a> {
    pub fn new(string: &'a str) -> Self {
        Self { start: string.as_ptr().addr(), string, chunks: Vec::new() }
    }

    pub fn position(&self) -> usize {
        self.string.as_ptr().addr() - self.start
    }

    #[expect(dead_code)]
    pub fn rest(&self) -> &str {
        self.string
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

    fn replace_inner(&mut self, mut range: Range<usize>, with: Cow<'a, str>) {
        let offset = self.position();

        if range.start < offset {
            panic!("tried to replace string out of order pos={offset:?} range={range:?}");
        }

        range.start -= offset;
        range.end -= offset;

        assert_le!(range.start, range.end);
        assert_le!(range.end, self.string.len());

        let start_chunk = &self.string[..range.start];

        if !start_chunk.is_empty() {
            self.chunks.push(Cow::Borrowed(start_chunk));
        }

        if !with.is_empty() {
            self.chunks.push(with);
        }

        self.string = &self.string[range.end..];
    }

    pub fn finish(&self) -> String {
        let mut cap = self.string.len();
        self.chunks.iter().for_each(|c: &Cow<'_, str>| cap += c.len());

        let mut out = String::with_capacity(cap);
        self.chunks.iter().for_each(|c| out.push_str(c));
        out.push_str(self.string);
        out
    }
}
