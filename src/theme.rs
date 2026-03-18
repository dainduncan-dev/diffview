use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub base_fg: Color,
    pub add_bg: Color,
    pub add_fg: Color,
    pub del_bg: Color,
    pub del_fg: Color,
    pub meta_fg: Color,
    pub warn_bg: Color,
    pub warn_fg: Color,
    pub border_left: Color,
    pub border_right: Color,
    pub header_chip_bg: Color,
    pub header_chip_fg: Color,
    pub footer_fg: Color,
    pub dim_fg: Color,
}

impl Theme {
    pub fn github_dark() -> Self {
        Theme {
            base_fg: Color::Rgb(201, 209, 217),
            add_bg: Color::Rgb(14, 68, 41),
            add_fg: Color::Rgb(201, 209, 217),
            del_bg: Color::Rgb(76, 30, 30),
            del_fg: Color::Rgb(201, 209, 217),
            meta_fg: Color::Rgb(201, 209, 217),
            warn_bg: Color::Rgb(45, 29, 0),
            warn_fg: Color::Rgb(255, 193, 7),
            border_left: Color::Rgb(239, 68, 68),
            border_right: Color::Rgb(34, 197, 94),
            header_chip_bg: Color::Rgb(33, 38, 45),
            header_chip_fg: Color::Rgb(201, 209, 217),
            footer_fg: Color::Rgb(139, 148, 158),
            dim_fg: Color::Rgb(139, 148, 158),
        }
    }
}
