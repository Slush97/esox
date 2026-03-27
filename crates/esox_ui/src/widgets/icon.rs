//! Icon widget — renders Phosphor icons as glyphs.

use esox_gfx::Color;

use crate::icon::Icon;
use crate::Ui;

impl<'f> Ui<'f> {
    /// Draw an icon at the current theme's foreground color.
    ///
    /// `size` is the icon height in pixels (icons are square).
    pub fn icon(&mut self, icon: Icon, size: f32) {
        self.icon_colored(icon, size, self.theme.fg);
    }

    /// Draw an icon with a custom color.
    pub fn icon_colored(&mut self, icon: Icon, size: f32, color: Color) {
        let rect = self.allocate_rect(size, size);
        self.text.draw_icon(
            icon,
            rect.x,
            rect.y,
            size,
            color,
            self.frame,
            self.gpu,
            self.resources,
        );
    }
}
