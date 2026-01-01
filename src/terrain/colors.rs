//! Color schemes for terrain visualization.
//!
//! Provides multiple color mapping functions for height-based coloring.

/// Available color schemes for terrain rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorScheme {
    /// Natural terrain colors: blue (water) → green → brown → white (snow)
    #[default]
    Terrain,
    /// Scientific heatmap: blue (cold/low) → cyan → green → yellow → red (hot/high)
    Heatmap,
    /// Single color with intensity based on height
    Monochrome,
}

/// Convert normalized height (0.0-1.0) to RGB color based on scheme.
pub fn height_to_color(t: f32, scheme: ColorScheme) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    match scheme {
        ColorScheme::Terrain => terrain_color(t),
        ColorScheme::Heatmap => heatmap_color(t),
        ColorScheme::Monochrome => monochrome_color(t),
    }
}

/// Natural terrain gradient: blue → cyan → green → brown → white
fn terrain_color(t: f32) -> [f32; 3] {
    if t < 0.3 {
        // Blue to cyan (water/low)
        let s = t / 0.3;
        [0.0, s * 0.5, 0.8 + s * 0.2]
    } else if t < 0.5 {
        // Cyan to green
        let s = (t - 0.3) / 0.2;
        [s * 0.2, 0.5 + s * 0.3, 1.0 - s * 0.6]
    } else if t < 0.8 {
        // Green to brown
        let s = (t - 0.5) / 0.3;
        [0.2 + s * 0.4, 0.8 - s * 0.4, 0.4 - s * 0.3]
    } else {
        // Brown to white (snow)
        let s = (t - 0.8) / 0.2;
        [0.6 + s * 0.4, 0.4 + s * 0.6, 0.1 + s * 0.9]
    }
}

/// Scientific heatmap: blue → cyan → green → yellow → red
fn heatmap_color(t: f32) -> [f32; 3] {
    if t < 0.25 {
        // Blue to cyan
        let s = t / 0.25;
        [0.0, s, 1.0]
    } else if t < 0.5 {
        // Cyan to green
        let s = (t - 0.25) / 0.25;
        [0.0, 1.0, 1.0 - s]
    } else if t < 0.75 {
        // Green to yellow
        let s = (t - 0.5) / 0.25;
        [s, 1.0, 0.0]
    } else {
        // Yellow to red
        let s = (t - 0.75) / 0.25;
        [1.0, 1.0 - s, 0.0]
    }
}

/// Grayscale based on height
fn monochrome_color(t: f32) -> [f32; 3] {
    let v = 0.1 + t * 0.9;
    [v, v, v]
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Terrain Color Tests ====================

    #[test]
    fn test_terrain_low_is_bluish() {
        let color = height_to_color(0.0, ColorScheme::Terrain);
        // Blue channel should dominate at low heights
        assert!(color[2] > color[0], "Low terrain should be bluish");
        assert!(color[2] > color[1], "Blue > Green at low heights");
    }

    #[test]
    fn test_terrain_mid_is_greenish() {
        let color = height_to_color(0.5, ColorScheme::Terrain);
        // Green channel should be prominent at mid heights
        assert!(color[1] > color[0], "Mid terrain should have strong green");
    }

    #[test]
    fn test_terrain_high_is_whitish() {
        let color = height_to_color(1.0, ColorScheme::Terrain);
        // All channels should be high (white/snow)
        assert!(color[0] > 0.9, "High terrain R should be near 1.0");
        assert!(color[1] > 0.9, "High terrain G should be near 1.0");
        assert!(color[2] > 0.9, "High terrain B should be near 1.0");
    }

    // ==================== Heatmap Color Tests ====================

    #[test]
    fn test_heatmap_low_is_blue() {
        let color = height_to_color(0.0, ColorScheme::Heatmap);
        // Blue should dominate at low values
        assert!(color[2] > color[0], "Low heatmap should be blue");
    }

    #[test]
    fn test_heatmap_high_is_red() {
        let color = height_to_color(1.0, ColorScheme::Heatmap);
        // Red should dominate at high values
        assert!(color[0] > color[2], "High heatmap should be red");
        assert!(color[0] > 0.8, "High heatmap R should be strong");
    }

    #[test]
    fn test_heatmap_mid_is_greenish() {
        let color = height_to_color(0.5, ColorScheme::Heatmap);
        // Green/yellow in the middle
        assert!(color[1] > 0.5, "Mid heatmap should have green component");
    }

    // ==================== Monochrome Color Tests ====================

    #[test]
    fn test_monochrome_low_is_dark() {
        let color = height_to_color(0.0, ColorScheme::Monochrome);
        // Should be dark at low values
        let brightness = (color[0] + color[1] + color[2]) / 3.0;
        assert!(brightness < 0.3, "Low monochrome should be dark");
    }

    #[test]
    fn test_monochrome_high_is_bright() {
        let color = height_to_color(1.0, ColorScheme::Monochrome);
        // Should be bright at high values
        let brightness = (color[0] + color[1] + color[2]) / 3.0;
        assert!(brightness > 0.7, "High monochrome should be bright");
    }

    #[test]
    fn test_monochrome_is_grayscale() {
        let color = height_to_color(0.5, ColorScheme::Monochrome);
        // All channels should be equal (grayscale)
        let diff_rg = (color[0] - color[1]).abs();
        let diff_rb = (color[0] - color[2]).abs();
        assert!(diff_rg < 0.01, "Monochrome R and G should be equal");
        assert!(diff_rb < 0.01, "Monochrome R and B should be equal");
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_clamps_below_zero() {
        let color = height_to_color(-0.5, ColorScheme::Terrain);
        let expected = height_to_color(0.0, ColorScheme::Terrain);
        assert_eq!(color, expected, "Values below 0 should clamp to 0");
    }

    #[test]
    fn test_clamps_above_one() {
        let color = height_to_color(1.5, ColorScheme::Terrain);
        let expected = height_to_color(1.0, ColorScheme::Terrain);
        assert_eq!(color, expected, "Values above 1 should clamp to 1");
    }
}
