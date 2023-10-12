use paddle::quicksilver_compat::Color;

pub const ACCENT: Color = POPPY;
pub const NEUTRAL: Color = MINT;
pub const SHADE: Color = SILVER;
pub const MAIN: Color = BLUE;
pub const NEUTRAL_DARK: Color = DARK_BLUE;

/// Same as NEUTRAL_DARK
pub const CSS_FONT_DARK: &str = "#1D3354";
/// Same as NEUTRAL
pub const CSS_FONT_LIGHT: &str = "#E9FFF9";

const POPPY: Color = Color::new(214.0 / 255.0, 64.0 / 255.0, 69.0 / 255.0);
const MINT: Color = Color::new(233.0 / 255.0, 255.0 / 255.0, 249.0 / 255.0);
const SILVER: Color = Color::new(158.0 / 255.0, 216.0 / 255.0, 219.0 / 255.0);
const BLUE: Color = Color::new(70.0 / 255.0, 117.0 / 255.0, 153.0 / 255.0);
const DARK_BLUE: Color = Color::new(29.0 / 255.0, 51.0 / 255.0, 84.0 / 255.0);
