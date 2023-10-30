//! Utility functions/macros

/// Shortcut for
/// ```no_compile
/// if let Some(thing) = optional_thing {
///     thing.<whatever>
/// }
/// ```
/// ```no_compile
/// if_some!(optional_thing, <whatever>);
/// ```
#[macro_export]
macro_rules! if_some {
    ( $option:ident, $($function:tt)* ) => {{
        if let Some(thing) = &$option {
            thing.$($function)*
        }
    }};
}
