use std::str::FromStr;

use lsp_types::{Location, Position, Range, Uri};

/// Returns `main.dummy`
#[must_use]
pub fn get_source_path() -> String {
    "main.dummy".to_string()
}

/// Returns a different `Vec<Location>` based on `response_num`.
///
/// # Panics
///
/// This function will not panic
#[must_use]
pub fn get_references_response(response_num: u32) -> Vec<Location> {
    let uri = Uri::from_str(&get_source_path()).unwrap();
    match response_num {
        0 => vec![],
        1 => vec![Location {
            uri,
            range: Range {
                start: Position::new(1, 2),
                end: Position::new(3, 4),
            },
        }],
        2 => vec![
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
        ],
        _ => vec![
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
        ],
    }
}
