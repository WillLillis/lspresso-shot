use std::str::FromStr;

use lsp_types::{Location, Position, Range, Uri};

/// Returns `main.dummy`
#[must_use]
pub fn get_source_path() -> String {
    "main.dummy".to_string()
}

/// For use with `test_references`.
/// Returns a different `Vec<Location>` based on `response_num`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn get_references_response(response_num: u32) -> Option<Vec<Location>> {
    let uri = Uri::from_str(&get_source_path()).unwrap();
    match response_num {
        0 => Some(vec![]),
        1 => Some(vec![Location {
            uri,
            range: Range {
                start: Position::new(1, 2),
                end: Position::new(3, 4),
            },
        }]),
        2 => Some(vec![
            Location {
                uri: uri.clone(),
                range: Range {
                    start: Position::new(1, 2),
                    end: Position::new(3, 4),
                },
            },
            Location {
                uri,
                range: Range {
                    start: Position::new(5, 6),
                    end: Position::new(7, 8),
                },
            },
        ]),
        3 => Some(vec![
            Location {
                uri: uri.clone(),
                range: Range {
                    start: Position::new(1, 2),
                    end: Position::new(3, 4),
                },
            },
            Location {
                uri: uri.clone(),
                range: Range {
                    start: Position::new(5, 6),
                    end: Position::new(7, 8),
                },
            },
            Location {
                uri,
                range: Range {
                    start: Position::new(9, 10),
                    end: Position::new(11, 12),
                },
            },
        ]),
        _ => None,
    }
}
    }
}
