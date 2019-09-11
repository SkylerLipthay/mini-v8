# MiniV8

A minimal embedded [V8 JavaScript engine](https://v8.dev/) wrapper for Rust.

MiniV8 is inspired by the [MiniRacer](https://github.com/discourse/mini_racer) Ruby gem, which implements a minimal bridge with V8. From its README: "This reduces the surface area making upgrading v8 much simpler and exhaustive testing simpler." Contrast this with the ambitious [v8-rs](https://github.com/dflemstr/v8-rs) crate, which collapsed because "the maintenance burden is too high."

This work is a companion to my own [ducc](https://github.com/SkylerLipthay/ducc) crate, which provides a minimal wrapper around the [Duktape](https://duktape.org/) JavaScript engine.

* Duktape
  * Pros
    * Small footprint
    * Simple build process
  * Cons
    * Underwhelming performance
    * Not up-to-date with changing JavaScript standards (only really supports ES5.1 at the time of writing)
* V8
  * Pros
    * World-class performance
    * Up-to-date with changing JavaScript standards (practically as up-to-date as it gets)
  * Cons
    * Larger footprint
    * Complicated build process

This project is very much a work in progress.

## Areas of weakness

* My C++ is a little rusty and V8's embedding paradigms are not super well documented (handle scopes, isolate scopes, local contexts...?), so `src/ffi.cc` could use some attention.
* JavaScript was more fun when we pretended that types weren't useful. So, MiniV8 only implements a minimal bridge for the full set of types that modern ECMAScript offers.
* Building V8 is scary as hell so I've fallen back on using a pre-built V8 and simply linking `libv8_monolith`.
