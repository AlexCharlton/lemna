# lemna

[![Crates.io](https://img.shields.io/crates/v/lemna)](https://crates.io/crates/lemna)
[![Docs.rs](https://docs.rs/lemna/badge.svg)](https://docs.rs/lemna)

*This is an alpha quality release. Don't expect much!*

A Reactive UI framework for Rust

Features:
- React(or Elm, etc.)-esque stateful UI
- Flexbox-like layout engine
- Configurable rendering targets (currently just wgpu, which offers cross-platform GPU-accelerated rendering)
- Configurable windowing backends (winit, wx-rs)
- Cross platform
- Widgets can be built using a combination of other widgets and graphical primitives that map well to GPU renderers.
- State and render-state is cached, so state changes only trigger recompute of the relevant nodes
- Built in widgets:
  - `Div`, a scrollable container
  - `Button`, a button that supports tool tips
  - `RadioButtons`, arrays of selectable buttons
  - `Toggle`, a simple state-toggling button
  - `Text`, some text
  - `TextBox`, a box for entering text
  - `Selection`, a dropdown menu
  - `RoundedRect`, a stylable-rectangle
- OpenIconic icons built-in
- wgpu rendering backend batches primitives together to use as few calls out to wgpu (which makes it a lot faster than things that don't do this!)

What's missing:
- A way for handling global styles
- Raster (image) rendering support
- More robust widgets (e.g. text selection)


## Running
Select your preferred windowing backend:
```
cargo run -p lemna-baseview --example hello
```

```
cargo run -p lemna-wx-rs --example hello
```

```
cargo run -p lemna-winit --example scroll
```

See `./backends/**/**examples` for other examples. Note that wx-rs presently has compilation limitations on most platforms.


## TODO
- Fix fonts (and more?) after resizing
- Why is resizing so slow?
