use ratatui::style::Color;

pub const UP: Color = Color::Rgb(240, 68, 82);
pub const DOWN: Color = Color::Rgb(49, 130, 246);
pub const ACCENT: Color = Color::Rgb(255, 215, 0);
pub const DIM: Color = Color::Rgb(120, 130, 150);
pub const SURFACE: Color = Color::Rgb(25, 28, 36);
pub const BORDER: Color = Color::Rgb(55, 65, 81);

const UP_RGB: (u8, u8, u8) = (240, 68, 82);
const DOWN_RGB: (u8, u8, u8) = (49, 130, 246);
/// Dark base used for a flat (near-zero) heatmap cell.
const HEAT_NEUTRAL: (u8, u8, u8) = (38, 42, 52);

pub fn change_color(value: f64) -> Color {
    if value >= 0.0 {
        UP
    } else {
        DOWN
    }
}

/// Background color for a heatmap cell: dark when flat, saturating toward red
/// (up) or blue (down) as the move grows. Saturates around ±3%.
pub fn heat_bg(change_pct: f64) -> Color {
    let t = (change_pct.abs() / 3.0).clamp(0.0, 1.0);
    let target = if change_pct >= 0.0 { UP_RGB } else { DOWN_RGB };
    lerp_rgb(HEAT_NEUTRAL, target, t)
}

/// Foreground color that stays readable on top of [`heat_bg`].
pub fn heat_fg(change_pct: f64) -> Color {
    if change_pct.abs() < 0.4 {
        Color::Rgb(150, 158, 174)
    } else {
        Color::Rgb(244, 246, 250)
    }
}

fn lerp_rgb(from: (u8, u8, u8), to: (u8, u8, u8), t: f64) -> Color {
    let mix = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
    Color::Rgb(mix(from.0, to.0), mix(from.1, to.1), mix(from.2, to.2))
}
