//! Card and surface container widgets — visual grouping with background + border.
//!
//! # Examples
//!
//! ```ignore
//! ui.card(|ui| {
//!     ui.heading("Settings");
//!     ui.text_input(id!("name"), &mut name, "Name…");
//! });
//!
//! ui.surface(|ui| {
//!     ui.label("Secondary content");
//! });
//! ```

use esox_gfx::{BorderRadius, Color};

use crate::layout::Rect;
use crate::paint;
use crate::theme::Elevation;
use crate::Ui;

/// Builder for a styled container with configurable bg, border, radius, and padding.
pub struct ContainerBuilder<'a, 'f> {
    ui: &'a mut Ui<'f>,
    bg: Option<Color>,
    border_color: Option<Color>,
    border_width: f32,
    radius: f32,
    pad: f32,
    elevation: Option<Elevation>,
}

impl<'a, 'f> ContainerBuilder<'a, 'f> {
    /// Set background color.
    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    /// Set border color and width.
    pub fn border(mut self, color: Color, width: f32) -> Self {
        self.border_color = Some(color);
        self.border_width = width;
        self
    }

    /// Set corner radius.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Set padding on all sides.
    pub fn padding(mut self, pad: f32) -> Self {
        self.pad = pad;
        self
    }

    /// Set elevation shadow (overrides default).
    pub fn elevation(mut self, e: Elevation) -> Self {
        self.elevation = Some(e);
        self
    }

    /// Draw the container with the given content closure.
    pub fn show(self, f: impl FnOnce(&mut Ui<'f>)) {
        let bg = self.bg.unwrap_or(self.ui.theme.bg_raised);
        let radius = BorderRadius::uniform(self.radius);
        let pad = self.pad;
        let content_spacing = self.ui.theme.content_spacing;
        let card_gap = self.ui.theme.card_gap;

        let placeholder_idx = self.ui.frame.instance_len();
        self.ui.frame.push(
            esox_gfx::ShapeBuilder::rect(0.0, 0.0, 0.0, 0.0)
                .color(Color::new(0.0, 0.0, 0.0, 0.0))
                .build(),
        );

        let start_y = self.ui.cursor.y;
        let saved_spacing = self.ui.spacing;
        self.ui.spacing = content_spacing;
        self.ui.padding(pad, f);
        self.ui.spacing = saved_spacing;
        let end_y = self.ui.cursor.y;

        let container_rect =
            Rect::new(self.ui.region.x, start_y, self.ui.region.w, end_y - start_y);

        // Replace placeholder with styled background.
        self.ui.frame.replace_instance(
            placeholder_idx,
            esox_gfx::ShapeBuilder::rect(
                container_rect.x,
                container_rect.y,
                container_rect.w,
                container_rect.h,
            )
            .color(bg)
            .border_radius(radius)
            .build(),
        );

        if let Some(border_color) = self.border_color {
            self.ui.frame.push(
                esox_gfx::ShapeBuilder::rect(
                    container_rect.x,
                    container_rect.y,
                    container_rect.w,
                    container_rect.h,
                )
                .color(border_color)
                .border_radius(radius)
                .stroke(self.border_width)
                .build(),
            );
        }

        if let Some(ref elev) = self.elevation {
            if elev.blur > 0.001 {
                // Re-draw background with shadow (replaces the placeholder).
                self.ui.frame.replace_instance(
                    placeholder_idx,
                    esox_gfx::ShapeBuilder::rect(
                        container_rect.x,
                        container_rect.y,
                        container_rect.w,
                        container_rect.h,
                    )
                    .color(bg)
                    .border_radius(radius)
                    .shadow(elev.blur, elev.dx, elev.dy)
                    .color2(elev.color)
                    .build(),
                );
            }
        }

        self.ui.cursor.y += card_gap;
    }
}

impl<'f> Ui<'f> {
    /// Create a styled container with configurable bg, border, radius, and padding.
    pub fn box_container(&mut self) -> ContainerBuilder<'_, 'f> {
        ContainerBuilder {
            bg: None,
            border_color: None,
            border_width: 1.0,
            radius: self.theme.corner_radius,
            pad: self.theme.padding,
            elevation: None,
            ui: self,
        }
    }

    /// Draw a card container — `bg_raised` background with border and padding.
    ///
    /// Cards provide visual grouping for related content sections.
    pub fn card(&mut self, f: impl FnOnce(&mut Self)) {
        self.card_colored(self.theme.bg_raised, f);
    }

    /// Draw a card container with a custom background color.
    pub fn card_colored(&mut self, bg: Color, f: impl FnOnce(&mut Self)) {
        let pad = self.theme.padding;
        let radius = self.theme.corner_radius;
        let border_color = self.theme.border;
        let content_spacing = self.theme.content_spacing;

        // Save the insert point — push a transparent placeholder for the background.
        let placeholder_idx = self.frame.instance_len();
        self.frame.push(
            esox_gfx::ShapeBuilder::rect(0.0, 0.0, 0.0, 0.0)
                .color(Color::new(0.0, 0.0, 0.0, 0.0))
                .build(),
        );

        let start_y = self.cursor.y;

        // Clip card content to prevent overflow (e.g. badges in tight rows).
        let saved_clip = self.frame.active_clip();
        let card_clip = Rect::new(self.region.x, start_y, self.region.w, self.region.h);
        let gpu_clip = match saved_clip {
            Some(prev) => {
                let prev_rect = Rect::new(prev[0], prev[1], prev[2], prev[3]);
                card_clip.intersect(&prev_rect).unwrap_or(card_clip)
            }
            None => card_clip,
        };
        self.frame.set_active_clip(Some(gpu_clip.to_clip_array()));

        let saved_spacing = self.spacing;
        self.spacing = content_spacing;
        self.padding(pad, f);
        self.spacing = saved_spacing;
        let end_y = self.cursor.y;

        // Restore clip before drawing border (border should not be clipped).
        self.frame.set_active_clip(saved_clip);

        let card_rect = Rect::new(self.region.x, start_y, self.region.w, end_y - start_y);

        // Replace placeholder with the correctly-sized background.
        self.frame.replace_instance(
            placeholder_idx,
            esox_gfx::ShapeBuilder::rect(card_rect.x, card_rect.y, card_rect.w, card_rect.h)
                .color(bg)
                .border_radius(esox_gfx::BorderRadius::uniform(radius))
                .build(),
        );

        // Subtle border for edge definition.
        paint::draw_rounded_border(self.frame, card_rect, border_color, radius);

        // Add card_gap after the card for breathing room between siblings.
        self.cursor.y += self.theme.card_gap;
    }

    /// Draw a surface container — `bg_surface` background with padding, no border.
    ///
    /// Surfaces are subtler than cards, good for secondary grouping.
    pub fn surface(&mut self, f: impl FnOnce(&mut Self)) {
        let pad = self.theme.padding;
        let radius = self.theme.corner_radius;
        let bg = self.theme.bg_surface;
        let content_spacing = self.theme.content_spacing;

        let placeholder_idx = self.frame.instance_len();
        self.frame.push(
            esox_gfx::ShapeBuilder::rect(0.0, 0.0, 0.0, 0.0)
                .color(Color::new(0.0, 0.0, 0.0, 0.0))
                .build(),
        );

        let start_y = self.cursor.y;
        let saved_spacing = self.spacing;
        self.spacing = content_spacing;
        self.padding(pad, f);
        self.spacing = saved_spacing;
        let end_y = self.cursor.y;

        let surface_rect = Rect::new(self.region.x, start_y, self.region.w, end_y - start_y);

        self.frame.replace_instance(
            placeholder_idx,
            esox_gfx::ShapeBuilder::rect(
                surface_rect.x,
                surface_rect.y,
                surface_rect.w,
                surface_rect.h,
            )
            .color(bg)
            .border_radius(esox_gfx::BorderRadius::uniform(radius))
            .build(),
        );

        self.cursor.y += self.theme.card_gap;
    }

    /// Draw a titled section — header label + content with automatic spacing.
    ///
    /// Sections are the primary way to group related content on a page.
    /// The header is drawn as a `header_label` and children get `content_spacing`.
    pub fn section(&mut self, title: &str, f: impl FnOnce(&mut Self)) {
        self.header_label(title);
        let saved = self.spacing;
        self.spacing = self.theme.content_spacing;
        f(self);
        self.spacing = saved;
        // Advance to section_gap, compensating for the content_spacing already added
        // after the last child widget.
        let extra = self.theme.section_gap - self.theme.content_spacing;
        if extra > 0.0 {
            self.cursor.y += extra;
        }
    }
}
