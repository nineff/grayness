# Grayness

This is a proof of concept [Typst](https://typst.app/) wasm-plugin to allow simple image editing functions from within Typst, written in Rust.
It uses the [wasm-minimal-protocol](https://github.com/astrale-sharp/wasm-minimal-protocol) crate to define the plugin functions. The image editing functionality is provided by the [image](https://crates.io/crates/image) crate.

## Usage
The simplest way to use this plugin is to import the package grayness into your typst code and use it's wrapper functions:
```typst
#import "@preview/grayness:0.4.0":*
#let imagedata = read("path-to-your-picture.jpg", encoding: none)
#image-grayscale(imagedata)
```
The [manual](https://github.com/typst/packages/blob/main/packages/preview/grayness/0.3.0/manual.pdf) provides further details.

You can also use this plugin directly, e.g. if you have compiled the wasm binary yourself.

```typst
#let plg = plugin("grayness.wasm")
#let imagedata = read("path-to-your-picture.jpg", encoding: none)
#image(plg.grayscale(imagedata))
```

## Compile

To compile this plugin, you need to have a working [Rust toolchain](https://www.rust-lang.org/). Then you need to install the `wasm32-unknown-unknown` target:

```sh
rustup target add wasm32-unknown-unknown
```

Then, build the crate with this target:

```sh
cargo build --release --target wasm32-unknown-unknown
```
