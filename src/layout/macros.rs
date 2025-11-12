//--------------------------------
// MARK: lay! macro
//--------------------------------
#[macro_export]
macro_rules! lay {
    // Finish it
    ( @ { } -> ($($result:tt)*) ) => (
        $crate::layout::Layout {
            $($result)*
                ..Default::default()
        }
    );

    // margin
    ( @ { $(,)* margin : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                margin : bounds!($($vals)*),
        ))
    );
    ( @ { $(,)* margin_pct : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                margin : bounds_pct!($($vals)*),
        ))
    );

    // padding
    ( @ { $(,)* padding : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                padding : bounds!($($vals)*),
        ))
    );
    ( @ { $(,)* padding_pct : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                padding : bounds_pct!($($vals)*),
        ))
    );

    // position
    ( @ { $(,)* position : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                position : bounds!($($vals)*),
        ))
    );
    ( @ { $(,)* position_pct : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                position : bounds_pct!($($vals)*),
        ))
    );

    // size
    ( @ { $(,)* size : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                size : size!($($vals)*),
        ))
    );
    ( @ { $(,)* size_pct : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                size : size_pct!($($vals)*),
        ))
    );
    ( @ { $(,)* min_size : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                min_size : size!($($vals)*),
        ))
    );
    ( @ { $(,)* max_size : [$($vals:tt)+] $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                max_size : size!($($vals)*),
        ))
    );

    // Direction
    ( @ { $(,)* $param:ident : Row $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                $param : $crate::layout::Direction::Row,
        ))
    );
    ( @ { $(,)* $param:ident : Column $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                $param : $crate::layout::Direction::Column,
        ))
    );

    // PositionType
    ( @ { $(,)* $param:ident : Relative $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                $param : $crate::layout::PositionType::Relative,
        ))
    );
    ( @ { $(,)* $param:ident : Absolute $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                $param : $crate::layout::PositionType::Absolute,
        ))
    );


    // Alignment
    ( @ { $(,)* $param:ident : Start $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                $param : $crate::layout::Alignment::Start,
        ))
    );
    ( @ { $(,)* $param:ident : End $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                $param : $crate::layout::Alignment::End,
        ))
    );
    ( @ { $(,)* $param:ident : Center $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                $param : $crate::layout::Alignment::Center,
        ))
    );
    ( @ { $(,)* $param:ident : Stretch $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                $param : $crate::layout::Alignment::Stretch,
        ))
    );

    // z_index
    ( @ { $(,)* z_index : $z_index:expr, $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                z_index : Some($z_index .into()),
        ))
    );
    ( @ { $(,)* z_index : $z_index:expr} -> ($($result:tt)*) ) => (
        lay!(@ { } -> ( $($result)* z_index : Some($z_index .into()), ))
    );

    // Debug
    ( @ { $(,)* debug : $debug:expr, $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                debug : Some($debug .into()),
        ))
    );
    ( @ { $(,)* debug : $debug:expr} -> ($($result:tt)*) ) => (
        lay!(@ { } -> ( $($result)* debug : Some($debug .into()), ))
    );


    // Everything else
    ( @ { $(,)* $param:ident : $val:expr } -> ($($result:tt)*) ) => (
        lay!(@ { } -> (
            $($result)*
                $param : $val,
        ))
    );
    ( @ { $(,)* $param:ident : $val:expr, $($rest:tt)* } -> ($($result:tt)*) ) => (
        lay!(@ { $($rest)* } -> (
            $($result)*
                $param : $val,
        ))
    );
    ( @ { $(,)* } -> ($($result:tt)*) ) => (
        lay!(@ {} -> (
            $($result)*
        ))
    );


    // Entry point
    ( $( $tt:tt )* ) => (
        lay!(@ { $($tt)* } -> ())
    );
}

//--------------------------------
// MARK: bounds! macro
//--------------------------------
#[macro_export]
macro_rules! bounds {
    // One arg
    (Auto) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($all:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Px($all.into()),
            right: $crate::layout::Dimension::Px($all.into()),
            top: $crate::layout::Dimension::Px($all.into()),
            bottom: $crate::layout::Dimension::Px($all.into()),
        }
    };
    // Two args
    (Auto, $se:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Px($se.into()),
            right: $crate::layout::Dimension::Px($se.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($tb:expr, Auto) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Px($tb.into()),
            bottom: $crate::layout::Dimension::Px($tb.into()),
        }
    };
    ($tb:expr, $se:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Px($se.into()),
            right: $crate::layout::Dimension::Px($se.into()),
            top: $crate::layout::Dimension::Px($tb.into()),
            bottom: $crate::layout::Dimension::Px($tb.into()),
        }
    };
    // Three args
    ($t:expr, Auto, Auto) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Px($t),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, Auto, $b:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Px($b.into()),
        }
    };
    (Auto, $se:expr, $b:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Px($se.into()),
            right: $crate::layout::Dimension::Px($se.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Px($b.into()),
        }
    };
    ($t:expr, Auto, $b:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Px($t.into()),
            bottom: $crate::layout::Dimension::Px($b.into()),
        }
    };
    ($t:expr, $se:expr, Auto) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Px($se.into()),
            right: $crate::layout::Dimension::Px($se.into()),
            top: $crate::layout::Dimension::Px($t.into()),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($t:expr, $se:expr, $b:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Px($se.into()),
            right: $crate::layout::Dimension::Px($se.into()),
            top: $crate::layout::Dimension::Px($t.into()),
            bottom: $crate::layout::Dimension::Px($b.into()),
        }
    };
    // Four args
    (Auto, $s:expr, Auto, Auto) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Px($s.into()),
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, Auto, Auto, $e:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Px($e.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, $s:expr, Auto, $e:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Px($s.into()),
            right: $crate::layout::Dimension::Px($e.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, Auto, $b:expr, $e:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Px($e.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Px($b.into()),
        }
    };
    ($t:expr, $s:expr, Auto, Auto) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Px($s.into()),
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Px($t.into()),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($t:expr, Auto, Auto, $e:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Px($e.into()),
            top: $crate::layout::Dimension::Px($t.into()),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, $s:expr, $b:expr, $e:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Px($s.into()),
            right: $crate::layout::Dimension::Px($e.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Px($b.into()),
        }
    };
    ($t:expr, Auto, $b:expr, $e:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Px($e.into()),
            top: $crate::layout::Dimension::Px($t.into()),
            bottom: $crate::layout::Dimension::Px($b.into()),
        }
    };
    ($t:expr, $s:expr, Auto, $e:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Px($s.into()),
            right: $crate::layout::Dimension::Px($e.into()),
            top: $crate::layout::Dimension::Px($t.into()),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($t:expr, $s:expr, $b:expr, Auto) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Px($s.into()),
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Px($t.into()),
            bottom: $crate::layout::Dimension::Px($b.into()),
        }
    };
    ($t:expr, $s:expr, $b:expr, $e:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Px($s.into()),
            right: $crate::layout::Dimension::Px($e.into()),
            top: $crate::layout::Dimension::Px($t.into()),
            bottom: $crate::layout::Dimension::Px($b.into()),
        }
    };
}

#[macro_export]
macro_rules! bounds_pct {
    // One arg
    (Auto) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($all:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Pct($all.into()),
            right: $crate::layout::Dimension::Pct($all.into()),
            top: $crate::layout::Dimension::Pct($all.into()),
            bottom: $crate::layout::Dimension::Pct($all.into()),
        }
    };
    // Two args
    (Auto, $se:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Pct($se.into()),
            right: $crate::layout::Dimension::Pct($se.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($tb:expr, Auto) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Pct($tb.into()),
            bottom: $crate::layout::Dimension::Pct($tb.into()),
        }
    };
    ($tb:expr, $se:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Pct($se.into()),
            right: $crate::layout::Dimension::Pct($se.into()),
            top: $crate::layout::Dimension::Pct($tb.into()),
            bottom: $crate::layout::Dimension::Pct($tb.into()),
        }
    };
    // Three args
    ($t:expr, Auto, Auto) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, Auto, $b:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Pct($b.into()),
        }
    };
    (Auto, $se:expr, $b:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Pct($se.into()),
            right: $crate::layout::Dimension::Pct($se.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Pct($b.into()),
        }
    };
    ($t:expr, Auto, $b:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Pct($b.into()),
        }
    };
    ($t:expr, $se:expr, Auto) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Pct($se.into()),
            right: $crate::layout::Dimension::Pct($se.into()),
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($t:expr, $se:expr, $b:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Pct($se.into()),
            right: $crate::layout::Dimension::Pct($se.into()),
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Pct($b.into()),
        }
    };
    // Four args
    (Auto, $s:expr, Auto, Auto) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Pct($s.into()),
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, Auto, Auto, $e:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Pct($e.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, $s:expr, Auto, $e:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Pct($s.into()),
            right: $crate::layout::Dimension::Pct($e.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, Auto, $b:expr, $e:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Pct($e.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Pct($b.into()),
        }
    };
    ($t:expr, $s:expr, Auto, Auto) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Pct($s.into()),
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($t:expr, Auto, Auto, $e:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Pct($e.into()),
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, $s:expr, $b:expr, $e:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Pct($s.into()),
            right: $crate::layout::Dimension::Pct($e.into()),
            top: $crate::layout::Dimension::Auto,
            bottom: $crate::layout::Dimension::Pct($b.into()),
        }
    };
    ($t:expr, Auto, $b:expr, $e:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Auto,
            right: $crate::layout::Dimension::Pct($e.into()),
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Pct($b.into()),
        }
    };
    ($t:expr, $s:expr, Auto, $e:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Pct($s.into()),
            right: $crate::layout::Dimension::Pct($e.into()),
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Auto,
        }
    };
    ($t:expr, $s:expr, $b:expr, Auto) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Pct($s.into()),
            right: $crate::layout::Dimension::Auto,
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Pct($b.into()),
        }
    };
    ($t:expr, $s:expr, $b:expr, $e:expr) => {
        $crate::layout::Bounds {
            left: $crate::layout::Dimension::Pct($s.into()),
            right: $crate::layout::Dimension::Pct($e.into()),
            top: $crate::layout::Dimension::Pct($t.into()),
            bottom: $crate::layout::Dimension::Pct($b.into()),
        }
    };
}

//--------------------------------
// MARK: Other macros
//--------------------------------
#[macro_export]
macro_rules! px {
    ($val:expr) => {
        $crate::layout::Dimension::Px($val)
    };
}

#[macro_export]
macro_rules! pct {
    ($val:expr) => {
        $crate::layout::Dimension::Pct($val)
    };
}

#[macro_export]
macro_rules! size {
    ($width:expr, Auto) => {
        $crate::layout::Size {
            width: $crate::layout::Dimension::Px($width.into()),
            height: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, $height:expr) => {
        $crate::layout::Size {
            width: $crate::layout::Dimension::Auto,
            height: $crate::layout::Dimension::Px($height.into()),
        }
    };
    ($width:expr, $height:expr) => {
        $crate::layout::Size {
            width: $crate::layout::Dimension::Px($width.into()),
            height: $crate::layout::Dimension::Px($height.into()),
        }
    };
    (Auto) => {
        $crate::layout::Size {
            width: $crate::layout::Dimension::Auto,
            height: $crate::layout::Dimension::Auto,
        }
    };
    ($x:expr) => {
        $crate::layout::Size {
            width: $crate::layout::Dimension::Px($x.into()),
            height: $crate::layout::Dimension::Px($x.into()),
        }
    };
}

#[macro_export]
macro_rules! size_pct {
    ($width:expr, Auto) => {
        $crate::layout::Size {
            width: $crate::layout::Dimension::Pct($width.into()),
            height: $crate::layout::Dimension::Auto,
        }
    };
    (Auto, $height:expr) => {
        $crate::layout::Size {
            width: $crate::layout::Dimension::Auto,
            height: $crate::layout::Dimension::Pct($height.into()),
        }
    };
    ($width:expr, $height:expr) => {
        $crate::layout::Size {
            width: $crate::layout::Dimension::Pct($width.into()),
            height: $crate::layout::Dimension::Pct($height.into()),
        }
    };
    ($x:expr) => {
        $crate::layout::Size {
            width: $crate::layout::Dimension::Pct($x.into()),
            height: $crate::layout::Dimension::Pct($x.into()),
        }
    };
}
