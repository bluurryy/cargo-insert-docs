use std::ops::Range;

use crate::markdown_rs::{
    self,
    event::{Event, Kind, Name},
    unist::Position,
};

/// Tree interface to a slice of parser events.
///
/// All node iteration is done in reverse order to work nice with a [`StringReplacer`].
pub struct Tree<'m> {
    pub markdown: &'m str,
    pub events: Vec<Event>,
}

impl<'m> Tree<'m> {
    pub fn new(markdown: &'m str) -> Self {
        Self { markdown, events: parse(markdown) }
    }

    /// Create a node from the given event index.
    ///
    /// A node must point at an `Enter` event.
    /// Returns `None` if the event is an `Exit` event.
    pub fn at(&self, index: usize) -> Option<Node<'m, '_>> {
        if self.events[index].kind == Kind::Exit {
            return None;
        }

        Some(Node { tree: self, index })
    }

    pub fn depth_first(&self) -> impl Iterator<Item = Node<'m, '_>> {
        (0..self.events.len()).filter_map(|i| self.at(i))
    }
}

#[derive(Clone, Copy)]
pub struct Node<'m, 't> {
    tree: &'t Tree<'m>,
    index: usize,
}

impl<'m, 't> Node<'m, 't> {
    pub fn name(&self) -> Name {
        self.tree.events[self.index].name.clone()
    }

    pub fn str(&self) -> &'m str {
        &self.tree.markdown[self.byte_range()]
    }

    pub fn child(self, name: Name) -> Option<Self> {
        self.children_with_name(name).next()
    }

    pub fn children_with_name(self, name: Name) -> impl Iterator<Item = Self> {
        self.children().filter(move |n| n.name() == name)
    }

    pub fn children(self) -> impl Iterator<Item = Self> {
        let mut depth = 0;

        (self.index + 1..self.tree.events.len())
            .map_while(move |i| {
                let kind = self.tree.events[i].kind.clone();

                if depth == 0 && kind == Kind::Exit {
                    return None;
                }

                match kind {
                    Kind::Enter => depth += 1,
                    Kind::Exit => depth -= 1,
                }

                Some((i, depth))
            })
            .filter_map(|(i, depth)| (depth == 1).then_some(i))
            .filter_map(|index| self.tree.at(index))
    }

    pub fn descendant(self, name: Name) -> Option<Self> {
        self.descendants_with_name(name).next()
    }

    pub fn descendants_with_name(self, name: Name) -> impl Iterator<Item = Self> {
        self.descendants().filter(move |n| n.name() == name)
    }

    pub fn descendants(self) -> impl Iterator<Item = Self> {
        let mut depth = 0;

        (self.index + 1..self.tree.events.len())
            .take_while(move |&i| {
                let kind = self.tree.events[i].kind.clone();

                if depth == 0 && kind == Kind::Exit {
                    return false;
                }

                match kind {
                    Kind::Enter => depth += 1,
                    Kind::Exit => depth -= 1,
                }

                true
            })
            .filter_map(|i| self.tree.at(i))
    }

    pub fn byte_range(self) -> Range<usize> {
        let pos = self.position();
        pos.start.offset..pos.end.offset
    }

    pub fn position(self) -> Position {
        let event = &self.tree.events[self.index];
        let start = event.point.to_unist();
        let exit_index = self.exit(self.index);
        let end = self.tree.events[exit_index].point.to_unist();
        Position { start, end }
    }

    fn exit(&self, mut i: usize) -> usize {
        let mut depth = 0;

        loop {
            i += 1;

            let Some(event) = self.tree.events.get(i) else {
                unreachable!("unpaired enter/exit event")
            };

            if depth == 0 && event.kind == Kind::Exit {
                return i;
            }

            match event.kind {
                Kind::Enter => depth += 1,
                Kind::Exit => depth -= 1,
            }
        }
    }
}

fn parse(markdown: &str) -> Vec<markdown_rs::event::Event> {
    markdown_rs::parser::parse(markdown, &markdown_rs::ParseOptions::gfm())
        .expect("should only fail for mdx which we don't enable")
        .0
}
