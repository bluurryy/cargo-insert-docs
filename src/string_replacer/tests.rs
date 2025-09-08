use super::StringReplacer;

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
#[should_panic = "assertion `range.start <= range.end` failed"]
#[expect(clippy::reversed_empty_ranges)]
fn test_panic_reverse_range() {
    let str = "foobarbaz";
    let mut replacer = StringReplacer::new(str);
    replacer.replace(1..0, "a");
}

#[test]
#[should_panic = "assertion `range.end <= self.string.len()` failed"]
fn test_panic_out_of_bounds() {
    let str = "foobarbaz";
    let mut replacer = StringReplacer::new(str);
    replacer.replace(3..10, "a");
}

#[test]
#[should_panic = "assertion `range.end <= self.string.len()` failed"]
fn test_panic_overlap() {
    let str = "foobarbaz";
    let mut replacer = StringReplacer::new(str);
    replacer.replace(6..9, "whatever");
    replacer.replace(5..7, "b");
}
