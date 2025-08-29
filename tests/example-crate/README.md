# my-crate-name

Badges go here.

<!-- crate documentation start -->
Use the [Image] type to load images.

## Feature Flags
<!-- feature documentation start -->
- **`std`** *(enabled by default)* — Enables loading [`Image`]s
  from [`std::io::Read`].

### Image formats
The following formats are supported.

- **`jpg`** *(enabled by default)* — Enables support for jpg images
- **`png`** — Enables support for png images
<!-- feature documentation end -->

## Examples
```rust
let image = Image::load("cat.png");
```

[Image]: https://docs.rs/example-crate/0.0.0/example_crate/struct.Image.html
[`Image`]: https://docs.rs/example-crate/0.0.0/example_crate/struct.Image.html
[`std::io::Read`]: https://doc.rust-lang.org/std/io/trait.Read.html

<!-- crate documentation end -->

License goes there.