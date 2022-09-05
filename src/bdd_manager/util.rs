#[macro_export]
macro_rules! if_some {
    ( $option:ident, $($function:tt)* ) => {{
        if let Some(thing) = &$option {
            thing.$($function)*
        }
    }};
}
