use std::collections::HashMap;

use anstyle::{AnsiColor, Color, Style};
use lsp_types::{Position, Range, Uri};
use serde::Serialize;

pub const GREEN: Option<Color> = Some(anstyle::Color::Ansi(AnsiColor::Green));
pub const RED: Option<Color> = Some(anstyle::Color::Ansi(AnsiColor::Red));

pub fn paint(color: Option<impl Into<Color>>, text: &str) -> String {
    let style = Style::new().fg_color(color.map(Into::into));
    format!("{style}{text}{style:#}")
}

// Delete this?
#[macro_export]
macro_rules! type_name {
    ($t:ty) => {{
        let full_name = type_name::<$t>();
        full_name.rsplit("::").next().unwrap_or(full_name)
    }};
}

pub fn cmp_fallback<T: std::fmt::Debug + PartialEq>(
    f: &mut std::fmt::Formatter<'_>,
    expected: &T,
    actual: &T,
    depth: usize,
    name: Option<&str>,
    override_color: Option<anstyle::Color>,
) -> std::fmt::Result {
    let padding = "  ".repeat(depth);
    let name_str = name.map_or_else(String::new, |name| format!("{name}: "));
    if expected == actual {
        writeln!(
            f,
            "{padding}{name_str}{}",
            paint(
                override_color.unwrap_or(GREEN.unwrap()).into(),
                &format!("{expected:?}")
            )
        )?;
    } else {
        writeln!(
            f,
            "{padding}{name_str}\n{}",
            paint(
                override_color.unwrap_or(RED.unwrap()).into(),
                &format!("{padding}  Expected: {expected:?}\n{padding}  Actual: {actual:?}")
            )
        )?;
    }

    Ok(())
}

pub trait Compare {
    // TODO: Add `=()` once default associated types are stabilized
    type Nested1: Compare;
    type Nested2: Compare;
    fn compare(
        f: &mut std::fmt::Formatter<'_>,
        name: Option<&str>,
        expected: &Self,
        actual: &Self,
        depth: usize,
        override_color: Option<anstyle::Color>,
    ) -> std::fmt::Result
    where
        Self: std::fmt::Debug + PartialEq,
    {
        let padding = "  ".repeat(depth);
        let name_str = name.map_or_else(String::new, |name| format!("{name}: "));
        if expected == actual {
            writeln!(
                f,
                "{padding}{name_str}:{}",
                paint(
                    override_color.unwrap_or(GREEN.unwrap()).into(),
                    &format!("{expected:?}")
                )
            )?;
        } else {
            writeln!(
                f,
                "{padding}{name_str}\n{}",
                paint(
                    override_color.unwrap_or(RED.unwrap()).into(),
                    &format!("{padding}  Expected: {expected:?}\n{padding}  Actual: {actual:?}")
                )
            )?;
        }
        Ok(())
    }
}

impl Compare for () {
    type Nested1 = ();
    type Nested2 = ();
}

impl Compare for bool {
    type Nested1 = ();
    type Nested2 = ();
}

impl Compare for serde_json::Number {
    type Nested1 = ();
    type Nested2 = ();
    fn compare(
        f: &mut std::fmt::Formatter<'_>,
        name: Option<&str>,
        expected: &Self,
        actual: &Self,
        depth: usize,
        override_color: Option<anstyle::Color>,
    ) -> std::fmt::Result {
        cmp_fallback(f, expected, actual, depth, name, override_color)
    }
}

impl Compare for serde_json::Value {
    type Nested1 = ();
    type Nested2 = ();
    #[allow(clippy::too_many_lines)]
    fn compare(
        f: &mut std::fmt::Formatter<'_>,
        name: Option<&str>,
        expected: &Self,
        actual: &Self,
        depth: usize,
        override_color: Option<anstyle::Color>,
    ) -> std::fmt::Result {
        let padding = "  ".repeat(depth);
        let name_str = name.map_or_else(String::new, |name| format!("{name}: "));
        match (expected, actual) {
            (Self::Null, Self::Null) => {
                writeln!(
                    f,
                    "{padding}{name_str}{}",
                    paint(override_color.unwrap_or(GREEN.unwrap()).into(), "null")
                )?;
            }
            (Self::Bool(expected_b), Self::Bool(actual_b)) => {
                writeln!(f, "{padding}{name_str}Value::Bool (")?;
                bool::compare(f, None, expected_b, actual_b, depth + 1, override_color)?;
                writeln!(f, "{padding})")?;
            }
            (Self::Number(expected_num), Self::Number(actual_num)) => {
                writeln!(f, "{padding}{name_str}Value::Number (")?;
                serde_json::Number::compare(
                    f,
                    None,
                    expected_num,
                    actual_num,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            (Self::String(expected_str), Self::String(actual_str)) => {
                writeln!(f, "{padding}{name_str}Value::String (")?;
                String::compare(f, name, expected_str, actual_str, depth, override_color)?;
                writeln!(f, "{padding})")?;
            }
            (Self::Array(expected_vec), Self::Array(actual_vec)) => {
                writeln!(f, "{padding}{name_str}Value::Array (")?;
                <Vec<Self>>::compare(f, name, expected_vec, actual_vec, depth, override_color)?;
                writeln!(f, "{padding})")?;
            }
            (Self::Object(expected_map), Self::Object(actual_map)) => {
                writeln!(f, "{padding}{name_str}Value::Object (")?;
                <serde_json::Map<_, _>>::compare(
                    f,
                    name,
                    expected_map,
                    actual_map,
                    depth,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            _ => cmp_fallback(f, expected, actual, depth, name, override_color)?,
        }

        Ok(())
    }
}

impl Compare for String {
    type Nested1 = ();
    type Nested2 = ();
    fn compare(
        f: &mut std::fmt::Formatter<'_>,
        name: Option<&str>,
        expected: &Self,
        actual: &Self,
        depth: usize,
        override_color: Option<anstyle::Color>,
    ) -> std::fmt::Result {
        cmp_fallback(f, expected, actual, depth, name, override_color)
    }
}

impl Compare for Position {
    type Nested1 = ();
    type Nested2 = ();
    fn compare(
        f: &mut std::fmt::Formatter<'_>,
        name: Option<&str>,
        expected: &Self,
        actual: &Self,
        depth: usize,
        override_color: Option<anstyle::Color>,
    ) -> std::fmt::Result {
        let padding = "  ".repeat(depth);
        let name_str = name.map_or_else(String::new, |name| format!("{name}: "));
        writeln!(f, "{padding}{name_str}Position {{")?;
        u32::compare(f, name, &expected.line, &actual.line, depth, override_color)?;
        u32::compare(
            f,
            name,
            &expected.character,
            &actual.character,
            depth,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;
        Ok(())
    }
}

impl Compare for u32 {
    type Nested1 = ();
    type Nested2 = ();
    fn compare(
        f: &mut std::fmt::Formatter<'_>,
        name: Option<&str>,
        expected: &Self,
        actual: &Self,
        depth: usize,
        override_color: Option<anstyle::Color>,
    ) -> std::fmt::Result {
        cmp_fallback(f, expected, actual, depth, name, override_color)
    }
}

impl Compare for i32 {
    type Nested1 = ();
    type Nested2 = ();
    fn compare(
        f: &mut std::fmt::Formatter<'_>,
        name: Option<&str>,
        expected: &Self,
        actual: &Self,
        depth: usize,
        override_color: Option<anstyle::Color>,
    ) -> std::fmt::Result {
        cmp_fallback(f, expected, actual, depth, name, override_color)
    }
}

impl Compare for Range {
    type Nested1 = ();
    type Nested2 = ();
    fn compare(
        f: &mut std::fmt::Formatter<'_>,
        name: Option<&str>,
        expected: &Self,
        actual: &Self,
        depth: usize,
        override_color: Option<anstyle::Color>,
    ) -> std::fmt::Result {
        let padding = "  ".repeat(depth);
        let name_str = name.map_or_else(String::new, |name| format!("{name}:"));
        writeln!(f, "{padding}{name_str} Range {{")?;
        Position::compare(
            f,
            Some("start"),
            &expected.start,
            &actual.start,
            depth + 1,
            override_color,
        )?;
        Position::compare(
            f,
            Some("end"),
            &expected.end,
            &actual.end,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;
        Ok(())
    }
}

impl<T> Compare for Option<T>
where
    T: std::fmt::Debug + PartialEq + Compare,
{
    type Nested1 = T;
    type Nested2 = ();
    fn compare(
        f: &mut std::fmt::Formatter<'_>,
        name: Option<&str>,
        expected: &Self,
        actual: &Self,
        depth: usize,
        override_color: Option<anstyle::Color>,
    ) -> std::fmt::Result {
        let padding = "  ".repeat(depth);
        let name_str = name.map_or_else(String::new, |name| format!("{name}: "));
        match (expected, actual) {
            (Some(expected), Some(actual)) => {
                T::compare(f, name, expected, actual, depth, override_color)?;
            }
            (None, None) => {
                writeln!(
                    f,
                    "{padding}{name_str}{}",
                    paint(override_color.unwrap_or(GREEN.unwrap()).into(), "None")
                )?;
            }
            _ => cmp_fallback(f, expected, actual, depth, name, override_color)?,
        }
        Ok(())
    }
}

impl<T> Compare for Vec<T>
where
    T: Clone + std::fmt::Debug + PartialEq + Compare,
{
    type Nested1 = T;
    type Nested2 = ();
    fn compare(
        f: &mut std::fmt::Formatter<'_>,
        name: Option<&str>,
        expected: &Self,
        actual: &Self,
        depth: usize,
        override_color: Option<anstyle::Color>,
    ) -> std::fmt::Result {
        let padding = "  ".repeat(depth);
        let name_str = name.map_or_else(String::new, |name| format!("{name}: "));
        if expected == actual {
            writeln!(
                f,
                "{padding}{name_str}{}",
                paint(
                    override_color.unwrap_or(GREEN.unwrap()).into(),
                    &format!("{expected:?}")
                )
            )?;
        } else {
            writeln!(f, "{padding}{name_str}[")?;
            let expected_len = expected.len();
            let actual_len = actual.len();
            for (i, (exp, act)) in expected.iter().zip(actual.iter()).enumerate() {
                T::compare(
                    f,
                    Some(&format!("[{i}]")),
                    exp,
                    act,
                    depth + 1,
                    override_color,
                )?;
            }
            match expected_len.cmp(&actual_len) {
                std::cmp::Ordering::Less => {
                    writeln!(f, "{padding}  Additional actual items:")?;
                    for (i, act) in actual.iter().enumerate().skip(expected_len) {
                        T::compare(
                            f,
                            Some(&format!("[{i}]")),
                            &act.clone(),
                            &act.clone(),
                            depth + 1,
                            override_color.unwrap_or(RED.unwrap()).into(),
                        )?;
                    }
                }
                std::cmp::Ordering::Equal => {}
                std::cmp::Ordering::Greater => {
                    writeln!(f, "{padding}  Additional expected items:")?;
                    for (i, exp) in expected.iter().enumerate().skip(actual_len) {
                        T::compare(
                            f,
                            Some(&format!("[{i}]")),
                            &exp.clone(),
                            &exp.clone(),
                            depth + 1,
                            override_color.unwrap_or(RED.unwrap()).into(),
                        )?;
                    }
                }
            }
            writeln!(f, "{padding}]")?;
        }
        Ok(())
    }
}

impl<T, U> Compare for serde_json::Map<T, U>
where
    T: serde::Serialize,
    U: serde::Serialize,
    Self: PartialEq + Serialize,
{
    type Nested1 = ();
    type Nested2 = ();
    fn compare(
        f: &mut std::fmt::Formatter<'_>,
        name: Option<&str>,
        expected: &Self,
        actual: &Self,
        depth: usize,
        override_color: Option<anstyle::Color>,
    ) -> std::fmt::Result {
        let padding = "  ".repeat(depth);
        let name_str = name.map_or_else(String::new, |name| format!("{name}: "));
        let expected_str = serde_json::to_string(expected).unwrap();
        if expected == actual {
            writeln!(
                f,
                "{padding}{name_str}{}",
                paint(
                    override_color.unwrap_or(GREEN.unwrap()).into(),
                    &expected_str
                )
            )?;
        } else {
            let actual_str = serde_json::to_string(actual).unwrap();
            cmp_fallback(f, &expected_str, &actual_str, depth, name, override_color)?;
        }

        Ok(())
    }
}

impl<T, U> Compare for HashMap<T, U>
where
    T: Compare + std::cmp::Eq + std::hash::Hash + std::fmt::Debug,
    U: Compare + std::cmp::PartialEq + std::fmt::Debug,
{
    type Nested1 = T;
    type Nested2 = U;
    fn compare(
        f: &mut std::fmt::Formatter<'_>,
        name: Option<&str>,
        expected: &Self,
        actual: &Self,
        depth: usize,
        override_color: Option<anstyle::Color>,
    ) -> std::fmt::Result {
        cmp_fallback(f, expected, actual, depth, name, override_color)
    }
}

impl Compare for Uri {
    type Nested1 = ();
    type Nested2 = ();
    fn compare(
        f: &mut std::fmt::Formatter<'_>,
        name: Option<&str>,
        expected: &Self,
        actual: &Self,
        depth: usize,
        override_color: Option<anstyle::Color>,
    ) -> std::fmt::Result {
        cmp_fallback(f, expected, actual, depth, name, override_color)
    }
}
