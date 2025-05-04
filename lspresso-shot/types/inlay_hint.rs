use lsp_types::{
    Command, InlayHint, InlayHintKind, InlayHintLabel, InlayHintLabelPart,
    InlayHintLabelPartTooltip, InlayHintTooltip, LSPAny, Location, MarkupContent, Position,
    TextEdit,
};
use thiserror::Error;

use super::{
    compare::{cmp_fallback, Compare},
    CleanResponse, Empty,
};

impl Empty for Vec<InlayHint> {}
impl CleanResponse for Vec<InlayHint> {}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct InlayHintMismatchError {
    pub test_id: String,
    pub expected: Vec<InlayHint>,
    pub actual: Vec<InlayHint>,
}

impl std::fmt::Display for InlayHintMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Inlay Hint response:", self.test_id)?;
        <Vec<InlayHint>>::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

impl Compare for InlayHint {
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
        writeln!(f, "{padding}{name_str}InlayHint {{")?;
        Position::compare(
            f,
            Some("position"),
            &expected.position,
            &actual.position,
            depth + 1,
            override_color,
        )?;
        InlayHintLabel::compare(
            f,
            Some("label"),
            &expected.label,
            &actual.label,
            depth + 1,
            override_color,
        )?;
        <Option<InlayHintKind>>::compare(
            f,
            Some("kind"),
            &expected.kind,
            &actual.kind,
            depth + 1,
            override_color,
        )?;
        <Option<Vec<TextEdit>>>::compare(
            f,
            Some("text_edits"),
            &expected.text_edits,
            &actual.text_edits,
            depth + 1,
            override_color,
        )?;
        <Option<InlayHintTooltip>>::compare(
            f,
            Some("tooltip"),
            &expected.tooltip,
            &actual.tooltip,
            depth + 1,
            override_color,
        )?;
        <Option<bool>>::compare(
            f,
            Some("padding_left"),
            &expected.padding_left,
            &actual.padding_left,
            depth + 1,
            override_color,
        )?;
        <Option<bool>>::compare(
            f,
            Some("padding_right"),
            &expected.padding_right,
            &actual.padding_right,
            depth + 1,
            override_color,
        )?;
        <Option<LSPAny>>::compare(
            f,
            Some("data"),
            &expected.data,
            &actual.data,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for InlayHintLabel {
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
        match (expected, actual) {
            (Self::String(expected), Self::String(actual)) => {
                writeln!(f, "{padding}{name_str}InlayHintLabel::String (")?;
                String::compare(f, None, expected, actual, depth + 1, override_color)?;
                writeln!(f, "{padding})")?;
            }
            (Self::LabelParts(expected), Self::LabelParts(actual)) => {
                writeln!(f, "{padding}{name_str}InlayHintLabel::String (")?;
                <Vec<InlayHintLabelPart>>::compare(
                    f,
                    None,
                    expected,
                    actual,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            _ => cmp_fallback(f, expected, actual, depth, name, override_color)?,
        }

        Ok(())
    }
}

impl Compare for InlayHintLabelPart {
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
        writeln!(f, "{padding}{name_str}InlayHintLabelPart {{")?;
        String::compare(
            f,
            Some("value"),
            &expected.value,
            &actual.value,
            depth + 1,
            override_color,
        )?;
        <Option<InlayHintLabelPartTooltip>>::compare(
            f,
            Some("tooltip"),
            &expected.tooltip,
            &actual.tooltip,
            depth + 1,
            override_color,
        )?;
        <Option<Location>>::compare(
            f,
            Some("location"),
            &expected.location,
            &actual.location,
            depth + 1,
            override_color,
        )?;
        <Option<Command>>::compare(
            f,
            Some("command"),
            &expected.command,
            &actual.command,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for InlayHintLabelPartTooltip {
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
        match (expected, actual) {
            (Self::String(expected), Self::String(actual)) => {
                writeln!(f, "{padding}{name_str}InlayHintLabelPartTooltip::String (")?;
                String::compare(f, None, expected, actual, depth + 1, override_color)?;
                writeln!(f, "{padding})")?;
            }
            (Self::MarkupContent(expected), Self::MarkupContent(actual)) => {
                writeln!(
                    f,
                    "{padding}{name_str}InlayHintLabelPartTooltip::MarkupContent ("
                )?;
                MarkupContent::compare(f, None, expected, actual, depth + 1, override_color)?;
                writeln!(f, "{padding})")?;
            }
            _ => cmp_fallback(f, expected, actual, depth, name, override_color)?,
        }
        Ok(())
    }
}

impl Compare for InlayHintKind {
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

impl Compare for InlayHintTooltip {
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
        match (expected, actual) {
            (Self::String(expected), Self::String(actual)) => {
                writeln!(f, "{padding}{name_str}InlayHintTooltip::String (")?;
                String::compare(f, None, expected, actual, depth + 1, override_color)?;
                writeln!(f, "{padding})")?;
            }
            (Self::MarkupContent(expected), Self::MarkupContent(actual)) => {
                writeln!(f, "{padding}{name_str}InlayHintTooltip::MarkupContent (")?;
                MarkupContent::compare(f, None, expected, actual, depth + 1, override_color)?;
                writeln!(f, "{padding})")?;
            }
            _ => cmp_fallback(f, expected, actual, depth, name, override_color)?,
        }
        Ok(())
    }
}
