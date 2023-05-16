use std::cell::UnsafeCell;
use std::collections::HashMap;

use crate::base_types::*;
use crate::layout::*;

// TODO Styled Derive macro
// TODO Style constructor macro

#[derive(Clone, Debug, PartialEq)]
pub enum StyleVal {
    Dimension(Dimension),
    Size(Size),
    Rect(Rect),
    Point(Point),
    Pos(Pos),
    Color(Color),
    Layout(Layout),
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
}

impl Default for Style {
    fn default() -> Self {
        // TODO styles for the crate widgets
        Self(Default::default())
    }
}

thread_local!(
    static CURRENT_STYLE: UnsafeCell<Style> = {
        UnsafeCell::new(Style::new())
    }
);

pub fn current_style<'a>() -> &'a Style {
    CURRENT_STYLE.with(|s| unsafe { s.get().as_ref().unwrap() })
}

pub fn set_current_style(s: Style) {
    CURRENT_STYLE.with(|c| unsafe { *c.get().as_mut().unwrap() = s })
}

trait Styled: Sized {
    fn name() -> &'static str;
    fn class(&self) -> Option<&'static str>;
    fn class_mut(&mut self) -> &mut Option<&'static str>;
    fn style_overrides(&self) -> &StyleOverride;
    fn style_overrides_mut(&mut self) -> &mut StyleOverride;

    fn with_class(mut self, class: &'static str) -> Self {
        *self.class_mut() = Some(class);
        self
    }

    fn override_style(mut self, parameter: &'static str, val: StyleVal) -> Self {
        self.style_overrides_mut().0.insert(parameter, val);
        self
    }

    fn style_key(&self, parameter_name: &'static str, class: Option<&'static str>) -> StyleKey {
        StyleKey {
            struct_name: Self::name(),
            parameter_name,
            class,
        }
    }

    fn style_param(&self, param: &'static str) -> Option<StyleVal> {
        if let Some(v) = self.style_overrides().0.get(param) {
            Some(v.clone())
        } else if let Some(c) = self.class() {
            if let Some(v) = current_style().get(self.style_key(param, Some(c))) {
                Some(v)
            } else {
                current_style().get(self.style_key(param, None))
            }
        } else {
            current_style().get(self.style_key(param, None))
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
    fn test_base_style_param() {
        set_current_style(test_style());

        let w = Widget::default();
        let c: Color = w.style_param("color").into();
        assert_eq!(c, Color::WHITE);
    }

    #[test]
    fn test_style_param_with_class() {
        set_current_style(test_style());

        let w = Widget::default().with_class("dark");
        let c: Color = w.style_param("color").into();
        assert_eq!(c, Color::BLACK);
    }

    #[test]
    fn test_style_param_overrides() {
        set_current_style(test_style());

        let w = Widget::default().override_style("color", Color::BLUE.into());
        let c: Color = w.style_param("color").into();
        assert_eq!(c, Color::BLUE);

        let w = Widget::default()
            .with_class("dark") // Classes should not impact outcome
            .override_style("color", Color::BLUE.into());
        let c: Color = w.style_param("color").into();
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
