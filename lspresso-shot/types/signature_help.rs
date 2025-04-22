use lsp_types::{
    Documentation, ParameterInformation, ParameterLabel, SignatureHelp, SignatureInformation,
};
use thiserror::Error;

use super::{
    compare::{cmp_fallback, Compare},
    CleanResponse, Empty,
};

#[derive(Debug, Error, PartialEq, Eq)]
pub struct SignatureHelpMismatchError {
    pub test_id: String,
    pub expected: SignatureHelp,
    pub actual: SignatureHelp,
}

impl std::fmt::Display for SignatureHelpMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Test {}: Incorrect Signature Help response:",
            self.test_id
        )?;
        SignatureHelp::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

impl Empty for SignatureHelp {}
impl CleanResponse for SignatureHelp {}

impl Compare for SignatureHelp {
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
        writeln!(f, "{padding}{name_str} SignatureHelp {{")?;
        <Vec<SignatureInformation>>::compare(
            f,
            Some("signatures"),
            &expected.signatures,
            &actual.signatures,
            depth + 1,
            override_color,
        )?;
        <Option<u32>>::compare(
            f,
            Some("active_signature"),
            &expected.active_signature,
            &actual.active_signature,
            depth + 1,
            override_color,
        )?;
        <Option<u32>>::compare(
            f,
            Some("active_parameter"),
            &expected.active_parameter,
            &actual.active_parameter,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for SignatureInformation {
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
        writeln!(f, "{padding}{name_str} SignatureInformation {{")?;
        String::compare(
            f,
            Some("label"),
            &expected.label,
            &actual.label,
            depth + 1,
            override_color,
        )?;
        <Option<Documentation>>::compare(
            f,
            Some("documentation"),
            &expected.documentation,
            &actual.documentation,
            depth + 1,
            override_color,
        )?;
        <Option<Vec<ParameterInformation>>>::compare(
            f,
            Some("parameters"),
            &expected.parameters,
            &actual.parameters,
            depth + 1,
            override_color,
        )?;
        <Option<u32>>::compare(
            f,
            Some("active_parameter"),
            &expected.active_parameter,
            &actual.active_parameter,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for ParameterInformation {
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
        writeln!(f, "{padding}{name_str} ParameterInformation {{")?;
        <ParameterLabel>::compare(
            f,
            Some("label"),
            &expected.label,
            &actual.label,
            depth + 1,
            override_color,
        )?;
        <Option<Documentation>>::compare(
            f,
            Some("documentation"),
            &expected.documentation,
            &actual.documentation,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for ParameterLabel {
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
            (Self::Simple(expected_str), Self::Simple(actual_str)) => {
                writeln!(f, "{padding}{name_str} ParameterLabel::Simple (")?;
                String::compare(f, None, expected_str, actual_str, depth + 1, override_color)?;
                writeln!(f, "{padding})")?;
            }
            (Self::LabelOffsets(expected_offsets), Self::LabelOffsets(actual_offsets)) => {
                writeln!(f, "{padding}{name_str} ParameterLabel::LabelOffsets (")?;
                <[u32; 2]>::compare(
                    f,
                    None,
                    expected_offsets,
                    actual_offsets,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            _ => cmp_fallback(f, expected, actual, depth, name, override_color)?,
        }
        writeln!(f, "{padding})")?;

        Ok(())
    }
}
