#![doc = include_str!("doc.md")]

pub mod instrumenting;

#[macro_use]
mod base_types;
pub use base_types::*;

#[macro_use]
pub mod layout;

pub mod render;
pub use render::Renderable;

pub mod input;
pub use input::Data;

pub mod event;
pub use event::*;

pub mod window;
pub use window::*;

#[macro_use]
pub mod node;
pub use node::*;

#[macro_use]
pub mod component;
pub use component::*;

mod font_cache;
pub use font_cache::{FontCache, HorizontalAlign, SectionText};

#[macro_use]
pub mod style;
pub use style::{set_current_style, Style, Styled};

mod ui;
pub use ui::*;

#[macro_use]
pub mod widgets;
pub use widgets::*;

#[doc(hidden)]
pub use lemna_macros;
#[doc(inline)]
pub use lemna_macros::{component, state_component_impl};

#[cfg(feature = "open_iconic")]
pub mod open_iconic;
pub use open_iconic::Icon;
