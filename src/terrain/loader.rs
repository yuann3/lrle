
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

/// Parse a single value which can be "height" or "height,0xRRGGBB"
fn parse_value(s: &str, line: usize) -> Result<(f32, Option<u32>), LoadError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(LoadError::ParseError {
            line,
            message: "empty value".to_string(),
        });
    }

    if let Some((height_str, color_str)) = s.split_once(',') {
        let height: f32 = height_str
            .trim()
            .parse()
            .map_err(|_| LoadError::ParseError {
                line,
                message: format!("expected number, got '{}'", height_str),
            })?;

        let color_str = color_str
            .trim()
            .trim_start_matches("0x")
            .trim_start_matches("0X");
        let color = u32::from_str_radix(color_str, 16).map_err(|_| LoadError::ParseError {
            line,
            message: format!("invalid color format '{}'", color_str),
        })?;

        Ok((height, Some(color)))
    } else {
        let height: f32 = s.parse().map_err(|_| LoadError::ParseError {
            line,
            message: format!("expected number, got '{}'", s),
        })?;
        Ok((height, None))
    }
}

/// Load terrain data from a .fdf file
pub fn load_fdf<P: AsRef<Path>>(path: P) -> Result<TerrainData, LoadError> {
    let path = path.as_ref();
    let content = fs::read_to_string(path)
        .map_err(|_| LoadError::FileNotFound(path.display().to_string()))?;

    parse_fdf_content(&content)
}

/// Parse .fdf content string (useful for testing)
pub fn parse_fdf_content(content: &str) -> Result<TerrainData, LoadError> {
    let mut points: Vec<Vec<f32>> = Vec::new();
    let mut colors: Vec<Vec<u32>> = Vec::new();
    let mut has_any_color = false;
    let mut expected_width: Option<usize> = None;

    for (line_idx, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let mut row_heights: Vec<f32> = Vec::new();
        let mut row_colors: Vec<u32> = Vec::new();

        for value in line.split_whitespace() {
            let (height, color) = parse_value(value, line_idx + 1)?;
            row_heights.push(height);
            row_colors.push(color.unwrap_or(0xFFFFFF));
            if color.is_some() {
                has_any_color = true;
            }
        }

        // Check row width consistency
        if let Some(expected) = expected_width {
            if row_heights.len() != expected {
                return Err(LoadError::InconsistentRow {
                    row: line_idx + 1,
                    actual: row_heights.len(),
                    expected,
                });
            }
        } else {
            expected_width = Some(row_heights.len());
        }

        points.push(row_heights);
        colors.push(row_colors);
    }

    if points.is_empty() {
        return Err(LoadError::EmptyFile);
    }

    let colors = if has_any_color { Some(colors) } else { None };
    Ok(TerrainData::new(points, colors))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_fdf() {
        let content = "0 1 2\n3 4 5";
        let terrain = parse_fdf_content(content).unwrap();

        assert_eq!(terrain.width, 3);
        assert_eq!(terrain.height, 2);
        assert_eq!(terrain.points[0], vec![0.0, 1.0, 2.0]);
        assert_eq!(terrain.points[1], vec![3.0, 4.0, 5.0]);
        assert!(terrain.colors.is_none());
    }

    #[test]
    fn test_parse_with_colors() {
        let content = "0,0xFF0000 1,0x00FF00\n2,0x0000FF 3,0xFFFFFF";
        let terrain = parse_fdf_content(content).unwrap();

        assert!(terrain.colors.is_some());
        let colors = terrain.colors.unwrap();
        assert_eq!(colors[0][0], 0xFF0000);
        assert_eq!(colors[0][1], 0x00FF00);
    }

    #[test]
    fn test_parse_inconsistent_rows() {
        let content = "0 1 2\n3 4";
        let result = parse_fdf_content(content);

        assert!(matches!(result, Err(LoadError::InconsistentRow { .. })));
    }

    #[test]
    fn test_parse_empty_file() {
        let content = "";
        let result = parse_fdf_content(content);

        assert!(matches!(result, Err(LoadError::EmptyFile)));
    }

    #[test]
    fn test_parse_negative_heights() {
        let content = "-5 0 5\n-10 0 10";
        let terrain = parse_fdf_content(content).unwrap();

        assert_eq!(terrain.points[0][0], -5.0);
        assert_eq!(terrain.points[1][0], -10.0);
    }
}
