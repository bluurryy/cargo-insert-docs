use std::fmt::{self, Write as _};

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
