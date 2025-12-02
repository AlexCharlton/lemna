//! This is the documentation for the lemna crate. See the [readme](https://github.com/AlexCharlton/lemna) for a feature breakdown as well as usage instructions.
//!
#![cfg_attr(feature = "docs",
            cfg_attr(all(),
                     doc = ::embed_doc_image::embed_image!("tut1", "docs/images/tut1.png"),
                     doc = ::embed_doc_image::embed_image!("tut2", "docs/images/tut2.png"),
                     doc = ::embed_doc_image::embed_image!("nodes", "docs/images/tutorial-nodes.svg"),
                     doc = ::embed_doc_image::embed_image!("relationships", "docs/images/tutorial-relationships.svg"),
            ))]
#![cfg_attr(
    not(feature = "docs"),
    doc = "**Doc images not enabled**. Compile with feature `doc-images` and Rust version >= 1.54 \
           to enable."
)]
//!
#![doc = include_str!("../docs/tutorial.md")]
// If the `std` feature is not enabled, set no_std
#![cfg_attr(not(feature = "std"), no_std)]

pub mod instrumenting;

mod base_types;
pub use base_types::*;

#[macro_use]
pub mod layout;

mod render;
#[doc(inline)]
pub use render::*;

pub mod input;

pub(crate) mod focus;

pub mod event;
#[doc(inline)]
pub use event::Event;

pub mod window;

#[macro_use]
mod node;
pub use node::*;

#[macro_use]
mod component;
pub use component::*;

mod font_cache;
#[doc(inline)]
pub use font_cache::*;

#[macro_use]
pub mod style;
#[doc(inline)]
pub use style::{Style, Styled};

pub mod time;

mod ui;
pub use ui::*;

#[macro_use]
pub mod widgets;

#[doc(hidden)]
pub use lemna_macros;
#[doc(inline)]
pub use lemna_macros::{component, state_component_impl};

#[cfg(feature = "open_iconic")]
pub mod open_iconic;
#[cfg(feature = "open_iconic")]
pub use open_iconic::Icon;

// Test stub window
#[cfg(feature = "docs")]
#[doc(hidden)]
pub mod lemna_baseview {
    use super::*;
    pub struct Window {}
    use super::focus::FocusState;
    use hashbrown::HashMap;

    impl Window {
        pub fn open_blocking<A>(_options: WindowOptions)
        where
            A: 'static + Component + Default + Send + Sync,
        {
            let app = A::default();
            let mut node = Node::new(Box::new(app), 0, layout::Layout::default());
            let mut references = HashMap::new();
            let mut focus_state = FocusState::default();
            node.view(None, &mut references, &mut focus_state, 0);
        }
    }

    pub struct WindowOptions {}

    impl WindowOptions {
        pub fn new<T: Into<String>>(_title: T, _dims: (u32, u32)) -> Self {
            Self {}
        }

        pub fn scale_factor(self, _scale: f32) -> Self {
            self
        }

        pub fn system_scale_factor(self) -> Self {
            self
        }

        pub fn fonts(self, mut _fonts: Vec<(String, &'static [u8])>) -> Self {
            self
        }

        pub fn resizable(self, _resizable: bool) -> Self {
            self
        }
    }
}
