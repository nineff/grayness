# Grayness

This is a proof of concept typst wasm-plugin to allow simple image editing functions from within typst, written in Rust.
It uses the [wasm-minimal-protocol](https://github.com/astrale-sharp/wasm-minimal-protocol) crate to define the plugin functions. The image editing functionality is provided by the [image](https://crates.io/crates/image) crate.

## Usage
The simplest way to use this plugin is to import the package grayness into your typst code and use it's wrapper functions:
```typst
#import "@preview/grayness:0.1.0":*
#let imagedata = read("path-to-your-picture.jpg", encoding: none)
#grayscale-image(imagedata)
```
The [manual](https://github.com/typst/packages/blob/main/packages/preview/grayness/0.1.0/manual.pdf) provides further details.

You can also use this plugin directly, e.g. if you have compiled the wasm binary yourself.

This plugin provides the following functions:

- `blur(imagebytes, sigma)`: performs a Gaussian blur on the image. Sigma is a measure of how much to blur by.

  *Warning: This operation is SLOW*

  Example usage:

  ```typst
  #let pl = plugin("gray_ness.wasm")
  #image.decode(
    pl.blur(
      read("example.jpg",encoding: none), //read raw bytes from file
      bytes("10") //specify bluring amount. Integers need to be converted to bytes from string
    )
  )

  ```

- `convert(imagebytes)`: used to display filetypes not directly supported by typst such as WebP by converting to PNG internally.

  Supported filetypes:
  - Bmp
  - Dds
  - Farbfeld
  - Gif
  - Hdr
  - Ico
  - Jpeg
  - OpenExr
  - Png
  - Pnm
  - Qoi
  - Tga
  - Tiff
  - WebP

Example usage:

```typst
#let pl = plugin("gray_ness.wasm")
#image.decode(
  pl.convert(read("example.webp",encoding: none))
)
```

- `crop(imagebytes, startx, starty, width, height)`: crop the image starting from the given x,y coordinates (top left corner) to the specified width and height from there. All values are in pixels
  
  Example Usage:

  ```typst
  #let pl = plugin("gray_ness.wasm")
  #image.decode(
    pl.crop(
      read("example.jpg",encoding: none), //read raw bytes from file
      bytes("10"), //start x location in pixels
      bytes("20"), //start y location in pixels
      bytes("100"), //width in pixels
      bytes("200") //height in pixels
    )
  )

- `grayscale(imagebytes)`: turn the image into a black-and white version of itself

  Example Usage:

  ```typst
  #let pl = plugin("gray_ness.wasm")
  #image.decode(
    pl.grayscale(
      read("example.jpg",encoding: none)
    )
  )

- `flipv(imagebytes)`: flip the image vertically
  
  Example Usage:

  ```typst
  #let pl = plugin("gray_ness.wasm")
  #image.decode(
    pl.flipv(
      read("example.jpg",encoding: none)
    )
  )

- `fliph(imagebytes)`: flip the image horizontally
  
  Example Usage:

  ```typst
  #let pl = plugin("gray_ness.wasm")
  #image.decode(
    pl.fliph(
      read("example.jpg",encoding: none)
    )
  )

- `transparency(imagebytes, alpha)`: add (or change) the transparency of the image to the level given by alpha (0-255)

  Example Usage:

  ```typst
  #let pl = plugin("gray_ness.wasm")
  #image.decode(
    pl.transparency(
      read("example.jpg",encoding: none), //formats without transparency support will be converted to PNG
   bytes("128") //50% transparent
    )
  )

## Compile

To compile this plugin, you need to have a working [Rust toolchain](https://www.rust-lang.org/). Then you need to install the `wasm32-unknown-unknown` target:

```sh
rustup target add wasm32-unknown-unknown
```

Then, build the crate with this target:

```sh
cargo build --release --target wasm32-unknown-unknown
```
