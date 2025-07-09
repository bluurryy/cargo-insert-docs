This is before the crate docs.

Notice how `lib.rs` does not have to be in the `src` directory for this to work.

Now the crate documentation begins...

<!-- crate documentation start -->
- A shortcut link: [`Vec`](https://doc.rust-lang.org/alloc/vec/struct.Vec.html)!
- An inline link: [`String`](https://doc.rust-lang.org/alloc/string/struct.String.html)!
- A reference: [`ThinRope`](https://doc.rust-lang.org/alloc/string/struct.String.html).

- A link with title: [`str`](https://doc.rust-lang.org/std/primitive.str.html "A String!")
- A http link: [rust](https://www.rust-lang.org/)
- A link with a hash: [`Vec` examples](https://doc.rust-lang.org/alloc/vec/struct.Vec.html#examples).
- A broken reference: [goes nowhere]
- A broken link: [goes somewhere](i lied)

- A link to another crate: [`glob_match`](https://docs.rs/fast-glob/0.4.5/fast_glob/fn.glob_match.html).
- A shortcut to another crate [`fast_glob::glob_match`](https://docs.rs/fast-glob/0.4.5/fast_glob/fn.glob_match.html).
- A link to a crate from github: [`indoc::indoc!`](https://docs.rs/indoc/2.0.6/indoc/macro.indoc.html).

- A link to a struct that is re-exported: [`Reexport`](https://docs.rs/test-crate/0.0.0/test_crate/reexport/struct.Reexport.html).
- A link to a struct that is re-exported with `#[doc(inline)]`: [`ReexportInline`](https://docs.rs/test-crate/0.0.0/test_crate/struct.ReexportInline.html).
- A link to a struct that is re-exported from a private module: [`ReexportPrivate`](https://docs.rs/test-crate/0.0.0/test_crate/struct.ReexportPrivate.html).

[`ThinRope`]: String

## Features
<!-- feature documentation start -->
- **`std`** *(enabled by default)* — Some docs about std
- **`serde`** — Some docs about serde

  Multiple lines work too
- **`something_undocumented`**

Here you can write documentation that goes
between the features

- **`something_else`** — Wow
<!-- feature documentation end -->

## Examples
```rust
// this is rust code
let one = 1;
let two = 2;
assert_eq!(one + two, 3);
```

```rust
// this is rust code as well
let one = 1;
let two = 2;
assert_eq!(one + two, 3);
```

```rust
// believe it or not: rust code
let one = 1;
let two = 2;
assert_eq!(one + two, 3);
```

```python
# this most certainly isn't though
def square(n):
    n * n
```
<!-- crate documentation end -->

This is after the crate docs.