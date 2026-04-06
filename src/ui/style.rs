use iced::{
    Background, Color,
    widget::{button, checkbox, container},
    Theme,
};

/// Transparent, borderless button - all visual state handled externally.
pub struct TabButton;

impl button::StyleSheet for TabButton {
    type Style = Theme;

    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: None,
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 0.0.into(),
            },
            text_color: Color::BLACK,
            ..Default::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        self.active(style)
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        self.active(style)
    }
}

/// Filled orange button with white text.
pub struct FilledButton;

impl button::StyleSheet for FilledButton {
    type Style = Theme;

    fn active(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color {
                r: 230.0 / 255.0,
                g: 77.0 / 255.0,
                b: 31.0 / 255.0,
                a: 1.0,
            })),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 4.0.into(),
            },
            text_color: Color::WHITE,
            ..Default::default()
        }
    }

    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color {
                r: 200.0 / 255.0,
                g: 60.0 / 255.0,
                b: 20.0 / 255.0,
                a: 1.0,
            })),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 4.0.into(),
            },
            text_color: Color::WHITE,
            ..Default::default()
        }
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        self.hovered(style)
    }
}

/// White background container.
pub struct WhiteBar;

impl container::StyleSheet for WhiteBar {
    type Style = Theme;

    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color::WHITE)),
            ..Default::default()
        }
    }
}

/// Light grey background container.
pub struct GrayBar;

impl container::StyleSheet for GrayBar {
    type Style = Theme;

    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color {
                r: 0.93,
                g: 0.93,
                b: 0.93,
                a: 1.0,
            })),
            ..Default::default()
        }
    }
}

/// Orange checkbox with white checkmark when ticked.
pub struct GraphCheckbox;

impl checkbox::StyleSheet for GraphCheckbox {
    type Style = Theme;

    fn active(&self, _: &Self::Style, is_checked: bool) -> checkbox::Appearance {
        if is_checked {
            checkbox::Appearance {
                background: Background::Color(Color { r: 0.902, g: 0.302, b: 0.122, a: 1.0 }),
                icon_color: Color::WHITE,
                border: iced::Border {
                    color: Color { r: 0.902, g: 0.302, b: 0.122, a: 1.0 },
                    radius: 3.0.into(),
                    width: 1.5,
                },
                text_color: None,
            }
        } else {
            checkbox::Appearance {
                background: Background::Color(Color::WHITE),
                icon_color: Color::WHITE,
                border: iced::Border {
                    color: Color { r: 0.65, g: 0.65, b: 0.65, a: 1.0 },
                    radius: 3.0.into(),
                    width: 1.5,
                },
                text_color: None,
            }
        }
    }

    fn hovered(&self, style: &Self::Style, is_checked: bool) -> checkbox::Appearance {
        let mut a = self.active(style, is_checked);
        if is_checked {
            a.background = Background::Color(Color { r: 0.78, g: 0.24, b: 0.08, a: 1.0 });
            a.border.color = Color { r: 0.78, g: 0.24, b: 0.08, a: 1.0 };
        } else {
            a.border.color = Color { r: 0.902, g: 0.302, b: 0.122, a: 1.0 };
        }
        a
    }
}

/// Solid-colour indicator bar (tab active/hover underline) or filled circle dot.
pub struct Indicator(pub Color);

impl container::StyleSheet for Indicator {
    type Style = Theme;

    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.0)),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        }
    }
}
