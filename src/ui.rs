use crate::{api::UsageData, display::Display};
use embedded_graphics::{
    mono_font::{ascii::*, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::{Alignment, Text},
};

// ── Palette ───────────────────────────────────────────────────────────────────
const BLACK: Rgb565 = Rgb565::BLACK;
const WHITE: Rgb565 = Rgb565::WHITE;
const CLAUDE_ORANGE: Rgb565 = Rgb565::new(31, 20, 4); // #FF8C20 approx in RGB565
const CLAUDE_DARK: Rgb565 = Rgb565::new(3, 6, 8);     // ~#1A1A2E
const GREEN: Rgb565 = Rgb565::new(0, 40, 0);
const YELLOW: Rgb565 = Rgb565::new(31, 50, 0);
const RED: Rgb565 = Rgb565::new(31, 10, 2);

const W: i32 = crate::display::WIDTH as i32;
const H: i32 = crate::display::HEIGHT as i32;

pub fn splash(disp: &mut Display<'_>) -> anyhow::Result<()> {
    disp.clear(CLAUDE_DARK)?;
    text_center(disp, "Clawdmeter", H / 2 - 10, CLAUDE_ORANGE, &FONT_10X20)?;
    text_center(disp, "by esp32-rs", H / 2 + 15, WHITE, &FONT_6X10)?;
    Ok(())
}

pub fn status(disp: &mut Display<'_>, msg: &str) -> anyhow::Result<()> {
    // Only repaint the bottom strip, keep the rest of the screen intact
    let strip = Rectangle::new(Point::new(0, H - 16), Size::new(W as u32, 16));
    strip.into_styled(
        PrimitiveStyleBuilder::new().fill_color(CLAUDE_DARK).build(),
    ).draw(disp)?;
    text_center(disp, msg, H - 5, WHITE, &FONT_6X10)?;
    Ok(())
}

pub fn draw_usage(disp: &mut Display<'_>, d: &UsageData) -> anyhow::Result<()> {
    disp.clear(CLAUDE_DARK)?;

    // ── Title bar ─────────────────────────────────────────────────────────────
    Rectangle::new(Point::zero(), Size::new(W as u32, 28))
        .into_styled(PrimitiveStyleBuilder::new().fill_color(CLAUDE_ORANGE).build())
        .draw(disp)?;
    text_center(disp, "Claude Usage", 18, BLACK, &FONT_9X18_BOLD)?;

    // ── Tokens section ────────────────────────────────────────────────────────
    let label = format_u32("Tokens", d.tokens_remaining, d.tokens_limit);
    text_left(disp, &label, 52, WHITE, &FONT_6X10)?;
    draw_bar(disp, 60, d.tokens_fraction())?;

    // ── Requests section ──────────────────────────────────────────────────────
    let req_label = format_u32("Requests", d.requests_remaining, d.requests_limit);
    text_left(disp, &req_label, 90, WHITE, &FONT_6X10)?;
    draw_bar(disp, 98, requests_fraction(d))?;

    // ── Big remaining token number ────────────────────────────────────────────
    let big = format_big(d.tokens_remaining);
    text_center(disp, &big, 155, CLAUDE_ORANGE, &FONT_10X20)?;
    text_center(disp, "tokens left", 175, WHITE, &FONT_6X10)?;

    // ── Reset time ────────────────────────────────────────────────────────────
    if !d.reset_at.is_empty() {
        let reset_str = format!("Resets: {}", hhmm(&d.reset_at));
        text_center(disp, &reset_str, 210, WHITE, &FONT_6X10)?;
    }

    Ok(())
}

pub fn draw_error(disp: &mut Display<'_>, msg: &str) -> anyhow::Result<()> {
    disp.clear(CLAUDE_DARK)?;
    text_center(disp, "Error", 90, RED, &FONT_9X18_BOLD)?;
    // Show first 40 chars of the error
    let snippet = &msg[..msg.len().min(40)];
    text_center(disp, snippet, 130, WHITE, &FONT_6X10)?;
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn draw_bar(disp: &mut Display<'_>, y: i32, fraction: f32) -> anyhow::Result<()> {
    let margin = 10i32;
    let bar_w = W - margin * 2;
    let bar_h = 8i32;

    // Background
    Rectangle::new(Point::new(margin, y), Size::new(bar_w as u32, bar_h as u32))
        .into_styled(PrimitiveStyleBuilder::new().fill_color(Rgb565::new(10, 20, 10)).build())
        .draw(disp)?;

    // Fill — color shifts green → yellow → red as usage grows
    let fill_w = ((bar_w as f32 * fraction) as i32).clamp(0, bar_w);
    if fill_w > 0 {
        let fill_color = usage_color(fraction);
        Rectangle::new(Point::new(margin, y), Size::new(fill_w as u32, bar_h as u32))
            .into_styled(PrimitiveStyleBuilder::new().fill_color(fill_color).build())
            .draw(disp)?;
    }
    Ok(())
}

fn usage_color(f: f32) -> Rgb565 {
    if f < 0.6 {
        GREEN
    } else if f < 0.85 {
        YELLOW
    } else {
        RED
    }
}

fn text_center(
    disp: &mut Display<'_>,
    s: &str,
    y: i32,
    color: Rgb565,
    font: &embedded_graphics::mono_font::MonoFont<'_>,
) -> anyhow::Result<()> {
    Text::with_alignment(
        s,
        Point::new(W / 2, y),
        MonoTextStyle::new(font, color),
        Alignment::Center,
    )
    .draw(disp)?;
    Ok(())
}

fn text_left(
    disp: &mut Display<'_>,
    s: &str,
    y: i32,
    color: Rgb565,
    font: &embedded_graphics::mono_font::MonoFont<'_>,
) -> anyhow::Result<()> {
    Text::new(s, Point::new(10, y), MonoTextStyle::new(font, color)).draw(disp)?;
    Ok(())
}

fn format_u32(label: &str, remaining: u32, limit: u32) -> String {
    format!("{label}: {remaining}/{limit}")
}

fn format_big(n: u32) -> String {
    format!("{n}")
}

fn requests_fraction(d: &UsageData) -> f32 {
    if d.requests_limit == 0 {
        return 0.0;
    }
    let used = d.requests_limit.saturating_sub(d.requests_remaining);
    used as f32 / d.requests_limit as f32
}

/// Extract HH:MM from an ISO-8601 string like "2025-05-15T14:30:00Z"
fn hhmm(s: &str) -> &str {
    // Find 'T' and take 5 chars after it
    if let Some(t) = s.find('T') {
        let rest = &s[t + 1..];
        &rest[..rest.len().min(5)]
    } else {
        s
    }
}
