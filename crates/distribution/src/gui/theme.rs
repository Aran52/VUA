use super::copy::Lang;
use std::{ffi::c_void, ptr};
use windows_sys::Win32::{
    Foundation::RECT,
    Graphics::Gdi::*,
    System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD},
    UI::WindowsAndMessaging::*,
};

pub const LOGICAL_WIDTH: i32 = 468;
pub const LOGICAL_HEIGHT: i32 = 244;

const DT_EDITCONTROL_FLAG: u32 = 0x0000_2000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const fn colorref(self) -> u32 {
        (self.0 as u32) | ((self.1 as u32) << 8) | ((self.2 as u32) << 16)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub is_dark: bool,
    pub canvas: Rgb,
    pub glow: Rgb,
    pub glow_alpha: f32,
    pub panel: Rgb,
    pub elevated: Rgb,
    pub border: Rgb,
    pub text_strong: Rgb,
    pub text_secondary: Rgb,
    pub text_muted: Rgb,
    pub accent: Rgb,
    pub accent_hover: Rgb,
    pub accent_pressed: Rgb,
    pub accent_tint: Rgb,
    pub accent_pill_border: Rgb,
    pub accent_text_on: Rgb,
    pub danger: Rgb,
    pub danger_hover: Rgb,
    pub danger_pressed: Rgb,
    pub danger_tint: Rgb,
    pub danger_border: Rgb,
    pub danger_text_on: Rgb,
    pub icon_badge_bg: Rgb,
    pub icon_badge_border: Rgb,
    pub secondary_bg: Rgb,
    pub secondary_hover: Rgb,
    pub secondary_pressed: Rgb,
}

impl Palette {
    pub const fn dark() -> Self {
        Self {
            is_dark: true,
            canvas: Rgb(0x10, 0x12, 0x16),
            glow: Rgb(0xa7, 0x8b, 0xfa),
            glow_alpha: 0.085,
            panel: Rgb(0x19, 0x1c, 0x22),
            elevated: Rgb(0x25, 0x2a, 0x34),
            border: Rgb(0x32, 0x38, 0x44),
            text_strong: Rgb(0xf5, 0xf5, 0xf7),
            text_secondary: Rgb(0xb7, 0xbb, 0xc7),
            text_muted: Rgb(0x7c, 0x82, 0x92),
            accent: Rgb(0xa7, 0x8b, 0xfa),
            accent_hover: Rgb(0xb7, 0x9f, 0xfc),
            accent_pressed: Rgb(0x93, 0x77, 0xe8),
            accent_tint: Rgb(0x26, 0x22, 0x38),
            accent_pill_border: Rgb(0x43, 0x39, 0x68),
            accent_text_on: Rgb(0x16, 0x13, 0x2b),
            danger: Rgb(0xfb, 0x71, 0x85),
            danger_hover: Rgb(0xfc, 0x89, 0x9a),
            danger_pressed: Rgb(0xe8, 0x58, 0x6e),
            danger_tint: Rgb(0x28, 0x17, 0x1e),
            danger_border: Rgb(0x5e, 0x29, 0x35),
            danger_text_on: Rgb(0x19, 0x0b, 0x0f),
            icon_badge_bg: Rgb(0x19, 0x1c, 0x22),
            icon_badge_border: Rgb(0x36, 0x3c, 0x49),
            secondary_bg: Rgb(0x1b, 0x1f, 0x26),
            secondary_hover: Rgb(0x25, 0x2a, 0x33),
            secondary_pressed: Rgb(0x2e, 0x34, 0x40),
        }
    }

    pub const fn light() -> Self {
        Self {
            is_dark: false,
            canvas: Rgb(0xf6, 0xf6, 0xf9),
            glow: Rgb(0x65, 0x57, 0xd2),
            glow_alpha: 0.055,
            panel: Rgb(0xff, 0xff, 0xff),
            elevated: Rgb(0xe7, 0xe6, 0xf0),
            border: Rgb(0xd8, 0xd7, 0xe3),
            text_strong: Rgb(0x19, 0x18, 0x24),
            text_secondary: Rgb(0x56, 0x54, 0x70),
            text_muted: Rgb(0x8a, 0x88, 0xa2),
            accent: Rgb(0x65, 0x57, 0xd2),
            accent_hover: Rgb(0x56, 0x48, 0xc2),
            accent_pressed: Rgb(0x46, 0x3a, 0xa8),
            accent_tint: Rgb(0xed, 0xeb, 0xfc),
            accent_pill_border: Rgb(0xd1, 0xcc, 0xf5),
            accent_text_on: Rgb(0xff, 0xff, 0xff),
            danger: Rgb(0xd1, 0x34, 0x38),
            danger_hover: Rgb(0xbe, 0x2b, 0x2f),
            danger_pressed: Rgb(0xa6, 0x23, 0x27),
            danger_tint: Rgb(0xfd, 0xf3, 0xf4),
            danger_border: Rgb(0xf0, 0xc2, 0xc5),
            danger_text_on: Rgb(0xff, 0xff, 0xff),
            icon_badge_bg: Rgb(0x1e, 0x1c, 0x2e),
            icon_badge_border: Rgb(0x38, 0x34, 0x52),
            secondary_bg: Rgb(0xff, 0xff, 0xff),
            secondary_hover: Rgb(0xef, 0xee, 0xf6),
            secondary_pressed: Rgb(0xe4, 0xe2, 0xee),
        }
    }

    pub fn system() -> Self {
        if detect_system_dark_mode() {
            Self::dark()
        } else {
            Self::light()
        }
    }
}

pub fn detect_system_dark_mode() -> bool {
    let subkey: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"
        .encode_utf16()
        .chain([0])
        .collect();
    let value_name: Vec<u16> = "AppsUseLightTheme".encode_utf16().chain([0]).collect();
    let mut light_value: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            subkey.as_ptr(),
            value_name.as_ptr(),
            RRF_RT_REG_DWORD,
            ptr::null_mut(),
            (&mut light_value as *mut u32).cast::<c_void>(),
            &mut size,
        )
    };
    if status == 0 {
        light_value == 0
    } else {
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HitRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl HitRect {
    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }

    pub fn to_win32(self) -> RECT {
        RECT {
            left: self.left,
            top: self.top,
            right: self.right,
            bottom: self.bottom,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonRole {
    Secondary,
    Primary,
    Danger,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonAction {
    Cancel,
    Close,
    Retry,
    ConfirmUninstall,
}

#[derive(Clone, Debug)]
pub struct ButtonSpec<'a> {
    pub label: &'a str,
    pub role: ButtonRole,
    pub action: ButtonAction,
    pub enabled: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardTone {
    Neutral,
    Danger,
}

#[derive(Clone, Debug)]
pub enum BodySpec<'a> {
    Progress {
        phase_label: &'a str,
        metric_label: Option<&'a str>,
        fraction: Option<f32>,
        anim_phase: f32,
    },
    Message {
        tone: CardTone,
        code_badge: Option<&'a str>,
        body: &'a str,
    },
}

#[derive(Clone, Debug)]
pub struct CardModel<'a> {
    pub lang: Lang,
    pub palette: Palette,
    pub title: &'a str,
    pub version_badge: Option<&'a str>,
    pub subtitle: &'a str,
    pub body: BodySpec<'a>,
    pub buttons: &'a [ButtonSpec<'a>],
}

#[derive(Clone, Debug, Default)]
pub struct RenderedLayout {
    pub buttons: Vec<(ButtonAction, bool, HitRect)>,
}

pub struct DibSurface {
    pub dc: HDC,
    bitmap: HBITMAP,
    old_bitmap: HGDIOBJ,
    pixels: *mut u32,
    pub width: i32,
    pub height: i32,
    cached_fonts: Option<((Lang, u32), FontSet)>,
}

impl DibSurface {
    pub fn new(width: i32, height: i32) -> Option<Self> {
        let width = width.max(1);
        let height = height.max(1);
        unsafe {
            let dc = CreateCompatibleDC(ptr::null_mut());
            if dc.is_null() {
                return None;
            }
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB,
                    ..std::mem::zeroed()
                },
                ..std::mem::zeroed()
            };
            let mut bits: *mut c_void = ptr::null_mut();
            let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, ptr::null_mut(), 0);
            if bitmap.is_null() || bits.is_null() {
                DeleteDC(dc);
                return None;
            }
            let old_bitmap = SelectObject(dc, bitmap as HGDIOBJ);
            SetBkMode(dc, TRANSPARENT as i32);
            Some(Self {
                dc,
                bitmap,
                old_bitmap,
                pixels: bits.cast::<u32>(),
                width,
                height,
                cached_fonts: None,
            })
        }
    }

    fn fonts(&mut self, lang: Lang, dpi: u32, scale: f32) -> FontSetHandles {
        let key = (lang, dpi);
        let rebuild = self
            .cached_fonts
            .as_ref()
            .is_none_or(|(cached_key, _)| *cached_key != key);
        if rebuild {
            self.cached_fonts = Some((key, FontSet::create(lang.font_family(), scale)));
        }
        self.cached_fonts
            .as_ref()
            .expect("cached fonts")
            .1
            .handles()
    }

    pub fn ensure_size(&mut self, width: i32, height: i32) {
        if self.width == width && self.height == height {
            return;
        }
        if let Some(next) = Self::new(width, height) {
            *self = next;
        }
    }

    fn pixels_mut(&mut self) -> &mut [u32] {
        unsafe { std::slice::from_raw_parts_mut(self.pixels, (self.width * self.height) as usize) }
    }

    #[cfg(test)]
    pub fn pixels(&self) -> &[u32] {
        unsafe { std::slice::from_raw_parts(self.pixels, (self.width * self.height) as usize) }
    }

    fn fill_background(&mut self, canvas: Rgb, glow: Rgb, glow_alpha: f32, scale: f32) {
        let w = self.width;
        let h = self.height;
        let gx = 54.0 * scale;
        let gy = 36.0 * scale;
        let radius = 250.0 * scale;
        let inv_r = 1.0 / radius.max(1.0);
        let buf = self.pixels_mut();
        for y in 0..h {
            let dy = (y as f32 + 0.5) - gy;
            let row = (y * w) as usize;
            for x in 0..w {
                let dx = (x as f32 + 0.5) - gx;
                let t = (1.0 - (dx.hypot(dy) * inv_r)).clamp(0.0, 1.0);
                let a = t * t * glow_alpha;
                let r = lerp_u8(canvas.0, glow.0, a);
                let g = lerp_u8(canvas.1, glow.1, a);
                let b = lerp_u8(canvas.2, glow.2, a);
                buf[row + x as usize] = pack_bgra(r, g, b);
            }
        }
    }

    fn draw_round_rect(
        &mut self,
        rect: HitRect,
        radius: f32,
        fill: Rgb,
        fill_alpha: f32,
        border: Option<(Rgb, f32, f32)>,
    ) {
        let x0 = (rect.left - 2).max(0);
        let y0 = (rect.top - 2).max(0);
        let x1 = (rect.right + 2).min(self.width);
        let y1 = (rect.bottom + 2).min(self.height);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let cx = (rect.left + rect.right) as f32 * 0.5;
        let cy = (rect.top + rect.bottom) as f32 * 0.5;
        let hx = (rect.right - rect.left) as f32 * 0.5;
        let hy = (rect.bottom - rect.top) as f32 * 0.5;
        let r = radius.min(hx).min(hy).max(0.0);
        let w = self.width;
        let buf = self.pixels_mut();

        for y in y0..y1 {
            let py = y as f32 + 0.5;
            let row = (y * w) as usize;
            for x in x0..x1 {
                let px = x as f32 + 0.5;
                let sd = sd_round_box(px - cx, py - cy, hx, hy, r);
                let cov_outer = (0.5 - sd).clamp(0.0, 1.0);
                if cov_outer <= 0.0 {
                    continue;
                }
                let idx = row + x as usize;
                let dst = buf[idx];
                if let Some((b_col, b_width, b_alpha)) = border {
                    let cov_inner = (0.5 - (sd + b_width)).clamp(0.0, 1.0);
                    let border_weight = (cov_outer - cov_inner).max(0.0) * b_alpha;
                    let fill_weight = cov_inner * fill_alpha;
                    let total = border_weight + fill_weight;
                    if total > 0.0 {
                        let src_r =
                            (fill.0 as f32 * fill_weight + b_col.0 as f32 * border_weight) / total;
                        let src_g =
                            (fill.1 as f32 * fill_weight + b_col.1 as f32 * border_weight) / total;
                        let src_b =
                            (fill.2 as f32 * fill_weight + b_col.2 as f32 * border_weight) / total;
                        buf[idx] = blend_pixel(
                            dst,
                            Rgb(
                                src_r.round() as u8,
                                src_g.round() as u8,
                                src_b.round() as u8,
                            ),
                            total.clamp(0.0, 1.0),
                        );
                    }
                } else {
                    buf[idx] = blend_pixel(dst, fill, cov_outer * fill_alpha);
                }
            }
        }
    }

    fn draw_clipped_progress_fill(
        &mut self,
        track: HitRect,
        bar_left: f32,
        bar_right: f32,
        radius: f32,
        color: Rgb,
    ) {
        if bar_right <= bar_left {
            return;
        }
        let x0 = track.left.max(0);
        let y0 = track.top.max(0);
        let x1 = track.right.min(self.width);
        let y1 = track.bottom.min(self.height);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let t_cx = (track.left + track.right) as f32 * 0.5;
        let t_cy = (track.top + track.bottom) as f32 * 0.5;
        let t_hx = (track.right - track.left) as f32 * 0.5;
        let t_hy = (track.bottom - track.top) as f32 * 0.5;
        let r_track = radius.min(t_hx).min(t_hy).max(0.0);

        let b_cx = (bar_left + bar_right) * 0.5;
        let b_hx = (bar_right - bar_left) * 0.5;
        let r_bar = radius.min(b_hx).min(t_hy).max(0.0);

        let w = self.width;
        let buf = self.pixels_mut();
        for y in y0..y1 {
            let py = y as f32 + 0.5;
            let row = (y * w) as usize;
            for x in x0..x1 {
                let px = x as f32 + 0.5;
                let sd_t = sd_round_box(px - t_cx, py - t_cy, t_hx, t_hy, r_track);
                let cov_t = (0.5 - sd_t).clamp(0.0, 1.0);
                if cov_t <= 0.0 {
                    continue;
                }
                let sd_b = sd_round_box(px - b_cx, py - t_cy, b_hx, t_hy, r_bar);
                let cov_b = (0.5 - sd_b).clamp(0.0, 1.0);
                let alpha = cov_t * cov_b;
                if alpha > 0.0 {
                    let idx = row + x as usize;
                    buf[idx] = blend_pixel(buf[idx], color, alpha);
                }
            }
        }
    }
}

impl Drop for DibSurface {
    fn drop(&mut self) {
        unsafe {
            if !self.dc.is_null() {
                if !self.old_bitmap.is_null() {
                    SelectObject(self.dc, self.old_bitmap);
                }
                if !self.bitmap.is_null() {
                    DeleteObject(self.bitmap as HGDIOBJ);
                }
                DeleteDC(self.dc);
            }
        }
    }
}

fn sd_round_box(dx: f32, dy: f32, hx: f32, hy: f32, r: f32) -> f32 {
    let qx = dx.abs() - (hx - r);
    let qy = dy.abs() - (hy - r);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.min(0.0).max(qy.min(0.0)) - r
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

const fn pack_bgra(r: u8, g: u8, b: u8) -> u32 {
    (b as u32) | ((g as u32) << 8) | ((r as u32) << 16) | 0xff00_0000
}

fn blend_pixel(dst: u32, src: Rgb, alpha: f32) -> u32 {
    let db = (dst & 0xff) as u8;
    let dg = ((dst >> 8) & 0xff) as u8;
    let dr = ((dst >> 16) & 0xff) as u8;
    pack_bgra(
        lerp_u8(dr, src.0, alpha),
        lerp_u8(dg, src.1, alpha),
        lerp_u8(db, src.2, alpha),
    )
}

#[derive(Clone, Copy)]
struct FontSetHandles {
    title: HFONT,
    subtitle: HFONT,
    body_medium: HFONT,
    body_regular: HFONT,
    badge: HFONT,
    button_semibold: HFONT,
    button_regular: HFONT,
}

struct FontSet {
    handles: FontSetHandles,
}

impl FontSet {
    fn create(family: &str, scale: f32) -> Self {
        Self {
            handles: FontSetHandles {
                title: make_font(family, (18.0 * scale).round() as i32, FW_SEMIBOLD as i32),
                subtitle: make_font(family, (13.0 * scale).round() as i32, FW_NORMAL as i32),
                body_medium: make_font(family, (12.5 * scale).round() as i32, FW_MEDIUM as i32),
                body_regular: make_font(family, (12.5 * scale).round() as i32, FW_NORMAL as i32),
                badge: make_font(
                    "Segoe UI",
                    (11.5 * scale).round() as i32,
                    FW_SEMIBOLD as i32,
                ),
                button_semibold: make_font(
                    family,
                    (13.0 * scale).round() as i32,
                    FW_SEMIBOLD as i32,
                ),
                button_regular: make_font(family, (13.0 * scale).round() as i32, FW_MEDIUM as i32),
            },
        }
    }

    fn handles(&self) -> FontSetHandles {
        self.handles
    }
}

impl Drop for FontSet {
    fn drop(&mut self) {
        unsafe {
            for f in [
                self.handles.title,
                self.handles.subtitle,
                self.handles.body_medium,
                self.handles.body_regular,
                self.handles.badge,
                self.handles.button_semibold,
                self.handles.button_regular,
            ] {
                if !f.is_null() {
                    DeleteObject(f as HGDIOBJ);
                }
            }
        }
    }
}

fn make_font(family: &str, pixel_height: i32, weight: i32) -> HFONT {
    let face: Vec<u16> = family.encode_utf16().chain([0]).collect();
    unsafe {
        CreateFontW(
            -pixel_height.max(8),
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_TT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            CLEARTYPE_QUALITY as u32,
            (DEFAULT_PITCH | FF_DONTCARE) as u32,
            face.as_ptr(),
        )
    }
}

fn measure_single_line(dc: HDC, font: HFONT, text: &str) -> (i32, i32) {
    if text.is_empty() {
        return (0, 0);
    }
    let mut wide_buf: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        let prev = SelectObject(dc, font as HGDIOBJ);
        let mut rc = RECT {
            left: 0,
            top: 0,
            right: 2000,
            bottom: 200,
        };
        DrawTextW(
            dc,
            wide_buf.as_mut_ptr(),
            wide_buf.len() as i32,
            &mut rc,
            DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX,
        );
        SelectObject(dc, prev);
        (rc.right - rc.left, rc.bottom - rc.top)
    }
}

fn draw_text(dc: HDC, font: HFONT, color: Rgb, rect: HitRect, flags: u32, text: &str) {
    if text.is_empty() || rect.right <= rect.left || rect.bottom <= rect.top {
        return;
    }
    let mut wide_buf: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        let prev = SelectObject(dc, font as HGDIOBJ);
        SetTextColor(dc, color.colorref());
        let mut rc = rect.to_win32();
        DrawTextW(
            dc,
            wide_buf.as_mut_ptr(),
            wide_buf.len() as i32,
            &mut rc,
            flags | DT_NOPREFIX,
        );
        SelectObject(dc, prev);
    }
}

pub fn render_card(
    surface: &mut DibSurface,
    dpi: u32,
    icon: HICON,
    model: &CardModel<'_>,
) -> RenderedLayout {
    let scale = (dpi.max(96) as f32) / 96.0;
    let px = |dp: f32| -> i32 { (dp * scale).round() as i32 };
    let border_px = scale.max(1.0);
    let pal = &model.palette;
    let fonts = surface.fonts(model.lang, dpi.max(96), scale);

    // 1. Background + subtle purple glow
    surface.fill_background(pal.canvas, pal.glow, pal.glow_alpha, scale);

    // 2. App Icon Squircle Badge
    let badge_rect = HitRect {
        left: px(28.0),
        top: px(20.0),
        right: px(80.0),
        bottom: px(72.0),
    };
    surface.draw_round_rect(
        badge_rect,
        12.0 * scale,
        pal.icon_badge_bg,
        1.0,
        Some((pal.icon_badge_border, border_px, 1.0)),
    );

    // Measure title & optional version pill before GdiFlush so pill background is drawn cleanly
    let (title_w, _) = measure_single_line(surface.dc, fonts.title, model.title);
    let max_header_right = px(440.0);
    let title_left = px(94.0);

    let mut pill_rect_and_text: Option<(HitRect, &str)> = None;
    let mut title_right = (title_left + title_w + px(4.0)).min(max_header_right);
    if let Some(ver) = model.version_badge.filter(|v| !v.is_empty()) {
        let (ver_w, _) = measure_single_line(surface.dc, fonts.badge, ver);
        let pill_w = ver_w + px(16.0);
        let pill_left = title_left + title_w + px(10.0);
        if pill_left + pill_w <= max_header_right {
            pill_rect_and_text = Some((
                HitRect {
                    left: pill_left,
                    top: px(24.0),
                    right: pill_left + pill_w,
                    bottom: px(44.0),
                },
                ver,
            ));
        } else {
            title_right = (max_header_right - pill_w - px(10.0)).max(title_left + px(80.0));
            let clamped_pill_left = title_right + px(8.0);
            if clamped_pill_left + pill_w <= max_header_right {
                pill_rect_and_text = Some((
                    HitRect {
                        left: clamped_pill_left,
                        top: px(24.0),
                        right: clamped_pill_left + pill_w,
                        bottom: px(44.0),
                    },
                    ver,
                ));
            }
        }
    }

    if let Some((pill_rect, _)) = pill_rect_and_text {
        surface.draw_round_rect(
            pill_rect,
            6.0 * scale,
            pal.accent_tint,
            1.0,
            Some((pal.accent_pill_border, border_px, 1.0)),
        );
    }

    // 3. Middle Card Surface (Progress or Message)
    let card_rect = HitRect {
        left: px(28.0),
        top: px(88.0),
        right: px(440.0),
        bottom: px(174.0),
    };
    match &model.body {
        BodySpec::Progress {
            fraction,
            anim_phase,
            ..
        } => {
            surface.draw_round_rect(
                card_rect,
                10.0 * scale,
                pal.panel,
                1.0,
                Some((pal.border, border_px, 1.0)),
            );
            let track_rect = HitRect {
                left: px(44.0),
                top: px(140.0),
                right: px(424.0),
                bottom: px(148.0),
            };
            let track_radius = 4.0 * scale;
            surface.draw_round_rect(track_rect, track_radius, pal.elevated, 1.0, None);
            let track_w = (track_rect.right - track_rect.left) as f32;
            if let Some(frac) = fraction {
                let clamped = frac.clamp(0.0, 1.0);
                if clamped > 0.002 {
                    let fill_w = (track_w * clamped).max(8.0 * scale).min(track_w);
                    surface.draw_clipped_progress_fill(
                        track_rect,
                        track_rect.left as f32,
                        track_rect.left as f32 + fill_w,
                        track_radius,
                        pal.accent,
                    );
                }
            } else {
                // Smooth indeterminate glider with gentle ease
                let seg_w = track_w * 0.32;
                let travel = track_w + seg_w;
                let p = anim_phase.rem_euclid(1.0);
                let eased = 0.5 - 0.5 * (p * std::f32::consts::PI).cos();
                let left = (track_rect.left as f32 - seg_w) + travel * eased;
                surface.draw_clipped_progress_fill(
                    track_rect,
                    left,
                    left + seg_w,
                    track_radius,
                    pal.accent,
                );
            }
        }
        BodySpec::Message { tone, .. } => {
            let (bg, bdr) = match tone {
                CardTone::Neutral => (pal.panel, pal.border),
                CardTone::Danger => (pal.danger_tint, pal.danger_border),
            };
            surface.draw_round_rect(
                card_rect,
                10.0 * scale,
                bg,
                1.0,
                Some((bdr, border_px, 1.0)),
            );
        }
    }

    // 4. Footer Buttons (measure right-to-left, draw backgrounds onto DIB)
    let mut button_layouts: Vec<(HitRect, &ButtonSpec<'_>)> =
        Vec::with_capacity(model.buttons.len());
    let mut cursor_right = px(440.0);
    let btn_top = px(190.0);
    let btn_bottom = px(224.0);
    let min_btn_w = px(92.0);
    let btn_gap = px(10.0);

    for spec in model.buttons.iter().rev() {
        let font = match spec.role {
            ButtonRole::Secondary => fonts.button_regular,
            ButtonRole::Primary | ButtonRole::Danger => fonts.button_semibold,
        };
        let (label_w, _) = measure_single_line(surface.dc, font, spec.label);
        let btn_w = (label_w + px(36.0)).max(min_btn_w);
        let rect = HitRect {
            left: cursor_right - btn_w,
            top: btn_top,
            right: cursor_right,
            bottom: btn_bottom,
        };
        cursor_right = rect.left - btn_gap;
        button_layouts.push((rect, spec));
    }
    button_layouts.reverse();

    for &(rect, spec) in &button_layouts {
        let alpha = if spec.enabled { 1.0 } else { 0.52 };
        let (fill, border_opt) = match spec.role {
            ButtonRole::Secondary => {
                let bg = if !spec.enabled {
                    pal.secondary_bg
                } else if spec.pressed {
                    pal.secondary_pressed
                } else if spec.hovered {
                    pal.secondary_hover
                } else {
                    pal.secondary_bg
                };
                let b_col = if spec.focused && spec.enabled {
                    pal.accent
                } else {
                    pal.border
                };
                let b_w = if spec.focused && spec.enabled {
                    border_px * 1.5
                } else {
                    border_px
                };
                (bg, Some((b_col, b_w, alpha)))
            }
            ButtonRole::Primary => {
                let bg = if !spec.enabled {
                    pal.accent
                } else if spec.pressed {
                    pal.accent_pressed
                } else if spec.hovered {
                    pal.accent_hover
                } else {
                    pal.accent
                };
                let b_opt = if spec.focused && spec.enabled {
                    Some((pal.text_strong, border_px * 1.4, 0.85))
                } else {
                    None
                };
                (bg, b_opt)
            }
            ButtonRole::Danger => {
                let bg = if !spec.enabled {
                    pal.danger
                } else if spec.pressed {
                    pal.danger_pressed
                } else if spec.hovered {
                    pal.danger_hover
                } else {
                    pal.danger
                };
                let b_opt = if spec.focused && spec.enabled {
                    Some((pal.text_strong, border_px * 1.4, 0.85))
                } else {
                    None
                };
                (bg, b_opt)
            }
        };
        surface.draw_round_rect(rect, 7.0 * scale, fill, alpha, border_opt);
    }

    // Flush direct DIB pixel writes before GDI icon & ClearType text drawing
    unsafe {
        GdiFlush();
    }

    // 5. Draw Embedded VUA Icon inside the squircle badge
    if !icon.is_null() {
        let icon_px = px(32.0);
        let icon_x = badge_rect.left + ((badge_rect.right - badge_rect.left) - icon_px) / 2;
        let icon_y = badge_rect.top + ((badge_rect.bottom - badge_rect.top) - icon_px) / 2;
        unsafe {
            DrawIconEx(
                surface.dc,
                icon_x,
                icon_y,
                icon,
                icon_px,
                icon_px,
                0,
                ptr::null_mut(),
                DI_NORMAL,
            );
        }
    } else {
        // Fallback monogram if icon handle is absent (e.g. unit test without resource)
        draw_text(
            surface.dc,
            fonts.title,
            Rgb(0xf5, 0xf5, 0xf7),
            badge_rect,
            DT_SINGLELINE | DT_CENTER | DT_VCENTER,
            "V",
        );
    }

    // 6. Header Typography
    draw_text(
        surface.dc,
        fonts.title,
        pal.text_strong,
        HitRect {
            left: title_left,
            top: px(20.0),
            right: title_right,
            bottom: px(48.0),
        },
        DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
        model.title,
    );

    if let Some((pill_rect, ver)) = pill_rect_and_text {
        draw_text(
            surface.dc,
            fonts.badge,
            pal.accent,
            pill_rect,
            DT_SINGLELINE | DT_CENTER | DT_VCENTER,
            ver,
        );
    }

    draw_text(
        surface.dc,
        fonts.subtitle,
        pal.text_secondary,
        HitRect {
            left: title_left,
            top: px(49.0),
            right: max_header_right,
            bottom: px(72.0),
        },
        DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
        model.subtitle,
    );

    // 7. Middle Card Typography
    match &model.body {
        BodySpec::Progress {
            phase_label,
            metric_label,
            ..
        } => {
            let mut phase_right = px(424.0);
            if let Some(metric) = metric_label.filter(|m| !m.is_empty()) {
                let (mw, _) = measure_single_line(surface.dc, fonts.body_regular, metric);
                let metric_left = (px(424.0) - mw - px(4.0)).max(px(210.0));
                phase_right = metric_left - px(8.0);
                draw_text(
                    surface.dc,
                    fonts.body_regular,
                    pal.text_secondary,
                    HitRect {
                        left: metric_left,
                        top: px(104.0),
                        right: px(424.0),
                        bottom: px(128.0),
                    },
                    DT_SINGLELINE | DT_RIGHT | DT_VCENTER,
                    metric,
                );
            }
            draw_text(
                surface.dc,
                fonts.body_medium,
                pal.text_strong,
                HitRect {
                    left: px(44.0),
                    top: px(104.0),
                    right: phase_right,
                    bottom: px(128.0),
                },
                DT_SINGLELINE | DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
                phase_label,
            );
        }
        BodySpec::Message {
            tone,
            code_badge,
            body,
        } => {
            let inner_left = px(44.0);
            let inner_right = px(424.0);
            let card_top = px(88.0);
            let card_bottom = px(174.0);
            let body_h = if body.is_empty() {
                0
            } else {
                let mut wide_buf: Vec<u16> = body.encode_utf16().collect();
                unsafe {
                    let prev = SelectObject(surface.dc, fonts.body_regular as HGDIOBJ);
                    let mut calc = RECT {
                        left: inner_left,
                        top: 0,
                        right: inner_right,
                        bottom: 400,
                    };
                    DrawTextW(
                        surface.dc,
                        wide_buf.as_mut_ptr(),
                        wide_buf.len() as i32,
                        &mut calc,
                        DT_CALCRECT | DT_WORDBREAK | DT_EDITCONTROL_FLAG | DT_NOPREFIX,
                    );
                    SelectObject(surface.dc, prev);
                    calc.bottom - calc.top
                }
            };
            let has_code = code_badge.is_some_and(|c| !c.is_empty());
            let code_block_h = if has_code { px(21.0) } else { 0 };
            let total_h = code_block_h + body_h;
            let start_y = (card_top + ((card_bottom - card_top) - total_h) / 2).max(px(95.0));

            let mut text_top = start_y;
            if let Some(code_line) = code_badge.filter(|c| !c.is_empty()) {
                let header_color = match tone {
                    CardTone::Danger => pal.danger,
                    CardTone::Neutral => pal.accent,
                };
                draw_text(
                    surface.dc,
                    fonts.body_medium,
                    header_color,
                    HitRect {
                        left: inner_left,
                        top: start_y,
                        right: inner_right,
                        bottom: start_y + px(19.0),
                    },
                    DT_SINGLELINE | DT_LEFT | DT_VCENTER | DT_END_ELLIPSIS,
                    code_line,
                );
                text_top = start_y + code_block_h;
            }
            let body_color = match tone {
                CardTone::Neutral => pal.text_secondary,
                CardTone::Danger => pal.text_strong,
            };
            draw_text(
                surface.dc,
                fonts.body_regular,
                body_color,
                HitRect {
                    left: inner_left,
                    top: text_top,
                    right: inner_right,
                    bottom: px(168.0),
                },
                DT_WORDBREAK | DT_EDITCONTROL_FLAG | DT_LEFT,
                body,
            );
        }
    }

    // 8. Button Labels
    let mut out = RenderedLayout {
        buttons: Vec::with_capacity(button_layouts.len()),
    };
    for (rect, spec) in button_layouts {
        let (font, text_col) = match spec.role {
            ButtonRole::Secondary => (
                fonts.button_regular,
                if spec.enabled {
                    pal.text_strong
                } else {
                    pal.text_muted
                },
            ),
            ButtonRole::Primary => (fonts.button_semibold, pal.accent_text_on),
            ButtonRole::Danger => (fonts.button_semibold, pal.danger_text_on),
        };
        draw_text(
            surface.dc,
            font,
            text_col,
            rect,
            DT_SINGLELINE | DT_CENTER | DT_VCENTER,
            spec.label,
        );
        out.buttons.push((spec.action, spec.enabled, rect));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn write_bmp_24(path: &Path, surface: &DibSurface) {
        let w = surface.width as usize;
        let h = surface.height as usize;
        let row_stride = (w * 3 + 3) & !3;
        let pixel_bytes = row_stride * h;
        let file_size = 54 + pixel_bytes;
        let mut out = Vec::with_capacity(file_size);
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&(file_size as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&54u32.to_le_bytes());
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&(w as i32).to_le_bytes());
        out.extend_from_slice(&(-(h as i32)).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&24u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(pixel_bytes as u32).to_le_bytes());
        out.extend_from_slice(&2835i32.to_le_bytes());
        out.extend_from_slice(&2835i32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());

        let pixels = surface.pixels();
        for y in 0..h {
            let start = out.len();
            for x in 0..w {
                let p = pixels[y * w + x];
                out.push((p & 0xff) as u8);
                out.push(((p >> 8) & 0xff) as u8);
                out.push(((p >> 16) & 0xff) as u8);
            }
            while out.len() - start < row_stride {
                out.push(0);
            }
        }
        std::fs::write(path, out).expect("write preview bmp");
    }

    fn load_app_icon_for_test(size_px: i32) -> HICON {
        let ico_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/desktop/packaging/icon.ico");
        let wide: Vec<u16> = ico_path
            .to_string_lossy()
            .encode_utf16()
            .chain([0])
            .collect();
        unsafe {
            LoadImageW(
                ptr::null_mut(),
                wide.as_ptr(),
                IMAGE_ICON,
                size_px,
                size_px,
                LR_LOADFROMFILE,
            ) as HICON
        }
    }

    #[test]
    fn renders_crisp_hidpi_cards_in_dark_and_light_modes() {
        let dpi = 144u32; // 150% HiDPI scale
        let scale = dpi as f32 / 96.0;
        let w = (LOGICAL_WIDTH as f32 * scale).round() as i32;
        let h = (LOGICAL_HEIGHT as f32 * scale).round() as i32;
        let mut surface = DibSurface::new(w, h).expect("create DIB surface");
        let icon = load_app_icon_for_test((32.0 * scale).round() as i32);

        let zh = Lang::ZhCn.copy();
        let en = Lang::En.copy();
        let ja = Lang::Ja.copy();

        // 1. Dark - Install Downloading 64%
        let cancel_btn = [ButtonSpec {
            label: zh.btn_cancel,
            role: ButtonRole::Secondary,
            action: ButtonAction::Cancel,
            enabled: true,
            hovered: false,
            pressed: false,
            focused: false,
        }];
        let layout_install = render_card(
            &mut surface,
            dpi,
            icon,
            &CardModel {
                lang: Lang::ZhCn,
                palette: Palette::dark(),
                title: zh.install_title,
                version_badge: Some("v1.1.0"),
                subtitle: zh.install_sub_download,
                body: BodySpec::Progress {
                    phase_label: zh.install_phase_download,
                    metric_label: Some("71.8 / 112.2 MB  ·  18.4 MB/s"),
                    fraction: Some(0.64),
                    anim_phase: 0.0,
                },
                buttons: &cancel_btn,
            },
        );
        assert_eq!(layout_install.buttons.len(), 1);
        assert!(layout_install.buttons[0].2.left >= 0);

        let preview_dir = std::env::var_os("VUA_GUI_PREVIEW_DIR").map(std::path::PathBuf::from);
        if let Some(dir) = &preview_dir {
            std::fs::create_dir_all(dir).expect("preview dir");
            write_bmp_24(&dir.join("01-install-dark-zh.bmp"), &surface);
        }

        // 2. Light - Install Preparing (Indeterminate)
        render_card(
            &mut surface,
            dpi,
            icon,
            &CardModel {
                lang: Lang::ZhCn,
                palette: Palette::light(),
                title: zh.install_title,
                version_badge: Some("v1.1.0"),
                subtitle: zh.install_sub_prep,
                body: BodySpec::Progress {
                    phase_label: zh.install_phase_verify,
                    metric_label: Some("82%"),
                    fraction: Some(0.82),
                    anim_phase: 0.35,
                },
                buttons: &cancel_btn,
            },
        );
        if let Some(dir) = &preview_dir {
            write_bmp_24(&dir.join("02-install-light-zh.bmp"), &surface);
        }

        // 3. Dark - Uninstall Confirmation
        let uninstall_btns = [
            ButtonSpec {
                label: zh.btn_cancel,
                role: ButtonRole::Secondary,
                action: ButtonAction::Cancel,
                enabled: true,
                hovered: false,
                pressed: false,
                focused: false,
            },
            ButtonSpec {
                label: zh.btn_uninstall,
                role: ButtonRole::Danger,
                action: ButtonAction::ConfirmUninstall,
                enabled: true,
                hovered: false,
                pressed: false,
                focused: false,
            },
        ];
        let layout_uninstall = render_card(
            &mut surface,
            dpi,
            icon,
            &CardModel {
                lang: Lang::ZhCn,
                palette: Palette::dark(),
                title: zh.uninstall_confirm_title,
                version_badge: Some("v1.1.0"),
                subtitle: zh.uninstall_confirm_sub,
                body: BodySpec::Message {
                    tone: CardTone::Neutral,
                    code_badge: None,
                    body: zh.uninstall_confirm_body,
                },
                buttons: &uninstall_btns,
            },
        );
        assert_eq!(layout_uninstall.buttons.len(), 2);
        assert!(layout_uninstall.buttons[0].2.right < layout_uninstall.buttons[1].2.left);
        if let Some(dir) = &preview_dir {
            write_bmp_24(&dir.join("03-uninstall-dark-zh.bmp"), &surface);
        }

        // 4. Light - Uninstall Confirmation (English)
        let uninstall_en_btns = [
            ButtonSpec {
                label: en.btn_cancel,
                role: ButtonRole::Secondary,
                action: ButtonAction::Cancel,
                enabled: true,
                hovered: false,
                pressed: false,
                focused: false,
            },
            ButtonSpec {
                label: en.btn_uninstall,
                role: ButtonRole::Danger,
                action: ButtonAction::ConfirmUninstall,
                enabled: true,
                hovered: false,
                pressed: false,
                focused: false,
            },
        ];
        render_card(
            &mut surface,
            dpi,
            icon,
            &CardModel {
                lang: Lang::En,
                palette: Palette::light(),
                title: en.uninstall_confirm_title,
                version_badge: Some("v1.1.0"),
                subtitle: en.uninstall_confirm_sub,
                body: BodySpec::Message {
                    tone: CardTone::Neutral,
                    code_badge: None,
                    body: en.uninstall_confirm_body,
                },
                buttons: &uninstall_en_btns,
            },
        );
        if let Some(dir) = &preview_dir {
            write_bmp_24(&dir.join("04-uninstall-light-en.bmp"), &surface);
        }

        // 5. Dark - Busy Retry Error (Japanese, long button text check!)
        let retry_ja_btns = [
            ButtonSpec {
                label: ja.btn_close,
                role: ButtonRole::Secondary,
                action: ButtonAction::Close,
                enabled: true,
                hovered: false,
                pressed: false,
                focused: false,
            },
            ButtonSpec {
                label: ja.btn_retry,
                role: ButtonRole::Primary,
                action: ButtonAction::Retry,
                enabled: true,
                hovered: true,
                pressed: false,
                focused: false,
            },
        ];
        let layout_ja = render_card(
            &mut surface,
            dpi,
            icon,
            &CardModel {
                lang: Lang::Ja,
                palette: Palette::dark(),
                title: ja.install_error_title,
                version_badge: None,
                subtitle: ja.install_error_sub,
                body: BodySpec::Message {
                    tone: CardTone::Danger,
                    code_badge: None,
                    body: ja.install_error_busy,
                },
                buttons: &retry_ja_btns,
            },
        );
        assert_eq!(layout_ja.buttons.len(), 2);
        if let Some(dir) = &preview_dir {
            write_bmp_24(&dir.join("05-error-busy-dark-ja.bmp"), &surface);
        }

        // 6. Dark - Launch Failure Diagnostic Card
        let close_zh_btn = [ButtonSpec {
            label: zh.btn_close,
            role: ButtonRole::Primary,
            action: ButtonAction::Close,
            enabled: true,
            hovered: false,
            pressed: false,
            focused: false,
        }];
        let code_line = format!("{}installed_files_need_repair", zh.failure_code_prefix);
        render_card(
            &mut surface,
            dpi,
            icon,
            &CardModel {
                lang: Lang::ZhCn,
                palette: Palette::dark(),
                title: zh.failure_title,
                version_badge: None,
                subtitle: zh.failure_sub,
                body: BodySpec::Message {
                    tone: CardTone::Danger,
                    code_badge: Some(&code_line),
                    body: zh.failure_body,
                },
                buttons: &close_zh_btn,
            },
        );
        if let Some(dir) = &preview_dir {
            write_bmp_24(&dir.join("06-failure-dark-zh.bmp"), &surface);
        }

        if !icon.is_null() {
            unsafe {
                DestroyIcon(icon);
            }
        }
    }
}
