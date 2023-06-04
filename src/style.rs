use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::base_types::*;
use crate::font_cache::HorizontalAlign;
use crate::layout::*;
use crate::widgets::{HorizontalPosition, VerticalPosition};

#[derive(Clone, Debug, PartialEq)]
pub enum StyleVal {
    Dimension(Dimension),
    Size(Size),
    Rect(Rect),
    Point(Point),
    Pos(Pos),
    Color(Color),
    Layout(Layout),
    HorizontalAlign(HorizontalAlign),
    HorizontalPosition(HorizontalPosition),
    VerticalPosition(VerticalPosition),
    Float(f64),
    Int(u32),
    Bool(bool),
    String(&'static str),
} // Impls below

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StyleKey {
    struct_name: &'static str,
    parameter_name: &'static str,
    class: Option<&'static str>, // TODO should this be an array?
}

impl StyleKey {
    pub fn new(
        struct_name: &'static str,
        parameter_name: &'static str,
        class: Option<&'static str>,
    ) -> Self {
        Self {
            struct_name,
            parameter_name,
            class,
        }
    }
}

type StyleMap = HashMap<StyleKey, StyleVal>;
type StyleOverrideMap = HashMap<&'static str, StyleVal>;

#[derive(Clone, Debug, PartialEq)]
pub struct Style(StyleMap);
#[derive(Clone, Default, Debug)]
pub struct StyleOverride(StyleOverrideMap);

impl Style {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(mut self, k: StyleKey, v: StyleVal) -> Self {
        self.0.insert(k, v);
        self
    }

    pub fn get(&self, k: StyleKey) -> Option<StyleVal> {
        self.0.get(&k).cloned()
    }

    pub fn style(&self, component: &'static str, parameter_name: &'static str) -> Option<StyleVal> {
        let key = StyleKey {
            struct_name: component,
            parameter_name,
            class: None,
        };
        self.get(key)
    }

    pub fn style_for_class(
        &self,
        component: &'static str,
        parameter_name: &'static str,
        class: &'static str,
    ) -> Option<StyleVal> {
        let key = StyleKey {
            struct_name: component,
            parameter_name,
            class: Some(class),
        };
        self.get(key)
    }
}

impl Default for Style {
    fn default() -> Self {
        let map = StyleMap::from([
            // Button
            (
                StyleKey::new("Button", "text_color", None),
                Color::BLACK.into(),
            ),
            (StyleKey::new("Button", "font_size", None), 12.0.into()),
            (
                StyleKey::new("Button", "background_color", None),
                Color::WHITE.into(),
            ),
            (
                StyleKey::new("Button", "highlight_color", None),
                Color::LIGHT_GREY.into(),
            ),
            (
                StyleKey::new("Button", "active_color", None),
                Color::MID_GREY.into(),
            ),
            (
                StyleKey::new("Button", "border_color", None),
                Color::BLACK.into(),
            ),
            (StyleKey::new("Button", "border_width", None), 2.0.into()),
            (StyleKey::new("Button", "radius", None), 4.0.into()),
            (StyleKey::new("Button", "padding", None), 2.0.into()),
            // RadioButton
            (
                StyleKey::new("RadioButton", "text_color", None),
                Color::BLACK.into(),
            ),
            (
                StyleKey::new("RadioButton", "font_size", None),
                Color::BLACK.into(),
            ),
            (
                StyleKey::new("RadioButton", "background_color", None),
                Color::WHITE.into(),
            ),
            (
                StyleKey::new("RadioButton", "highlight_color", None),
                Color::LIGHT_GREY.into(),
            ),
            (
                StyleKey::new("RadioButton", "active_color", None),
                Color::MID_GREY.into(),
            ),
            (
                StyleKey::new("RadioButton", "border_color", None),
                Color::BLACK.into(),
            ),
            (
                StyleKey::new("RadioButton", "border_width", None),
                2.0.into(),
            ),
            (StyleKey::new("RadioButton", "radius", None), 4.0.into()),
            (StyleKey::new("RadioButton", "padding", None), 2.0.into()),
            // Select
            (
                StyleKey::new("Select", "text_color", None),
                Color::BLACK.into(),
            ),
            (StyleKey::new("Select", "font_size", None), 12.0.into()),
            (
                StyleKey::new("Select", "background_color", None),
                Color::WHITE.into(),
            ),
            (
                StyleKey::new("Select", "highlight_color", None),
                Color::LIGHT_GREY.into(),
            ),
            (
                StyleKey::new("Select", "border_color", None),
                Color::BLACK.into(),
            ),
            (
                StyleKey::new("Select", "caret_color", None),
                Color::BLACK.into(),
            ),
            (StyleKey::new("Select", "border_width", None), 2.0.into()),
            (StyleKey::new("Select", "radius", None), 4.0.into()),
            (StyleKey::new("Select", "padding", None), 2.0.into()),
            (StyleKey::new("Select", "max_height", None), 250.0.into()),
            // Toggle
            (
                StyleKey::new("Toggle", "background_color", None),
                Color::LIGHT_GREY.into(),
            ),
            (
                StyleKey::new("Toggle", "highlight_color", None),
                Color::DARK_GREY.into(),
            ),
            (
                StyleKey::new("Toggle", "active_color", None),
                Color::MID_GREY.into(),
            ),
            (
                StyleKey::new("Toggle", "border_color", None),
                Color::BLACK.into(),
            ),
            (StyleKey::new("Toggle", "border_width", None), 2.0.into()),
            // ToolTip
            (
                StyleKey::new("ToolTip", "text_color", None),
                Color::BLACK.into(),
            ),
            (StyleKey::new("ToolTip", "font_size", None), 12.0.into()),
            (
                StyleKey::new("ToolTip", "background_color", None),
                Color::WHITE.into(),
            ),
            (
                StyleKey::new("ToolTip", "border_color", None),
                Color::BLACK.into(),
            ),
            (StyleKey::new("ToolTip", "border_width", None), 2.0.into()),
            (StyleKey::new("ToolTip", "padding", None), 4.0.into()),
            // TextBox
            (StyleKey::new("TextBox", "font_size", None), 12.0.into()),
            (
                StyleKey::new("TextBox", "text_color", None),
                Color::BLACK.into(),
            ),
            (
                StyleKey::new("TextBox", "background_color", None),
                Color::WHITE.into(),
            ),
            (
                StyleKey::new("TextBox", "selection_color", None),
                Color::MID_GREY.into(),
            ),
            (
                StyleKey::new("TextBox", "cursor_color", None),
                Color::BLACK.into(),
            ),
            (
                StyleKey::new("TextBox", "border_color", None),
                Color::BLACK.into(),
            ),
            (StyleKey::new("TextBox", "border_width", None), 1.0.into()),
            (StyleKey::new("TextBox", "padding", None), 1.0.into()),
            // Text
            (StyleKey::new("Text", "size", None), 12.0.into()),
            (StyleKey::new("Text", "color", None), Color::BLACK.into()),
            (
                StyleKey::new("Text", "h_alignment", None),
                HorizontalAlign::Left.into(),
            ),
            // Scroll
            (StyleKey::new("Scroll", "x", None), false.into()),
            (StyleKey::new("Scroll", "y", None), false.into()),
            (
                StyleKey::new("Scroll", "x_bar_position", None),
                VerticalPosition::Bottom.into(),
            ),
            (
                StyleKey::new("Scroll", "y_bar_position", None),
                HorizontalPosition::Right.into(),
            ),
            (StyleKey::new("Scroll", "bar_width", None), 12.0.into()),
            (
                StyleKey::new("Scroll", "bar_background_color", None),
                Color::LIGHT_GREY.into(),
            ),
            (
                StyleKey::new("Scroll", "bar_color", None),
                Into::<Color>::into(0.7).into(),
            ),
            (
                StyleKey::new("Scroll", "bar_highlight_color", None),
                Into::<Color>::into(0.5).into(),
            ),
            (
                StyleKey::new("Scroll", "bar_active_color", None),
                Color::DARK_GREY.into(),
            ),
        ]);
        Self(map)
    }
}

fn _current_style() -> &'static Mutex<Style> {
    static CURRENT_STYLE: OnceLock<Mutex<Style>> = OnceLock::new();
    CURRENT_STYLE.get_or_init(|| Mutex::new(Style::new()))
}

pub fn set_current_style(s: Style) {
    *_current_style().lock().unwrap() = s;
}

pub fn current_style(component: &'static str, parameter_name: &'static str) -> Option<StyleVal> {
    _current_style()
        .lock()
        .unwrap()
        .style(component, parameter_name)
}

pub fn get_current_style(k: StyleKey) -> Option<StyleVal> {
    _current_style().lock().unwrap().get(k)
}

pub trait Styled: Sized {
    fn name() -> &'static str;
    fn class(&self) -> Option<&'static str>;
    fn class_mut(&mut self) -> &mut Option<&'static str>;
    fn style_overrides(&self) -> &StyleOverride;
    fn style_overrides_mut(&mut self) -> &mut StyleOverride;

    fn with_class(mut self, class: &'static str) -> Self {
        *self.class_mut() = Some(class);
        self
    }

    fn style<V: Into<StyleVal>>(mut self, parameter: &'static str, val: V) -> Self {
        self.style_overrides_mut().0.insert(parameter, val.into());
        self
    }

    fn maybe_style(mut self, parameter: &'static str, val: Option<StyleVal>) -> Self {
        if let Some(val) = val {
            self.style_overrides_mut().0.insert(parameter, val);
        }
        self
    }

    fn style_key(&self, parameter_name: &'static str, class: Option<&'static str>) -> StyleKey {
        StyleKey {
            struct_name: Self::name(),
            parameter_name,
            class,
        }
    }

    fn style_val(&self, param: &'static str) -> Option<StyleVal> {
        if let Some(v) = self.style_overrides().0.get(param) {
            Some(v.clone())
        } else if let Some(c) = self.class() {
            if let Some(v) = get_current_style(self.style_key(param, Some(c))) {
                Some(v)
            } else {
                get_current_style(self.style_key(param, None))
            }
        } else {
            get_current_style(self.style_key(param, None))
        }
    }
}

#[macro_export]
macro_rules! style {
    // Widget.color = Color::WHITE;
    // ->
    // Style::new().add(StyleKey::new("Widget", "color", None), Color::WHITE.into())

    // class.Widget.color = Color::BLACK;
    // ->
    // Style::new().add(StyleKey::new("Widget", "color", Some("class")), Color::BLACK.into())

    //Finish it
    ( @ { } -> ($($result:tt)*) ) => (
        $crate::style::Style::new() $($result)*
    );


    ( @ { $component:ident . $param:ident = $val:expr ; $($rest:tt)* } -> ($($result:tt)*) ) => (
        style!(@ { $($rest)* } -> (
            $($result)*
            .add($crate::style::StyleKey::new(stringify!($component), stringify!($param), None), $val.into())
        ))
    );

    ( @ { $class:ident . $component:ident . $param:ident = $val:expr ; $($rest:tt)* } -> ($($result:tt)*) ) => (
        style!(@ { $($rest)* } -> (
            $($result)*
            .add($crate::style::StyleKey::new(stringify!($component), stringify!($param), Some(stringify!($class))), $val.into())
        ))
    );

    // Entry point
    ( $( $tt:tt )* ) => (
        style!(@ { $($tt)* } -> ())
    );

}

// StyleVal Froms
impl From<Color> for StyleVal {
    fn from(c: Color) -> Self {
        Self::Color(c)
    }
}
impl From<StyleVal> for Color {
    fn from(v: StyleVal) -> Self {
        match v {
            StyleVal::Color(c) => c,
            x => panic!("Tried to coerce {x:?} into a Color"),
        }
    }
}
impl From<Option<StyleVal>> for Color {
    fn from(v: Option<StyleVal>) -> Self {
        match v {
            Some(StyleVal::Color(c)) => c,
            x => panic!("Tried to coerce {x:?} into a Color"),
        }
    }
}
impl From<Dimension> for StyleVal {
    fn from(c: Dimension) -> Self {
        Self::Dimension(c)
    }
}
impl From<StyleVal> for Dimension {
    fn from(v: StyleVal) -> Self {
        match v {
            StyleVal::Dimension(c) => c,
            x => panic!("Tried to coerce {x:?} into a Dimension"),
        }
    }
}
impl From<Option<StyleVal>> for Dimension {
    fn from(v: Option<StyleVal>) -> Self {
        match v {
            Some(StyleVal::Dimension(c)) => c,
            x => panic!("Tried to coerce {x:?} into a Dimension"),
        }
    }
}
impl From<Size> for StyleVal {
    fn from(c: Size) -> Self {
        Self::Size(c)
    }
}
impl From<StyleVal> for Size {
    fn from(v: StyleVal) -> Self {
        match v {
            StyleVal::Size(c) => c,
            x => panic!("Tried to coerce {x:?} into a Size"),
        }
    }
}
impl From<Option<StyleVal>> for Size {
    fn from(v: Option<StyleVal>) -> Self {
        match v {
            Some(StyleVal::Size(c)) => c,
            x => panic!("Tried to coerce {x:?} into a Size"),
        }
    }
}
impl From<Pos> for StyleVal {
    fn from(c: Pos) -> Self {
        Self::Pos(c)
    }
}
impl From<StyleVal> for Pos {
    fn from(v: StyleVal) -> Self {
        match v {
            StyleVal::Pos(c) => c,
            x => panic!("Tried to coerce {x:?} into a Pos"),
        }
    }
}
impl From<Option<StyleVal>> for Pos {
    fn from(v: Option<StyleVal>) -> Self {
        match v {
            Some(StyleVal::Pos(c)) => c,
            x => panic!("Tried to coerce {x:?} into a Pos"),
        }
    }
}
impl From<Point> for StyleVal {
    fn from(c: Point) -> Self {
        Self::Point(c)
    }
}
impl From<StyleVal> for Point {
    fn from(v: StyleVal) -> Self {
        match v {
            StyleVal::Point(c) => c,
            x => panic!("Tried to coerce {x:?} into a Point"),
        }
    }
}
impl From<Option<StyleVal>> for Point {
    fn from(v: Option<StyleVal>) -> Self {
        match v {
            Some(StyleVal::Point(c)) => c,
            x => panic!("Tried to coerce {x:?} into a Point"),
        }
    }
}
impl From<Rect> for StyleVal {
    fn from(c: Rect) -> Self {
        Self::Rect(c)
    }
}
impl From<StyleVal> for Rect {
    fn from(v: StyleVal) -> Self {
        match v {
            StyleVal::Rect(c) => c,
            x => panic!("Tried to coerce {x:?} into a Rect"),
        }
    }
}
impl From<Option<StyleVal>> for Rect {
    fn from(v: Option<StyleVal>) -> Self {
        match v {
            Some(StyleVal::Rect(c)) => c,
            x => panic!("Tried to coerce {x:?} into a Rect"),
        }
    }
}
impl From<Layout> for StyleVal {
    fn from(c: Layout) -> Self {
        Self::Layout(c)
    }
}
impl From<StyleVal> for Layout {
    fn from(v: StyleVal) -> Self {
        match v {
            StyleVal::Layout(c) => c,
            x => panic!("Tried to coerce {x:?} into a Layout"),
        }
    }
}
impl From<Option<StyleVal>> for Layout {
    fn from(v: Option<StyleVal>) -> Self {
        match v {
            Some(StyleVal::Layout(c)) => c,
            x => panic!("Tried to coerce {x:?} into a Layout"),
        }
    }
}
impl From<HorizontalAlign> for StyleVal {
    fn from(c: HorizontalAlign) -> Self {
        Self::HorizontalAlign(c)
    }
}
impl From<StyleVal> for HorizontalAlign {
    fn from(v: StyleVal) -> Self {
        match v {
            StyleVal::HorizontalAlign(c) => c,
            x => panic!("Tried to coerce {x:?} into a HorizontalAlign"),
        }
    }
}
impl From<VerticalPosition> for StyleVal {
    fn from(c: VerticalPosition) -> Self {
        Self::VerticalPosition(c)
    }
}
impl From<StyleVal> for VerticalPosition {
    fn from(v: StyleVal) -> Self {
        match v {
            StyleVal::VerticalPosition(c) => c,
            x => panic!("Tried to coerce {x:?} into a VerticalPosition"),
        }
    }
}
impl From<Option<StyleVal>> for VerticalPosition {
    fn from(v: Option<StyleVal>) -> Self {
        match v {
            Some(StyleVal::VerticalPosition(c)) => c,
            x => panic!("Tried to coerce {x:?} into a VerticalPosition"),
        }
    }
}
impl From<HorizontalPosition> for StyleVal {
    fn from(c: HorizontalPosition) -> Self {
        Self::HorizontalPosition(c)
    }
}
impl From<StyleVal> for HorizontalPosition {
    fn from(v: StyleVal) -> Self {
        match v {
            StyleVal::HorizontalPosition(c) => c,
            x => panic!("Tried to coerce {x:?} into a HorizontalPosition"),
        }
    }
}
impl From<Option<StyleVal>> for HorizontalPosition {
    fn from(v: Option<StyleVal>) -> Self {
        match v {
            Some(StyleVal::HorizontalPosition(c)) => c,
            x => panic!("Tried to coerce {x:?} into a HorizontalPosition"),
        }
    }
}
impl From<f64> for StyleVal {
    fn from(c: f64) -> Self {
        Self::Float(c)
    }
}
impl From<StyleVal> for f64 {
    fn from(v: StyleVal) -> Self {
        match v {
            StyleVal::Float(c) => c,
            x => panic!("Tried to coerce {x:?} into a float"),
        }
    }
}
impl From<u32> for StyleVal {
    fn from(c: u32) -> Self {
        Self::Int(c)
    }
}
impl From<StyleVal> for u32 {
    fn from(v: StyleVal) -> Self {
        match v {
            StyleVal::Int(c) => c,
            x => panic!("Tried to coerce {x:?} into an int"),
        }
    }
}
impl From<bool> for StyleVal {
    fn from(c: bool) -> Self {
        Self::Bool(c)
    }
}
impl From<StyleVal> for bool {
    fn from(v: StyleVal) -> Self {
        match v {
            StyleVal::Bool(c) => c,
            x => panic!("Tried to coerce {x:?} into a bool"),
        }
    }
}
impl From<&'static str> for StyleVal {
    fn from(c: &'static str) -> Self {
        Self::String(c)
    }
}
impl From<StyleVal> for &str {
    fn from(v: StyleVal) -> Self {
        match v {
            StyleVal::String(c) => c,
            x => panic!("Tried to coerce {x:?} into a string"),
        }
    }
}

impl StyleVal {
    pub fn dimension(self) -> Dimension {
        self.into()
    }

    pub fn size(self) -> Size {
        self.into()
    }

    pub fn rect(self) -> Rect {
        self.into()
    }

    pub fn point(self) -> Point {
        self.into()
    }

    pub fn pos(self) -> Pos {
        self.into()
    }

    pub fn layout(self) -> Layout {
        self.into()
    }

    pub fn horizontal_align(self) -> HorizontalAlign {
        self.into()
    }

    pub fn horizontal_position(self) -> HorizontalPosition {
        self.into()
    }

    pub fn vertical_position(self) -> VerticalPosition {
        self.into()
    }

    pub fn color(self) -> Color {
        self.into()
    }

    pub fn str(self) -> &'static str {
        self.into()
    }

    pub fn string(self) -> String {
        self.str().to_string()
    }

    pub fn f32(self) -> f32 {
        Into::<f64>::into(self) as f32
    }

    pub fn f64(self) -> f64 {
        self.into()
    }

    pub fn bool(self) -> bool {
        self.into()
    }

    pub fn u32(self) -> u32 {
        self.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Widget {
        class: Option<&'static str>,
        style_overrides: StyleOverride,
    }
    impl Styled for Widget {
        fn name() -> &'static str {
            "Widget"
        }
        fn class(&self) -> Option<&'static str> {
            self.class
        }
        fn class_mut(&mut self) -> &mut Option<&'static str> {
            &mut self.class
        }
        fn style_overrides(&self) -> &StyleOverride {
            &self.style_overrides
        }
        fn style_overrides_mut(&mut self) -> &mut StyleOverride {
            &mut self.style_overrides
        }
    }

    fn test_style() -> Style {
        Style::new()
            .add(StyleKey::new("Widget", "color", None), Color::WHITE.into())
            .add(
                StyleKey::new("Widget", "color", Some("dark")),
                Color::BLACK.into(),
            )
    }

    #[test]
    fn test_base_style_val() {
        set_current_style(test_style());

        let w = Widget::default();
        let c: Color = w.style_val("color").into();
        assert_eq!(c, Color::WHITE);
    }

    #[test]
    fn test_style_val_with_class() {
        set_current_style(test_style());

        let w = Widget::default().with_class("dark");
        let c: Color = w.style_val("color").into();
        assert_eq!(c, Color::BLACK);
    }

    #[test]
    fn test_style_val_overrides() {
        set_current_style(test_style());

        let w = Widget::default().style("color", Color::BLUE);
        let c: Color = w.style_val("color").into();
        assert_eq!(c, Color::BLUE);

        let w = Widget::default()
            .with_class("dark") // Classes should not impact outcome
            .style("color", Color::BLUE);
        let c: Color = w.style_val("color").into();
        assert_eq!(c, Color::BLUE);
    }

    #[test]
    fn test_style_macro() {
        let s = style!(
            Widget.color = Color::WHITE;
            dark.Widget.color = Color::BLACK;
        );
        assert_eq!(s, test_style());
    }
}
