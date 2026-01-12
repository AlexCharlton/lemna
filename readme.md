# lemna

[![Crates.io](https://img.shields.io/crates/v/lemna)](https://crates.io/crates/lemna)
[![Docs.rs](https://docs.rs/lemna/badge.svg)](https://docs.rs/lemna)
[![CI](https://github.com/AlexCharlton/lemna/actions/workflows/ci.yml/badge.svg)](https://github.com/AlexCharlton/lemna/actions/workflows/ci.yml)

A Reactive UI framework for Rust that's both no-std compatible and GPU-accelerated (on supported std targets).

**Features:**
- React-esque stateful UI
- Flexbox-like layout engine
- Global styling
- Configurable rendering targets
  - The GPU renderer, built on [wgpu](https://github.com/gfx-rs/wgpu) for cross-platform GPU acceleration (feature: `wgpu_renderer`)
  - The CPU renderer, built on [tiny-skia](https://github.com/linebender/tiny-skia) (feature: `cpu_renderer` or `std_cpu` when used in a std environment)
- Configurable windowing backends (baseview, winit)
- Cross-platform
- Components can be built using a combination of other components and graphical primitives.
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
  - `FileSelector`, a dialog for selecting files (feature: `file_dialogs`)
- [OpenIconic icons](https://kordamp.org/ikonli/cheat-sheet-openiconic.html) built-in (feature: `open_iconic`)
- wgpu rendering backend batches primitives together to minimize calls out to wgpu (which makes it a lot faster than things that don't do this!)
- [nih-plug](https://github.com/robbert-vdh/nih-plug) support in the lemna-nih-plug package

**What's missing:**
- More robust and more widgets (e.g. text selection support on `Text` widget)
- Custom EventInput
- Debug detection of duplicate Node keys
- Only re-view dirty nodes

**To fix:**
- MSAA doesn't seem to be picking up clear color
- wgpu transparency
  - related to the above?
- Clicking on a button that performs some action that causes the button to disappear will make it so that the cursor is stuck on PointingHand
  - Send a MouseMotion action after every view?
- Wrapping with unresolved (but solvable) siblings
  - See newly added examples
  - The unstaged changes solve one of two of the new examples, but I'm not sure how good it is.
  - Most recent chat results (unapplied) passes all tests but results in terrible layouts: What tests are missing?
- Text with trailing whitespace after other whitespace doesn't display


## Running
Select your preferred windowing backend:
```
cargo run -p lemna-baseview --example hello
```

```
cargo run -p lemna-winit --example scroll
```

See `./backends/**/**examples` for other examples. Note that winit does not handle many events. The Baseview backend is not on cargo (because baseview itself is not) but it is the most functional.

## Practical Examples
- [midi-m8](https://github.com/AlexCharlton/midi-m8/tree/master/plugin)

## Dependencies
Lemna should have very few runtime dependencies. Exceptions listed below.

wgpu on Linux requires Vulkan libraries.

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

When using wgpu's Vulkan backend (will be selected for Linux), debug builds will require the validation layer `VK_LAYER_KHRONOS_validation`.

### Build deps
[shaderc](https://crates.io/crates/shaderc) – used at build-time to compile wgpu shaders – requires CMake, git, Python, and [ninja](https://github.com/ninja-build/ninja) on Windows to be built from source. Steps it took for me to build on Windows:
```
$ git clone https://github.com/google/shaderc
$ cd shaderc
$ ./utils/git-sync-deps
$ mkdir build
$ cd build
$ cmake -G "Visual Studio 17 2022" -A x64 ..
$ cmake --build . --config Release
```

Then set the environment variable `SHADERC_LIB_DIR = "/path/to/shaderc/build/libshaderc/Release"

It's easier, however, to just run cargo from a MSVS shell. Just make sure python an ninja (`pip install ninja`) are installed.
