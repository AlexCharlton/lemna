# lemna

[![Crates.io](https://img.shields.io/crates/v/lemna)](https://crates.io/crates/lemna)
[![Docs.rs](https://docs.rs/lemna/badge.svg)](https://docs.rs/lemna)

A Reactive UI framework for Rust

Features:
- React-esque stateful UI
- Flexbox-like layout engine
- Global styling
- Configurable rendering targets (currently just wgpu, which offers cross-platform GPU-accelerated rendering)
- Configurable windowing backends (baseview, winit, wx-rs)
- Cross-platform
- Components can be built using a combination of other components and graphical primitives that map well to GPU renderers.
- State and render-state is cached, so state changes only trigger recompute of the relevant nodes
- Built in components/widgets:
  - `Div`, a scrollable container
  - `Button`, a button that supports tool tips
  - `RadioButtons`, arrays of selectable buttons
  - `Toggle`, a simple state-toggling button
  - `Text`, some text
  - `TextBox`, a box for entering text
  - `Canvas`, for displaying raster images, including drawing to a blank canvas
  - `Selection`, a dropdown menu
  - `RoundedRect`, a stylable-rectangle
  - `FileSelector`, a dialog for selecting files
- OpenIconic icons built-in
- wgpu rendering backend batches primitives together to use few calls out to wgpu (which makes it a lot faster than things that don't do this!)
- [nih-plug](https://github.com/robbert-vdh/nih-plug) support in the lemna-nih-plug package

What's missing:
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

## Dependencies
Lemna should have very few runtime dependencies. Exceptions listed below.

The `FileSelector` widget uses [tinyfiledialogs](https://sourceforge.net/projects/tinyfiledialogs/), which will call out to the following:
```
- On unix you need one of the following:
  applescript, kdialog, zenity, matedialog, shellementary, qarma, yad,
  python (2 or 3)/tkinter/python-dbus (optional), Xdialog
  or curses dialogs (opens terminal if running without console).
- One of those is already included on most (if not all) desktops.
- In the absence of those it will use gdialog, gxmessage or whiptail
  with a textinputbox.
- If nothing is found, it switches to basic console input,
  it opens a console if needed (requires xterm + bash).
```
