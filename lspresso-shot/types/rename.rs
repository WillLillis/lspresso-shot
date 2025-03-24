use std::collections::HashMap;

use lsp_types::{
    AnnotatedTextEdit, ChangeAnnotationIdentifier, CreateFile, CreateFileOptions, DeleteFile,
    DeleteFileOptions, DocumentChangeOperation, DocumentChanges, OneOf,
    OptionalVersionedTextDocumentIdentifier, RenameFile, RenameFileOptions, ResourceOp,
    TextDocumentEdit, TextEdit, Uri, WorkspaceEdit,
};
use thiserror::Error;

use super::{
    clean_uri,
    compare::{cmp_fallback, Compare},
    CleanResponse, Empty, TestCase, TestResult,
};

impl Empty for WorkspaceEdit {}

impl CleanResponse for WorkspaceEdit {
    fn clean_response(mut self, test_case: &TestCase) -> TestResult<Self> {
        if let Some(ref mut changes) = self.changes {
            let mut new_changes = HashMap::new();
            for (uri, edits) in changes.drain() {
                let cleaned_uri = clean_uri(&uri, test_case)?;
                new_changes.insert(cleaned_uri, edits);
            }
            *changes = new_changes;
        }
        match self.document_changes {
            Some(DocumentChanges::Edits(ref mut edits)) => {
                for edit in edits {
                    edit.text_document.uri = clean_uri(&edit.text_document.uri, test_case)?;
                }
            }
            Some(DocumentChanges::Operations(ref mut ops)) => {
                for op in ops {
                    match op {
                        DocumentChangeOperation::Op(ref mut op) => match op {
                            ResourceOp::Create(ref mut create) => {
                                create.uri = clean_uri(&create.uri, test_case)?;
                            }
                            ResourceOp::Rename(ref mut rename) => {
                                rename.old_uri = clean_uri(&rename.old_uri, test_case)?;
                                rename.new_uri = clean_uri(&rename.new_uri, test_case)?;
                            }
                            ResourceOp::Delete(ref mut delete) => {
                                delete.uri = clean_uri(&delete.uri, test_case)?;
                            }
                        },
                        DocumentChangeOperation::Edit(edit) => {
                            edit.text_document.uri = clean_uri(&edit.text_document.uri, test_case)?;
                        }
                    }
                }
            }
            None => {}
        }
        Ok(self)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub struct RenameMismatchError {
    pub test_id: String,
    pub expected: WorkspaceEdit,
    pub actual: WorkspaceEdit,
}

impl std::fmt::Display for RenameMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test {}: Incorrect Rename response:", self.test_id)?;
        WorkspaceEdit::compare(f, None, &self.expected, &self.actual, 0, None)
    }
}

impl Compare for WorkspaceEdit {
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
        writeln!(f, "{padding}{name_str}WorkspaceEdit {{")?;
        <Option<HashMap<Uri, Vec<TextEdit>>>>::compare(
            f,
            Some("changes"),
            &expected.changes,
            &actual.changes,
            depth + 1,
            override_color,
        )?;
        <Option<DocumentChanges>>::compare(
            f,
            Some("document_changes"),
            &expected.document_changes,
            &actual.document_changes,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for OptionalVersionedTextDocumentIdentifier {
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
        writeln!(f, "{padding}{name_str}OptionalTextDocumentIdentifier {{")?;
        Uri::compare(
            f,
            Some("uri"),
            &expected.uri,
            &actual.uri,
            depth + 1,
            override_color,
        )?;
        <Option<i32>>::compare(
            f,
            Some("version"),
            &expected.version,
            &actual.version,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl<T, U> Compare for OneOf<T, U>
where
    T: Compare + PartialEq + std::fmt::Debug,
    U: Compare + PartialEq + std::fmt::Debug,
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
        let padding = "  ".repeat(depth);
        let name_str = name.map_or_else(String::new, |name| format!("{name}: "));
        match (expected, actual) {
            (Self::Left(expected), Self::Left(actual)) => {
                writeln!(f, "{padding}{name_str}OneOf::A (")?;
                T::compare(f, None, expected, actual, depth + 1, override_color)?;
                writeln!(f, "{padding})")?;
            }
            (Self::Right(expected), Self::Right(actual)) => {
                writeln!(f, "{padding}{name_str}OneOf::B (")?;
                U::compare(f, None, expected, actual, depth + 1, override_color)?;
                writeln!(f, "{padding})")?;
            }
            _ => cmp_fallback(f, expected, actual, depth, name, override_color)?,
        }

        Ok(())
    }
}

impl Compare for TextDocumentEdit {
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
        writeln!(f, "{padding}{name_str}TextDocumentEdit {{")?;
        OptionalVersionedTextDocumentIdentifier::compare(
            f,
            Some("text_document"),
            &expected.text_document,
            &actual.text_document,
            depth + 1,
            override_color,
        )?;
        <Vec<OneOf<TextEdit, AnnotatedTextEdit>>>::compare(
            f,
            Some("edits"),
            &expected.edits,
            &actual.edits,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for CreateFileOptions {
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
        writeln!(f, "{padding}{name_str}CreateFileOptions {{")?;
        <Option<bool>>::compare(
            f,
            Some("overwrite"),
            &expected.overwrite,
            &actual.overwrite,
            depth + 1,
            override_color,
        )?;
        <Option<bool>>::compare(
            f,
            Some("ignore_if_exists"),
            &expected.ignore_if_exists,
            &actual.ignore_if_exists,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for CreateFile {
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
        writeln!(f, "{padding}{name_str}CreateFile {{")?;
        Uri::compare(
            f,
            Some("uri"),
            &expected.uri,
            &actual.uri,
            depth + 1,
            override_color,
        )?;
        <Option<CreateFileOptions>>::compare(
            f,
            Some("options"),
            &expected.options,
            &actual.options,
            depth + 1,
            override_color,
        )?;
        <Option<ChangeAnnotationIdentifier>>::compare(
            f,
            Some("annotation_id"),
            &expected.annotation_id,
            &actual.annotation_id,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for RenameFileOptions {
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
        writeln!(f, "{padding}{name_str}RenameFileOptions {{")?;
        <Option<bool>>::compare(
            f,
            Some("overwrite"),
            &expected.overwrite,
            &actual.overwrite,
            depth + 1,
            override_color,
        )?;
        <Option<bool>>::compare(
            f,
            Some("ignore_if_exists"),
            &expected.ignore_if_exists,
            &actual.ignore_if_exists,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for RenameFile {
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
        writeln!(f, "{padding}{name_str}RenameFile {{")?;
        Uri::compare(
            f,
            Some("old_uri"),
            &expected.old_uri,
            &actual.old_uri,
            depth + 1,
            override_color,
        )?;
        Uri::compare(
            f,
            Some("new_uri"),
            &expected.new_uri,
            &actual.new_uri,
            depth + 1,
            override_color,
        )?;
        <Option<RenameFileOptions>>::compare(
            f,
            Some("options"),
            &expected.options,
            &actual.options,
            depth + 1,
            override_color,
        )?;
        <Option<ChangeAnnotationIdentifier>>::compare(
            f,
            Some("annotation_id"),
            &expected.annotation_id,
            &actual.annotation_id,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for DeleteFileOptions {
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
        writeln!(f, "{padding}{name_str}DeleteFileOptions {{")?;
        <Option<bool>>::compare(
            f,
            Some("recursive"),
            &expected.recursive,
            &actual.recursive,
            depth + 1,
            override_color,
        )?;
        <Option<bool>>::compare(
            f,
            Some("ignore_if_not_exists"),
            &expected.ignore_if_not_exists,
            &actual.ignore_if_not_exists,
            depth + 1,
            override_color,
        )?;
        <Option<ChangeAnnotationIdentifier>>::compare(
            f,
            Some("annotation_id"),
            &expected.annotation_id,
            &actual.annotation_id,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for DeleteFile {
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
        writeln!(f, "{padding}{name_str}DeleteFile {{")?;
        Uri::compare(
            f,
            Some("uri"),
            &expected.uri,
            &actual.uri,
            depth + 1,
            override_color,
        )?;
        <Option<DeleteFileOptions>>::compare(
            f,
            Some("options"),
            &expected.options,
            &actual.options,
            depth + 1,
            override_color,
        )?;
        writeln!(f, "{padding}}}")?;

        Ok(())
    }
}

impl Compare for ResourceOp {
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
            (Self::Create(expected_create), Self::Create(actual_create)) => {
                writeln!(f, "{padding}{name_str}ResourceOp::Create (")?;
                CreateFile::compare(
                    f,
                    None,
                    expected_create,
                    actual_create,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            (Self::Rename(expected_rename), Self::Rename(actual_rename)) => {
                writeln!(f, "{padding}{name_str}ResourceOp::Rename (")?;
                RenameFile::compare(
                    f,
                    Some("old_uri"),
                    expected_rename,
                    actual_rename,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            (Self::Delete(expected_delete), Self::Delete(actual_delete)) => {
                writeln!(f, "{padding}{name_str}ResourceOp::Delete (")?;
                DeleteFile::compare(
                    f,
                    None,
                    expected_delete,
                    actual_delete,
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

impl Compare for DocumentChangeOperation {
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
            (Self::Op(expected_op), Self::Op(actual_op)) => {
                writeln!(f, "{padding}{name_str}DocumentChangeOperation::Op (")?;
                ResourceOp::compare(f, None, expected_op, actual_op, depth + 1, override_color)?;
                writeln!(f, "{padding})")?;
            }
            (Self::Edit(expected_edit), Self::Edit(actual_edit)) => {
                writeln!(f, "{padding}{name_str}DocumentChangeOperation::Edit (")?;
                TextDocumentEdit::compare(
                    f,
                    None,
                    expected_edit,
                    actual_edit,
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

impl Compare for DocumentChanges {
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
            (Self::Edits(expected_edits), Self::Edits(actual_edits)) => {
                writeln!(f, "{padding}{name_str}DocumentChanges::Edits (")?;
                <Vec<TextDocumentEdit>>::compare(
                    f,
                    None,
                    expected_edits,
                    actual_edits,
                    depth + 1,
                    override_color,
                )?;
                writeln!(f, "{padding})")?;
            }
            (Self::Operations(expected_ops), Self::Operations(actual_ops)) => {
                writeln!(f, "{padding}{name_str}DocumentChanges::Operations (")?;
                <Vec<DocumentChangeOperation>>::compare(
                    f,
                    None,
                    expected_ops,
                    actual_ops,
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
