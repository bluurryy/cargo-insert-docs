This is before the crate docs.

Notice how `lib.rs` does not have to be in the `src` directory for this to work.

Now the crate documentation begins...

<!-- docs intro start -->
This is kitchen-sink test crate for `cargo-insert-docs`.
<!-- docs intro end -->

## Table of Contents

This table of contents exists only in the readme. 
The html docs already have a table of contents on the left side panel.

- [Links](#links)
- [Features](#features)
- [Examples](#examples)

## Links

<!-- docs rest start -->
##### Links
- A shortcut reference: [`Vec`]
- A collapsed reference: [`Vec`][]
- A full reference: [`Vector`][`Vec`]

- A shortcut reference with a definition: [`ThinRope`]
- A collapsed reference with a definition: [`ThinRope`][]
- A full reference with a definition: [`LeanCord`][`ThinRope`]

- A link: [`String`](https://doc.rust-lang.org/alloc/string/struct.String.html)
- A link with title: [`str`](https://doc.rust-lang.org/std/primitive.str.html "A String!")
- A http link: [rust](https://www.rust-lang.org/)
- A link with a hash: [`Vec` examples](https://doc.rust-lang.org/alloc/vec/struct.Vec.html#examples).
- A broken reference: [goes nowhere]
- A broken link: [goes somewhere](i lied)
- A link with escaped characters: [Vec \[...\] tor](https://doc.rust-lang.org/alloc/vec/struct.Vec.html "does \"this\" work?")

##### Inter-doc links
- A link to another crate: [`glob_match`](https://docs.rs/fast-glob/1.0.0/fast_glob/fn.glob_match.html).
- A shortcut to another crate [`fast_glob::glob_match`].
- A link to a crate from github: [`indoc::indoc!`].

##### Re-exports
- A link to a struct that is re-exported: [`Reexport`].
- A link to a struct that is re-exported with `#[doc(inline)]`: [`ReexportInline`].
- A link to a struct that is re-exported from a private module: [`ReexportPrivate`].

##### Glob re-exports
Rustdoc's json glob uses put the burden of resolving exports on the user.
This is too hard: <https://github.com/rust-lang/rustdoc-types/issues/51#issuecomment-3071677482>
But we can have a naive implementation and try to not crash.

- A link to types that are glob-imported: [`MyGlobImportedStruct`], [`my_glob_imported_fn`]
- A link to types that are glob-imported with `#[doc(inline)]`: [`MyInlineGlobImportedStruct`], [`my_inline_glob_imported_fn`]
- A link to types that are glob-imported from a private module: [`MyGlobImportedStructFromPrivateMod`], [`my_glob_imported_fn_from_private_mod`]
- A link to a struct from a mutually reexporting module: `Batman` (TODO)
- A link to structs from recursively glob-reexporting modules: `StructInGlobA`, `StructInGlobB`, `StructInGlobC` (TODO)

##### Item variants
- A link to a module: [`my_module`]
- A link to an extern crate: [`alloc`]
- A link to a use: [`MyStructUse`]
- A link to a union: [`MyUnion`]
- A link to a struct: [`MyStruct`]
- A link to a struct field: [`MyStruct::my_field`]
- A link to an enum: [`MyEnum`]
- A link to a variant: [`MyEnum::MyVariant`]
- A link to a function: [`my_function`]
- A link to a trait: [`MyTrait`]
- A link to a trait alias: [`IntoString`]
- A link to an impl block is not possible
- A link to a type alias: [`MyStructAlias`]
- A link to a constant: [`MY_CONSTANT`]
- A link to a static: [`MY_STATIC`]
- A link to an extern type: [`MyExternType`]
- A link to a macro: [`my_macro`]
- A link to a proc macro: [`phf_macros::phf_map`]
- A link to a primitive: [`i32`]
- A link to an associated constant: [`MyTrait::MY_ASSOCIATED_CONSTANT`]
- A link to an associated type: [`MyTrait::MyAssociatedType`]
- A link to a proc macro attribute is not possible?
- A link to a proc macro derive: [`Debug`]
- A link to a keyword is not possible
- A link to a builtin attribute: [`derive`]
- A link to a method: [`MyStruct::my_method`]


### Features
<!-- features start -->
- **`std`** *(enabled by default)* — Some docs about std
- **`serde`** — Some docs about serde

  Multiple lines work too
- **`something_undocumented`**
- **`recurse`** — Actually used feature, enables recursive imports that will cause errors.
- **`recurse-glob`** — Actually used feature, enables recursive glob imports that will cause errors.

Here you can write documentation that goes
between the features

- **`something_else`** — Wow
<!-- features end -->

### Examples
```rust
// this is rust code
let one = 1;
let two = 2;
assert_eq!(one + two, 3);
```

```rust
// this is rust code as well
"hello" + "world"
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

Test if ignoring lines work.
```rust
#[derive(Debug)] // don't ignore this line
struct Foo {
   foo: i32
}

  #[derive(Debug)] // don't ignore this line
struct Bar;

let s = "foo
# bar # baz";
assert_eq!(s, "foo\n# bar # baz");

let s = "foo
## bar # baz";
assert_eq!(s, "foo\n## bar # baz");
```

Test if ignoring lines work for indented code blocks.

```rust
#[derive(Debug)] // don't ignore this line
struct Foo {
    foo: i32
}

  #[derive(Debug)] // don't ignore this line
struct Bar;

let s = "foo
# bar # baz";
assert_eq!(s, "foo\n# bar # baz");

let s = "foo
## bar # baz";
assert_eq!(s, "foo\n## bar # baz");
```

Test if ignoring lines work in a quoted code block.

> ```rust
> assert_eq!(1 + 1, 2);
> ```

Test if ignoring lines work in a listed code block.

- ```rust
  assert_eq!(1 + 1, 2);
  ```
- ```rust
  assert_eq!(1 + 1, 2);
  ```


[`phf_macros::phf_map`]: https://docs.rs/phf_macros/0.12.1/phf_macros/macro.phf_map.html
[`my_module`]: https://docs.rs/test-crate/0.0.0/test_crate/my_module/index.html
[`my_macro`]: https://docs.rs/test-crate/0.0.0/test_crate/macro.my_macro.html
[`my_inline_glob_imported_fn`]: https://docs.rs/test-crate/0.0.0/test_crate/fn.my_inline_glob_imported_fn.html
[`my_glob_imported_fn`]: https://docs.rs/test-crate/0.0.0/test_crate/to_be_glob_imported/fn.my_glob_imported_fn.html
[`my_glob_imported_fn_from_private_mod`]: https://docs.rs/test-crate/0.0.0/test_crate/fn.my_glob_imported_fn_from_private_mod.html
[`my_function`]: https://docs.rs/test-crate/0.0.0/test_crate/fn.my_function.html
[`indoc::indoc!`]: https://docs.rs/indoc/2.0.6/indoc/macro.indoc.html
[`i32`]: https://doc.rust-lang.org/std/primitive.i32.html
[`fast_glob::glob_match`]: https://docs.rs/fast-glob/1.0.0/fast_glob/fn.glob_match.html
[`derive`]: https://doc.rust-lang.org/core/macros/builtin/attr.derive.html
[`alloc`]: https://doc.rust-lang.org/alloc/index.html
[`Vec`]: https://doc.rust-lang.org/alloc/vec/struct.Vec.html
[`Reexport`]: https://docs.rs/test-crate/0.0.0/test_crate/reexport/struct.Reexport.html
[`ReexportPrivate`]: https://docs.rs/test-crate/0.0.0/test_crate/struct.ReexportPrivate.html
[`ReexportInline`]: https://docs.rs/test-crate/0.0.0/test_crate/struct.ReexportInline.html
[`MyUnion`]: https://docs.rs/test-crate/0.0.0/test_crate/union.MyUnion.html
[`MyTrait`]: https://docs.rs/test-crate/0.0.0/test_crate/trait.MyTrait.html
[`MyTrait::MyAssociatedType`]: https://docs.rs/test-crate/0.0.0/test_crate/trait.MyTrait.html#associatedtype.MyAssociatedType
[`MyTrait::MY_ASSOCIATED_CONSTANT`]: https://docs.rs/test-crate/0.0.0/test_crate/trait.MyTrait.html#associatedconstant.MY_ASSOCIATED_CONSTANT
[`MyStruct`]: https://docs.rs/test-crate/0.0.0/test_crate/struct.MyStruct.html
[`MyStructUse`]: https://docs.rs/test-crate/0.0.0/test_crate/struct.MyStruct.html
[`MyStructAlias`]: https://docs.rs/test-crate/0.0.0/test_crate/type.MyStructAlias.html
[`MyStruct::my_method`]: https://docs.rs/test-crate/0.0.0/test_crate/struct.MyStruct.html#method.my_method
[`MyStruct::my_field`]: https://docs.rs/test-crate/0.0.0/test_crate/struct.MyStruct.html#structfield.my_field
[`MyInlineGlobImportedStruct`]: https://docs.rs/test-crate/0.0.0/test_crate/struct.MyInlineGlobImportedStruct.html
[`MyGlobImportedStruct`]: https://docs.rs/test-crate/0.0.0/test_crate/to_be_glob_imported/struct.MyGlobImportedStruct.html
[`MyGlobImportedStructFromPrivateMod`]: https://docs.rs/test-crate/0.0.0/test_crate/struct.MyGlobImportedStructFromPrivateMod.html
[`MyExternType`]: https://docs.rs/test-crate/0.0.0/test_crate/foreigntype.MyExternType.html
[`MyEnum`]: https://docs.rs/test-crate/0.0.0/test_crate/enum.MyEnum.html
[`MyEnum::MyVariant`]: https://docs.rs/test-crate/0.0.0/test_crate/enum.MyEnum.html#variant.MyVariant
[`MY_STATIC`]: https://docs.rs/test-crate/0.0.0/test_crate/static.MY_STATIC.html
[`MY_CONSTANT`]: https://docs.rs/test-crate/0.0.0/test_crate/constant.MY_CONSTANT.html
[`Debug`]: https://doc.rust-lang.org/core/fmt/macros/derive.Debug.html
[`ThinRope`]: https://doc.rust-lang.org/alloc/string/struct.String.html
<!-- docs rest end -->

This is after the crate docs.