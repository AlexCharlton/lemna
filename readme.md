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
- Components can be built using a combination of other components and graphical primitives that map well to GPU renderers.
- State and render-state is cached, so state changes only trigger recompute of the relevant nodes
- Built in components/widgets:
  - `Div`, a scrollable container
  - `Button`, a button that supports tool tips
  - `RadioButtons`, arrays of selectable buttons
  - `Toggle`, a simple state-toggling button
  - `Text`, some text
  - `TextBox`, a box for entering text
  - `Selection`, a dropdown menu
  - `RoundedRect`, a stylable-rectangle
- OpenIconic icons built-in
- wgpu rendering backend batches primitives together to use few calls out to wgpu (which makes it a lot faster than things that don't do this!)
- [nih-plug](https://github.com/robbert-vdh/nih-plug) support in the lemna-nih-plug package

What's missing:
- A way for handling global styles
- Raster (image) rendering support
- More robust and more widgets (e.g. text selection support on `Text` widget)


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

See `./backends/**/**examples` for other examples. Note that wx-rs presently has compilation limitations on most platforms, and winit does not handle many events. The Baseview backend is not on cargo (because baseview itself is not) but it is the most functional.

## Practical Examples
- [midi-m8](https://github.com/AlexCharlton/midi-m8/tree/master/plugin)
