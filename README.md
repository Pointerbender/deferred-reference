# Deferred Reference
This crate helps with creating multiple mutable references to the contents of a variable without triggering undefined behavior.
The Rust borrow rules dictate that it is undefined behavior to create more than one mutable reference to the same region,
even if the mutable reference is not used. However, this can sometimes be a tad too restrictive if the programmer knows
that two mutable references will not overlap. Using raw pointers, it is already possible to work-around the Rust borrow rules today,
but this requires wizard-like skills and in-depth knowledge of handling raw pointers and this is more prone to error than
using Rust references. With the introduction of non-lexical lifetimes in the Rust 2018 edition, the ergonomics around references 
have already significantly improved, but there are still some corner-cases where the programmer wished there was some way
to create non-overlapping mutable references into the same location (e.g. disjoint indices of a slice or array), without
resorting to manually managed raw pointers. In order to aid with this, this crate introduces the concept of a
"*deferred reference*"<sup >[1](#footnote1)</a></sup>. A deferred reference is almost exactly like a regular reference
(e.g. a `&T` or a `&mut T`), but it differs from a regular reference in the following ways:
* A deferred reference is not an actual reference, it is merely a smart pointer tied to the lifetime of the location it points to
  (regular raw pointers always have a static lifetime, and can thus become dangling if the location it points to is moved or dropped).
* It is allowed to keep multiple deferred mutable references around (as long as these are not dereferenced in a way so that
  these create an overlap between a mutable reference and another (de)reference).

## Getting started
If you're on nightly Rust, add the following dependency to your `Cargo.toml`:

```toml
[dependencies]
deferred-reference = { version = "0.1.2" }
```

This crate uses some unstable features, but it also works on stable Rust with less features.
For using this crate in stable Rust you need to disable the unstable nightly features using the `default-features` flag, like so:

```toml
[dependencies]
deferred-reference = { version = "0.1.2", default-features = false }
```

Please see the [documentation for this crate](https://docs.rs/deferred-reference) on how to get started with some concrete code examples.

## `#![no_std]` environments
This crate is entirely `#![no_std]` and does not depend on the `alloc` crate. No additional `Cargo.toml` features need to be configured
in order to support `#![no_std]` environments. This crate also does not have any dependencies in its `Cargo.toml`.

## Miri tested
This crate is extensively tested using [Miri](https://github.com/rust-lang/miri) using the `-Zmiri-track-raw-pointers` flag:
```bash
$ MIRIFLAGS="-Zmiri-track-raw-pointers" cargo miri test
```
Miri follows the [Stacked Borrows](https://github.com/rust-lang/unsafe-code-guidelines/blob/master/wip/stacked-borrows.md) model
(by Ralf Jung et al.) and so does this crate. If you happen to spot any violations of this model in this crate, feel free
to open a Github issue!

## License
This project is licensed under the MIT license. Please see the file `LICENSE.md` in the root of this project for the full license text.

## Contributing
It is encouraged to open a Github issue if you run into problems or if you have a feature request.
At this point, this project does not accept pull requests yet. Please check back later!

## Footnotes
<a name="footnote1"></a>
[1]: The concept of "deferred references" is inspired by the concept of "[Deferred Borrows](https://c1f.net/pubs/ecoop2020_defborrow.pdf)"
     authored by Chris Fallin. However, these are not entirely the same concept. These differ in the sense that deferred borrows bring
     an extension to the Rust type system (called *static path-dependent types*) and its implementation is intended to live within the
     Rust compiler, while deferred references are implemented in Rust code that is already possible today with its existing type system.
     The trade-off made here is that this requires minimal use of `unsafe` code blocks with deferred references, while deferred borrows
     would work entirely within "safe Rust" if these were to be implemented in the Rust compiler. There are also some similarities between
     the two concepts: both concepts are statically applied during compile-time and due not incur any runtime overhead. Also, with both
     approaches an actual reference is not created until the reference is actually in use (i.e. dereferenced or borrowed for an extended
     period of time).