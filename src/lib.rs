#![doc = include_str!("doc.md")]

pub mod instrumenting;

mod base_types;
pub use base_types::*;

#[macro_use]
pub mod layout;

mod render;
#[doc(inline)]
pub use render::*;

pub mod input;

pub mod event;
#[doc(inline)]
pub use event::Event;

mod window;
pub use window::*;

#[macro_use]
mod node;
pub use node::*;

#[macro_use]
mod component;
pub use component::*;

pub mod font_cache;

#[macro_use]
pub mod style;
#[doc(inline)]
pub use style::{set_current_style, Style, Styled};

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
pub use open_iconic::Icon;
