use core::ops::Range;
use std::fmt::{self, Write as _};

use crate::markdown_rs::{
    self,
    event::{Event, Kind},
};

#[allow(dead_code)]
pub fn events_to_string(markdown: &str) -> String {
    fn events_to_string(events: &[Event], source: &str) -> String {
        let mut fmt = TreeFormatterStack::new();

        fmt.push();
        fmt.label("Document");
        fmt.child_len(children(events));

        for (i, event) in events.iter().enumerate() {
            match event.kind {
                Kind::Enter => {
                    fmt.push();
                    let name = &event.name;
                    let range = range(&events[i..]);
                    let text = &source[range];
                    let link = event
                        .link
                        .as_ref()
                        .map(|link| format!(" {{ link: {link:?} }}"))
                        .unwrap_or_default();
                    fmt.label(format!("{name:?} {text:?}{link}"));
                    fmt.child_len(children(&events[i + 1..]));
                }
                Kind::Exit => fmt.pop(),
            }
        }

        fmt.finish()
    }

    fn range(events: &[Event]) -> Range<usize> {
        let start = events[0].point.index;
        let mut depth = 0usize;

        for event in &events[1..] {
            match event.kind {
                Kind::Enter => depth += 1,
                Kind::Exit => match depth.checked_sub(1) {
                    Some(new_depth) => depth = new_depth,
                    None => return start..event.point.index,
                },
            }
        }

        start..start
    }

    fn children(events: &[Event]) -> usize {
        let mut depth = 0usize;
        let mut count = 0usize;

        for event in events {
            match event.kind {
                Kind::Enter => {
                    if depth == 0 {
                        count += 1;
                    }

                    depth += 1;
                }
                Kind::Exit => match depth.checked_sub(1) {
                    Some(new_depth) => depth = new_depth,
                    None => return count,
                },
            }
        }

        count
    }

    let parse_options = markdown_rs::ParseOptions::gfm();

    let (events, _state) = markdown_rs::parser::parse(markdown, &parse_options)
        .expect("should only fail for mdx which we don't enable");

    events_to_string(&events, markdown)
}

pub struct TreeFormatterStack {
    vec: Vec<TreeFormatter>,
}

impl TreeFormatterStack {
    pub fn new() -> Self {
        Self { vec: vec![] }
    }

    pub fn push(&mut self) {
        self.vec.push(TreeFormatter::default())
    }

    pub fn pop(&mut self) {
        let str = self.vec.pop().unwrap().finish();
        assert!(!self.vec.is_empty(), "the last pop must be a finish");
        self.child(&str);
    }

    pub fn label(&mut self, label: impl fmt::Display) {
        self.vec.last_mut().unwrap().label(label);
    }

    pub fn child_len(&mut self, len: usize) {
        self.vec.last_mut().unwrap().child_len(len);
    }

    pub fn child(&mut self, str: &str) {
        self.vec.last_mut().unwrap().child(str);
    }

    pub fn finish(mut self) -> String {
        self.vec.pop().unwrap().finish()
    }
}

#[derive(Default)]
pub struct TreeFormatter {
    out: String,
    child_len: usize,
    child_i: usize,
}

impl TreeFormatter {
    pub fn label(&mut self, label: impl fmt::Display) {
        writeln!(self.out, "{label}").unwrap()
    }

    pub fn children(
        &mut self,
        iter: impl IntoIterator<Item = impl AsRef<str>, IntoIter: ExactSizeIterator>,
    ) {
        let iter = iter.into_iter();
        self.child_len(iter.len());
        iter.for_each(|child| self.child(child.as_ref()));
    }

    pub fn child_len(&mut self, len: usize) {
        self.child_len = len;
    }

    pub fn child(&mut self, string: &str) {
        let child_i = self.child_i;
        let is_last = child_i == self.child_len.wrapping_sub(1);

        for (i, line) in string.lines().enumerate() {
            #[expect(clippy::collapsible_else_if)]
            let indent = if i == 0 {
                if is_last { "└── " } else { "├── " }
            } else {
                if is_last { "    " } else { "│   " }
            };

            self.out.push_str(indent);
            self.out.push_str(line);
            self.out.push('\n');
        }

        self.child_i += 1;
    }

    pub fn finish(self) -> String {
        self.out
    }
}
