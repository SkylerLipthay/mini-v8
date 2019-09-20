# MiniV8

MiniV8 is a minimal embedded [V8 JavaScript engine](https://v8.dev/) wrapper for Rust.

## Quick tour

```rust
extern crate mini_v8;

use mini_v8::{MiniV8, Array, Object, Function};

fn main() {
    // A `MiniV8` is a V8 context that can execute JavaScript.
    let mv8 = MiniV8::new();

    // JavaScript can be evaluated and transformed into various Rust types.
    let value: usize = mv8.eval("2 + 2").unwrap();
    assert_eq!(value, 4);
    let value: String = mv8.eval("`Two plus two is ${2 + 2}`").unwrap();
    assert_eq!(value, "Two plus two is 4".to_string());

    // JavaScript objects can be directly manipulated, without eagerly converting into Rust.
    let array: Array = mv8.eval("[123, 'abc']").unwrap();
    let element: String = array.get(1).unwrap();
    assert_eq!(element, "abc".to_string());
    array.set(0, 456).unwrap();

    // JavaScript values can be created directly, without using `mv8.eval` as above.
    let object: Object = mv8.create_object();
    let js_string = mv8.create_string("This string is owned by JavaScript!");
    object.set("someString", js_string).unwrap();

    // Rust functions can be passed into JavaScript.
    let rust_add = mv8.create_function(|inv| {
        let (a, b): (f64, f64) = inv.args.into(inv.mv8)?;
        Ok(a + b)
    });
    // Like any other value, these functions can be bound as properties of an object.
    //
    // (Notice: Cloning values just creates a new reference to the value, similar to JavaScript's
    // own object referencing semantics.)
    object.set("add", rust_add.clone()).unwrap();

    // JavaScript functions can be passed into Rust.
    let js_add: Function = mv8.eval("(a, b) => a + b").unwrap();
    // Functions can be called from within Rust.
    let value: f64 = rust_add.call((1, 2)).unwrap();
    assert_eq!(value, 3.0);
    let value: f64 = js_add.call((1, 2)).unwrap();
    assert_eq!(value, 3.0);
}
```

## Other features

* Custom user data can be bound to a `MiniV8` (see `MiniV8::set_user_data`). This is useful for storing state between embedded Rust function calls.
* All kinds of standard Rust types can be passed in and out of the JavaScript environment (the number types, `String`, `Vec`, `BTreeMap`, `HashSet`, etc.). You can define a conversion interface for your own types, too. See `ToValue`/`FromValue` and `src/conversion.rs` for more information.

## Building

Building V8 is notoriously difficult so for the purposes of development I've fallen back on using [a pre-built V8](https://rubygems.org/gems/libv8/versions/) and crudely linking in `libv8_monolith`. This is what running the REPL example looks like for me:

```bash
$ V8_PATH=/path/to/ruby/gems/libv8-7.3.492.27.1-x86_64-linux/vendor/v8 cargo run --release --example repl
```

The build process obviously requires some serious attention. See `build.rs` and [issue #1](https://github.com/SkylerLipthay/mini-v8/issues/1) for more information about this situation. I'm sure [v8-rs](https://github.com/dflemstr/v8-rs/blob/master/v8-sys/build.rs) could provide inspiration here.

V8 7.3.492.27.1 is the only tested library version. Other major versions likely have compatibility issues.

## Prior art

MiniV8 is inspired by the [MiniRacer](https://github.com/discourse/mini_racer) Ruby gem, which implements a minimal bridge with V8. From its README: "This [minimal design] reduces the surface area making upgrading [V8] much simpler and exhaustive testing simpler." Contrast this with the ambitious [v8-rs](https://github.com/dflemstr/v8-rs) crate, which remains unmaintained because "the maintenance burden is too high."

This work is a companion to my own [ducc](https://github.com/SkylerLipthay/ducc) crate, which provides a minimal wrapper around the [Duktape](https://duktape.org/) JavaScript engine.

## Purpose

When utilizing any FFI, it's significantly easier to select a subset of the entire source library than to attempt to map one-to-one all of its constructs. This of course means sacrificing features and perhaps performance, but allows for flexibility during API design leading to more idiomatic code in the target language.

It's clear that I chose the "minimal bridge" model for MiniV8 (and ducc for that matter). If you're looking to take advantage of all of V8's many internal constructs then MiniV8 is not for you. If you're looking to ergonomically execute scripts with one of the fastest JavaScript engines there is from within Rust then MiniV8 might be for you.

## Shortcomings

* I'm out of practice with C++ and the V8 API is not perfectly documented, so `src/ffi.cc` deserves scrutiny and revision.
* JavaScript was more fun when we pretended that types weren't useful. MiniV8 only implements a minimal bridge for the full set of types that modern ECMAScript offers. Perhaps the current `Value` bridge could be expanded to support a few more special object types (`Uint8Array` seems useful).
* The `Error` type is very limited. Currently there's no way to pass a script or function name into JavaScript for error reporting. I generally believe I was onto something with [`ducc::Error`](https://github.com/SkylerLipthay/ducc/blob/dcc14aff/ducc/src/error.rs), but I've made little effort to reproduce that interface or continue in that direction.
* Like MiniRacer (and ducc to an extent), I would like to see support for execution canceling (timeouts) and memory usage limitation.
