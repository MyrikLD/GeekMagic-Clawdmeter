use crate::{api::UsageData, display::Display};
use embedded_graphics::{
    mono_font::{ascii::*, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::{Alignment, Text},
};

const BLACK: Rgb565 = Rgb565::BLACK;
const WHITE: Rgb565 = Rgb565::WHITE;
const CLAUDE_ORANGE: Rgb565 = Rgb565::new(31, 20, 4);
const CLAUDE_DARK: Rgb565 = Rgb565::new(3, 6, 8);
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

    // ── 5h window ─────────────────────────────────────────────────────────────
    let pct_5h = format!("5h:  {:.1}%", d.util_5h * 100.0);
    text_left(disp, &pct_5h, 50, WHITE, &FONT_6X10)?;
    draw_bar(disp, 56, d.util_5h)?;

    // ── 7d window ─────────────────────────────────────────────────────────────
    let pct_7d = format!("7d:  {:.1}%", d.util_7d * 100.0);
    text_left(disp, &pct_7d, 82, WHITE, &FONT_6X10)?;
    draw_bar(disp, 88, d.util_7d)?;

    // ── Status ────────────────────────────────────────────────────────────────
    let (status_str, status_color) = if d.allowed {
        ("ALLOWED", GREEN)
    } else {
        ("RATE LIMITED", RED)
    };
    text_center(disp, status_str, 125, status_color, &FONT_9X18_BOLD)?;

    // ── Big utilization % ─────────────────────────────────────────────────────
    let big = format!("{:.0}%", d.util_5h * 100.0);
    text_center(disp, &big, 170, CLAUDE_ORANGE, &FONT_10X20)?;
    text_center(disp, "5h utilization", 188, WHITE, &FONT_6X10)?;

    // ── Time until reset ─────────────────────────────────────────────────────
    if d.reset_5h > 0 && d.now_ts > 0 && d.reset_5h > d.now_ts {
        let secs = d.reset_5h - d.now_ts;
        let reset_str = format!("resets in {}h {:02}m", secs / 3600, (secs % 3600) / 60);
        text_center(disp, &reset_str, 215, WHITE, &FONT_6X10)?;
    }

    Ok(())
}

pub fn draw_error(disp: &mut Display<'_>, msg: &str) -> anyhow::Result<()> {
    disp.clear(CLAUDE_DARK)?;
    text_center(disp, "Error", 90, RED, &FONT_9X18_BOLD)?;
    let snippet = &msg[..msg.len().min(40)];
    text_center(disp, snippet, 130, WHITE, &FONT_6X10)?;
    Ok(())
}

fn draw_bar(disp: &mut Display<'_>, y: i32, fraction: f32) -> anyhow::Result<()> {
    let margin = 10i32;
    let bar_w = W - margin * 2;
    let bar_h = 8i32;

    Rectangle::new(Point::new(margin, y), Size::new(bar_w as u32, bar_h as u32))
        .into_styled(PrimitiveStyleBuilder::new().fill_color(Rgb565::new(10, 20, 10)).build())
        .draw(disp)?;

    let fill_w = ((bar_w as f32 * fraction) as i32).clamp(0, bar_w);
    if fill_w > 0 {
        Rectangle::new(Point::new(margin, y), Size::new(fill_w as u32, bar_h as u32))
            .into_styled(PrimitiveStyleBuilder::new().fill_color(usage_color(fraction)).build())
            .draw(disp)?;
    }
    Ok(())
}

fn usage_color(f: f32) -> Rgb565 {
    if f < 0.6 { GREEN } else if f < 0.85 { YELLOW } else { RED }
}

fn text_center(
    disp: &mut Display<'_>,
    s: &str,
    y: i32,
    color: Rgb565,
    font: &embedded_graphics::mono_font::MonoFont<'_>,
) -> anyhow::Result<()> {
    Text::with_alignment(s, Point::new(W / 2, y), MonoTextStyle::new(font, color), Alignment::Center)
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
