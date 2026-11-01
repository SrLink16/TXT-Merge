//! TXT Merge look: brand colors, light and dark palettes, fonts and the few
//! branded widgets (primary button, segmented control, drop zone, wordmark).

use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, Pos2, Rect, Response, Rounding,
    Sense, Shadow, Stroke, TextStyle, Ui, Vec2, pos2, vec2,
};

pub const NAVY: Color32 = Color32::from_rgb(0x14, 0x3D, 0x66);
pub const TEAL: Color32 = Color32::from_rgb(0x17, 0xA5, 0x89);
pub const MINT: Color32 = Color32::from_rgb(0x1A, 0xBC, 0x9C);
pub const SUCCESS: Color32 = Color32::from_rgb(0x10, 0xB9, 0x81);
pub const WARNING: Color32 = Color32::from_rgb(0xF5, 0x9E, 0x0B);
pub const ERROR: Color32 = Color32::from_rgb(0xEF, 0x44, 0x44);

/// Editor text size and line height (1.6), from the brand manual.
pub const EDITOR_FONT_SIZE: f32 = 14.0;
pub const EDITOR_LINE_HEIGHT: f32 = EDITOR_FONT_SIZE * 1.6;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub fn key(self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key.trim() {
            "light" => Some(Theme::Light),
            "dark" => Some(Theme::Dark),
            _ => None,
        }
    }
}

pub struct Palette {
    pub bg_app: Color32,
    pub bg_editor: Color32,
    pub bg_sidebar: Color32,
    pub text: Color32,
    pub text_weak: Color32,
    pub border: Color32,
    pub hover: Color32,
    pub active: Color32,
    /// Color of "TXT" in the wordmark.
    pub word_txt: Color32,
    pub subtitle: Color32,
    /// Text of selected items and links on this background.
    pub accent_text: Color32,
    pub shadow: Color32,
}

impl Palette {
    pub fn of(dark: bool) -> Self {
        if dark {
            Self {
                bg_app: Color32::from_rgb(0x0F, 0x17, 0x2A),
                bg_editor: Color32::from_rgb(0x1E, 0x29, 0x3B),
                bg_sidebar: Color32::from_rgb(0x13, 0x1D, 0x31),
                text: Color32::from_rgb(0xF8, 0xFA, 0xFC),
                text_weak: Color32::from_rgb(0x94, 0xA3, 0xB8),
                border: Color32::from_rgb(0x33, 0x41, 0x55),
                hover: Color32::from_rgb(0x27, 0x34, 0x49),
                active: Color32::from_rgb(0x33, 0x41, 0x55),
                word_txt: Color32::from_rgb(0xF8, 0xFA, 0xFC),
                subtitle: Color32::from_rgb(0x94, 0xA3, 0xB8),
                accent_text: MINT,
                shadow: Color32::from_black_alpha(90),
            }
        } else {
            Self {
                bg_app: Color32::from_rgb(0xF4, 0xF7, 0xFA),
                bg_editor: Color32::WHITE,
                bg_sidebar: Color32::from_rgb(0xEB, 0xF0, 0xF5),
                text: Color32::from_rgb(0x1E, 0x29, 0x2B),
                text_weak: Color32::from_rgb(0x64, 0x74, 0x8B),
                border: Color32::from_rgb(0xCB, 0xD5, 0xE1),
                hover: Color32::from_rgb(0xE2, 0xE8, 0xF0),
                active: Color32::from_rgb(0xCB, 0xD5, 0xE1),
                word_txt: NAVY,
                subtitle: Color32::from_rgb(0x33, 0x41, 0x55),
                accent_text: Color32::from_rgb(0x0E, 0x7C, 0x67),
                shadow: Color32::from_rgba_unmultiplied(20, 61, 102, 22),
            }
        }
    }

    pub fn current(ui: &Ui) -> Self {
        Self::of(ui.visuals().dark_mode)
    }
}

pub fn semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("semibold".into()))
}

pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}

/// Uses Segoe UI and Consolas from Windows when present; egui's own fonts
/// stay as fallback for missing glyphs or a system without them.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let mut load = |key: &str, file: &str| -> Option<String> {
        let bytes = std::fs::read(format!(r"C:\Windows\Fonts\{file}")).ok()?;
        fonts.font_data.insert(key.to_owned(), FontData::from_owned(bytes));
        Some(key.to_owned())
    };
    let regular = load("segoe", "segoeui.ttf");
    let semi = load("segoe-semibold", "seguisb.ttf");
    let heavy = load("segoe-bold", "segoeuib.ttf");
    let symbols = load("segoe-symbol", "seguisym.ttf");
    let mono = load("consolas", "consola.ttf");

    let fallback = fonts.families[&FontFamily::Proportional].clone();
    let family = |first: &[&Option<String>]| -> Vec<String> {
        first.iter().filter_map(|f| f.as_ref().cloned()).chain(fallback.iter().cloned()).collect()
    };
    let proportional = family(&[&regular, &symbols]);
    let semibold = family(&[&semi, &regular, &symbols]);
    let boldest = family(&[&heavy, &semi, &regular, &symbols]);
    fonts.families.insert(FontFamily::Proportional, proportional);
    fonts.families.insert(FontFamily::Name("semibold".into()), semibold);
    fonts.families.insert(FontFamily::Name("bold".into()), boldest);
    if let Some(mono) = mono {
        fonts.families.entry(FontFamily::Monospace).or_default().insert(0, mono);
    }
    ctx.set_fonts(fonts);
}

/// Applies the light or dark brand look to the whole app.
pub fn apply(ctx: &egui::Context, theme: Theme) {
    let dark = theme == Theme::Dark;
    let p = Palette::of(dark);
    let mut v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = p.bg_app;
    v.window_fill = p.bg_editor;
    v.extreme_bg_color = p.bg_editor;
    v.faint_bg_color = p.bg_sidebar;
    v.code_bg_color = p.bg_sidebar;
    v.hyperlink_color = p.accent_text;
    v.warn_fg_color = WARNING;
    v.error_fg_color = ERROR;
    v.selection.bg_fill = TEAL.gamma_multiply(if dark { 0.35 } else { 0.22 });
    v.selection.stroke = Stroke::new(1.0_f32, p.accent_text);
    v.text_cursor.stroke = Stroke::new(2.0_f32, TEAL);
    v.window_rounding = Rounding::same(12.0);
    v.menu_rounding = Rounding::same(8.0);
    v.window_stroke = Stroke::new(1.0_f32, p.border);
    v.window_shadow = Shadow { offset: vec2(0.0, 10.0), blur: 32.0, spread: 0.0, color: p.shadow };
    v.popup_shadow = Shadow { offset: vec2(0.0, 4.0), blur: 12.0, spread: 0.0, color: p.shadow };
    v.striped = false;

    // Small enough that checkboxes read as squares, so they are not mistaken
    // for the round single-choice radio buttons.
    let rounding = Rounding::same(3.0);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.bg_editor;
    w.noninteractive.weak_bg_fill = p.bg_editor;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text);
    w.noninteractive.rounding = rounding;

    w.inactive.bg_fill = p.bg_editor;
    w.inactive.weak_bg_fill = p.bg_editor;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, p.border);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, p.text);
    w.inactive.rounding = rounding;

    w.hovered.bg_fill = p.hover;
    w.hovered.weak_bg_fill = p.hover;
    w.hovered.bg_stroke = Stroke::new(1.0_f32, TEAL);
    w.hovered.fg_stroke = Stroke::new(1.5_f32, p.text);
    w.hovered.rounding = rounding;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = p.active;
    w.active.weak_bg_fill = p.active;
    w.active.bg_stroke = Stroke::new(1.0_f32, MINT);
    w.active.fg_stroke = Stroke::new(1.5_f32, p.text);
    w.active.rounding = rounding;
    w.active.expansion = 0.0;

    w.open.bg_fill = p.hover;
    w.open.weak_bg_fill = p.hover;
    w.open.bg_stroke = Stroke::new(1.0_f32, TEAL);
    w.open.fg_stroke = Stroke::new(1.0_f32, p.text);
    w.open.rounding = rounding;

    ctx.set_theme(if dark { egui::Theme::Dark } else { egui::Theme::Light });
    ctx.style_mut(|s| {
        s.visuals = v;
        s.spacing.item_spacing = vec2(8.0, 6.0);
        s.spacing.button_padding = vec2(10.0, 5.0);
        s.spacing.interact_size.y = 26.0;
        s.text_styles = [
            (TextStyle::Small, FontId::proportional(11.5)),
            (TextStyle::Body, FontId::proportional(13.0)),
            (TextStyle::Button, FontId::proportional(13.0)),
            (TextStyle::Heading, semibold(16.0)),
            (TextStyle::Monospace, FontId::monospace(EDITOR_FONT_SIZE)),
        ]
        .into();
    });
}

/// Card frame: the surface color with a soft border, used for the file
/// list and the viewer.
pub fn card(ui: &Ui) -> egui::Frame {
    let p = Palette::current(ui);
    egui::Frame::none()
        .fill(p.bg_editor)
        .stroke(Stroke::new(1.0_f32, p.border))
        .rounding(10.0)
        .inner_margin(10.0)
        .shadow(Shadow { offset: vec2(0.0, 2.0), blur: 8.0, spread: 0.0, color: p.shadow })
}

/// Points around a rounded rectangle, clockwise from the top-right corner.
fn rounded_rect_points(rect: Rect, radius: f32, segments: usize) -> Vec<Pos2> {
    let r = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let corners = [
        (pos2(rect.right() - r, rect.top() + r), -90.0_f32),
        (pos2(rect.right() - r, rect.bottom() - r), 0.0),
        (pos2(rect.left() + r, rect.bottom() - r), 90.0),
        (pos2(rect.left() + r, rect.top() + r), 180.0),
    ];
    let mut points = Vec::with_capacity(4 * (segments + 1));
    for (center, start) in corners {
        for i in 0..=segments {
            let angle = (start + 90.0 * i as f32 / segments as f32).to_radians();
            points.push(center + r * vec2(angle.cos(), angle.sin()));
        }
    }
    points
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()), mix(a.a(), b.a()))
}

/// Fills a rounded rectangle with the brand's 135° gradient.
fn paint_gradient(painter: &egui::Painter, rect: Rect, radius: f32, from: Color32, to: Color32) {
    let color_at = |p: Pos2| {
        lerp_color(from, to, (p.x - rect.left() + p.y - rect.top()) / (rect.width() + rect.height()))
    };
    let points = rounded_rect_points(rect, radius, 8);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.center(), color_at(rect.center()));
    for p in &points {
        mesh.colored_vertex(*p, color_at(*p));
    }
    let n = points.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    painter.add(mesh);
    // The mesh has hard edges; a thin outline smooths the rounded corners.
    let edge = lerp_color(from, to, 0.5).gamma_multiply(0.6);
    painter.add(egui::epaint::PathShape::closed_line(points, Stroke::new(1.0_f32, edge)));
}

/// Full-color call to action with the navy-to-teal gradient.
pub fn primary_button(ui: &mut Ui, text: impl Into<String>, enabled: bool, min_size: Vec2) -> Response {
    let text_color = if enabled { Color32::WHITE } else { Color32::from_white_alpha(200) };
    let galley = ui.painter().layout_no_wrap(text.into(), semibold(14.0), text_color);
    let size = vec2(
        (galley.size().x + 36.0).max(min_size.x),
        (galley.size().y + 16.0).max(min_size.y),
    );
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    if ui.is_rect_visible(rect) {
        let (from, to) = if !enabled {
            (NAVY.gamma_multiply(0.35), TEAL.gamma_multiply(0.35))
        } else if response.is_pointer_button_down_on() {
            (NAVY, TEAL)
        } else if response.hovered() {
            (lerp_color(NAVY, TEAL, 0.25), MINT)
        } else {
            (NAVY, TEAL)
        };
        paint_gradient(ui.painter(), rect, 8.0, from, to);
        let pos = rect.center() - galley.size() / 2.0;
        ui.painter().galley(pos, galley, text_color);
    }
    if enabled { response.on_hover_cursor(egui::CursorIcon::PointingHand) } else { response }
}

/// A row of pill-shaped choices where exactly one is selected. Returns true
/// when the user picked a different one.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    id: &str,
    value: &mut T,
    options: &[(T, &str)],
    item_width: f32,
) -> bool {
    let p = Palette::current(ui);
    let (pad, height) = (3.0, 28.0);
    let size = vec2(item_width * options.len() as f32 + pad * 2.0, height + pad * 2.0);
    let (outer, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect(outer, 10.0, p.bg_sidebar, Stroke::new(1.0_f32, p.border));
    let mut changed = false;
    for (i, (option, label)) in options.iter().enumerate() {
        let rect = Rect::from_min_size(
            outer.min + vec2(pad + item_width * i as f32, pad),
            vec2(item_width, height),
        );
        let response = ui.interact(rect, ui.id().with((id, i)), Sense::click());
        let selected = *value == *option;
        if selected {
            ui.painter().rect(rect, 8.0, p.bg_editor, Stroke::new(1.0_f32, p.border));
        } else if response.hovered() {
            ui.painter().rect_filled(rect, 8.0, p.hover);
        }
        let color = if selected { p.accent_text } else { p.text_weak };
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, *label, semibold(13.0), color);
        if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !selected {
            *value = *option;
            changed = true;
        }
    }
    changed
}

/// Square icon button without a frame until hovered, like a toolbar icon.
pub fn icon_button(ui: &mut Ui, icon: &str, hover: &str) -> Response {
    let button = egui::Button::new(egui::RichText::new(icon).size(17.0))
        .frame(false)
        .min_size(vec2(34.0, 34.0));
    let response = ui.add(button);
    if response.hovered() {
        let p = Palette::current(ui);
        ui.painter().rect_filled(response.rect, 8.0, p.hover);
        ui.painter().text(
            response.rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            FontId::proportional(17.0),
            p.text,
        );
    }
    response.on_hover_text(hover).on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Big dashed area that invites dropping a folder or clicking to browse.
/// Turns teal while hovered or while files are dragged over the window.
pub fn drop_zone(ui: &mut Ui, icon: &str, title: &str, subtitle: &str, highlight: bool) -> Response {
    let p = Palette::current(ui);
    let size = vec2(ui.available_width().min(520.0), 190.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hot = highlight || response.hovered();
    let line = if hot { TEAL } else { p.text_weak };
    let fill = if hot { TEAL.gamma_multiply(0.07) } else { p.bg_editor };
    let painter = ui.painter();
    painter.rect_filled(rect, 12.0, fill);
    let mut outline = rounded_rect_points(rect.shrink(1.0), 12.0, 6);
    outline.push(outline[0]);
    painter.extend(egui::Shape::dashed_line(&outline, Stroke::new(2.0_f32, line), 7.0, 5.0));

    let icon_center = rect.center() - vec2(0.0, 38.0);
    painter.circle_filled(icon_center, 24.0, p.bg_sidebar);
    painter.text(icon_center, egui::Align2::CENTER_CENTER, icon, FontId::proportional(20.0), line);
    painter.text(rect.center() + vec2(0.0, 12.0), egui::Align2::CENTER_CENTER, title, semibold(16.0), p.text);
    painter.text(
        rect.center() + vec2(0.0, 38.0),
        egui::Align2::CENTER_CENTER,
        subtitle,
        FontId::proportional(12.5),
        p.text_weak,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Logo tile plus "TXT Merge" and the MERGER & EDITOR tagline.
pub fn wordmark(ui: &mut Ui, logo: &egui::TextureHandle) {
    let p = Palette::current(ui);
    ui.horizontal(|ui| {
        let image = egui::Image::new(egui::load::SizedTexture::from_handle(logo));
        ui.add(image.fit_to_exact_size(vec2(36.0, 36.0)));
        ui.add_space(2.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let mut name = egui::text::LayoutJob::default();
            let format = |font_id: FontId, color: Color32| egui::TextFormat { font_id, color, ..Default::default() };
            name.append("TXT ", 0.0, format(bold(19.0), p.word_txt));
            name.append("Merge", 0.0, format(bold(19.0), TEAL));
            ui.label(name);
            let mut tagline = egui::text::LayoutJob::default();
            tagline.append(
                "MERGER & EDITOR",
                0.0,
                egui::TextFormat {
                    font_id: semibold(8.5),
                    color: p.subtitle,
                    // 0.25em, as in the brand manual.
                    extra_letter_spacing: 8.5 * 0.25,
                    ..Default::default()
                },
            );
            ui.label(tagline);
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_keys_round_trip() {
        for t in [Theme::Light, Theme::Dark] {
            assert_eq!(Theme::from_key(t.key()), Some(t));
        }
        assert_eq!(Theme::from_key("blue"), None);
    }

    #[test]
    fn gradient_goes_from_navy_to_teal() {
        assert_eq!(lerp_color(NAVY, TEAL, 0.0), NAVY);
        assert_eq!(lerp_color(NAVY, TEAL, 1.0), TEAL);
        assert_eq!(lerp_color(NAVY, TEAL, 7.0), TEAL);
    }

    #[test]
    fn rounded_rect_outline_stays_inside_the_rect() {
        let rect = Rect::from_min_size(pos2(10.0, 20.0), vec2(100.0, 40.0));
        let points = rounded_rect_points(rect, 12.0, 6);
        assert_eq!(points.len(), 28);
        assert!(points.iter().all(|p| rect.expand(0.01).contains(*p)));
    }
}
