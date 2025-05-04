use std::collections::HashSet;

use anstyle::{AnsiColor, Color, Style};
use serde::Serialize;

pub const GREEN: Option<Color> = Some(anstyle::Color::Ansi(AnsiColor::Green));
pub const RED: Option<Color> = Some(anstyle::Color::Ansi(AnsiColor::Red));

// TODO: Our rendering logic could probably use some cleanup/fxes

fn compare_fields(
    f: &mut std::fmt::Formatter<'_>,
    indent: usize,
    key: &str,
    expected: &serde_json::Value,
    actual: &serde_json::Value,
) -> std::fmt::Result {
    let padding = "  ".repeat(indent);
    let key_render = format!("{key}: ");

    if expected == actual {
        writeln!(
            f,
            "{}",
            paint(GREEN, &format!("{padding}{key_render}{expected}"))
        )?;
    } else {
        // TODO: Pull in some sort of diffing library to make this more readable,
        // as it can be very difficult to spot what's off when comparing long strings
        let expected_render = if expected.is_string() {
            format!("\n{padding}    {expected}")
        } else {
            format!(" {expected}")
        };
        let actual_render = if actual.is_string() {
            format!("\n{padding}    {actual}")
        } else {
            format!(" {actual}")
        };
        writeln!(
                f,
                "{}",
                paint(
                    RED,
                    &format!("{padding}{key_render}\n{padding}  Expected:{expected_render}\n{padding}  Actual:{actual_render}")
                )
            )?;
    }

    std::fmt::Result::Ok(())
}

pub fn write_fields_comparison<T: Serialize>(
    f: &mut std::fmt::Formatter<'_>,
    name: &str,
    expected: &T,
    actual: &T,
    indent: usize,
) -> std::fmt::Result {
    let mut expected_value = serde_json::to_value(expected).unwrap();
    let mut actual_value = serde_json::to_value(actual).unwrap();
    let padding = "  ".repeat(indent);
    let key_render = if indent == 0 {
        String::new()
    } else {
        format!("{name}: ")
    };

    match expected_value {
        serde_json::Value::Object(ref mut map) => {
            let expected_keys: HashSet<_> = map.keys().map(|k| k.to_owned()).collect();
            map.sort_keys(); // ensure a deterministic ordering
            writeln!(f, "{padding}{key_render}{{",)?;
            for (expected_key, expected_val) in &map.clone() {
                let actual_val = actual_value
                    .get(expected_key)
                    .unwrap_or(&serde_json::Value::Null)
                    .to_owned();
                match expected_val {
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        write_fields_comparison(
                            f,
                            expected_key,
                            expected_val,
                            &actual_val,
                            indent + 1,
                        )?;
                    }
                    _ => {
                        compare_fields(f, indent + 1, expected_key, expected_val, &actual_val)?;
                    }
                }
            }
            // Display entries present in the `actual` map but not in the `expected` map
            if let Some(ref mut actual_map) = actual_value.as_object_mut() {
                actual_map.sort_keys(); // ensure a deterministic ordering
                for (actual_key, actual_val) in actual_map
                    .iter()
                    .filter(|(k, _)| !expected_keys.contains(k.as_str()))
                {
                    compare_fields(
                        f,
                        indent + 1,
                        actual_key,
                        &serde_json::Value::Null,
                        actual_val,
                    )?;
                }
            }
            writeln!(f, "{padding}}},")?;
        }
        serde_json::Value::Array(ref array) => {
            writeln!(f, "{padding}{key_render}[")?;
            for (i, expected_val) in array.iter().enumerate() {
                let actual_val = actual_value
                    .get(i)
                    .unwrap_or(&serde_json::Value::Null)
                    .to_owned();
                write_fields_comparison(f, name, expected_val, &actual_val, indent + 1)?;
            }
            // Display entries present in the `actual` array but not in the `expected` array
            for i in array.len()..actual_value.as_array().map_or(0, |a| a.len()) {
                let actual_val = actual_value
                    .get(i)
                    .unwrap_or(&serde_json::Value::Null)
                    .to_owned();
                write_fields_comparison(
                    f,
                    name,
                    &serde_json::Value::Null,
                    &actual_val,
                    indent + 1,
                )?;
            }
            writeln!(f, "{padding}],")?;
        }
        _ => compare_fields(f, indent + 1, name, &expected_value, &actual_value)?,
    }

    Ok(())
}

pub fn paint(color: Option<impl Into<Color>>, text: &str) -> String {
    let style = Style::new().fg_color(color.map(Into::into));
    format!("{style}{text}{style:#}")
}
