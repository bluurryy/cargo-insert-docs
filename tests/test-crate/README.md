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

- A link to a module: [`my_module`](https://docs.rs/test-crate/0.0.0/test_crate/my_module/index.html)
- A link to an extern crate: [`alloc`](https://doc.rust-lang.org/alloc/index.html)
- A link to a use: [`MyStructUse`](https://docs.rs/test-crate/0.0.0/test_crate/struct.MyStruct.html)
- A link to a union: [`MyUnion`](https://docs.rs/test-crate/0.0.0/test_crate/union.MyUnion.html)
- A link to a struct: [`MyStruct`](https://docs.rs/test-crate/0.0.0/test_crate/struct.MyStruct.html)
- A link to a struct field: [`MyStruct::my_field`](https://docs.rs/test-crate/0.0.0/test_crate/struct.MyStruct.html#structfield.my_field)
- A link to an enum: [`MyEnum`](https://docs.rs/test-crate/0.0.0/test_crate/enum.MyEnum.html)
- A link to a variant: [`MyEnum::MyVariant`](https://docs.rs/test-crate/0.0.0/test_crate/enum.MyEnum.html#variant.MyVariant)
- A link to a function: [`my_function`](https://docs.rs/test-crate/0.0.0/test_crate/fn.my_function.html)
- A link to a trait: [`MyTrait`](https://docs.rs/test-crate/0.0.0/test_crate/trait.MyTrait.html)
- A link to a trait alias: [`IntoString`]
- A link to an impl block is not possible
- A link to a type alias: [`MyStructAlias`](https://docs.rs/test-crate/0.0.0/test_crate/type.MyStructAlias.html)
- A link to a constant: [`MY_CONSTANT`](https://docs.rs/test-crate/0.0.0/test_crate/constant.MY_CONSTANT.html)
- A link to a static: [`MY_STATIC`](https://docs.rs/test-crate/0.0.0/test_crate/static.MY_STATIC.html)
- A link to an extern type: [`MyExternType`](https://docs.rs/test-crate/0.0.0/test_crate/foreigntype.MyExternType.html)
- A link to a macro: [`my_macro`](https://docs.rs/test-crate/0.0.0/test_crate/macro.my_macro.html)
- A link to a proc macro: [`phf_macros::phf_map`](https://docs.rs/phf_macros/0.12.1/phf_macros/macro.phf_map.html)
- A link to a primitive: [`i32`](https://doc.rust-lang.org/std/primitive.i32.html)
- A link to an associated constant: [`MyTrait::MY_ASSOCIATED_CONSTANT`](https://docs.rs/test-crate/0.0.0/test_crate/trait.MyTrait.html#associatedconstant.MY_ASSOCIATED_CONSTANT)
- A link to an associated type: [`MyTrait::MyAssociatedType`](https://docs.rs/test-crate/0.0.0/test_crate/trait.MyTrait.html#associatedtype.MyAssociatedType)
- A link to a proc macro attribute is not possible?
- A link to a proc macro derive: [`Debug`](https://doc.rust-lang.org/core/fmt/macros/derive.Debug.html)
- A link to a keyword is not possible
- A link to a method: [`MyStruct::my_method`](https://docs.rs/test-crate/0.0.0/test_crate/struct.MyStruct.html#method.my_method)

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

```custom,{.language-c}
// i don't see what this language could be
int main(void) { return 0; }
```
<!-- crate documentation end -->

This is after the crate docs.