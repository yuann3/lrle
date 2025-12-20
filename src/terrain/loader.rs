use std::fs;
use std::path::Path;
use thiserror::Error;

use super::TerrainData;

#[derive(Error, Debug)]
pub enum LoadError {
    #[error("Cannot open file: {0}")]
    FileNotFound(String),
    #[error("Parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },
    #[error("Row {row} has {actual} values, expected {expected}")]
    InconsistentRow {
        row: usize,
        actual: usize,
        expected: usize,
    },
    #[error("File is empty")]
    EmptyFile,
}

fn parse_value(s: &str, line: usize) -> Result<(f32, Option<u32>), LoadError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(LoadError::ParseError {
            line,
            message: "empty value".to_string(),
        });
    }
}
