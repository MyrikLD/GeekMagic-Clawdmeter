use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{Dimensions, Size},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::Rectangle,
    Pixel,
};
use embedded_hal::spi::SpiDevice;
use esp_idf_hal::{
    delay::FreeRtos,
    gpio::{Gpio18, Gpio2, Gpio21, Gpio23, Gpio3, Gpio4, Output, PinDriver},
    peripheral::Peripheral,
    spi::{
        config::{Config as SpiConfig, DriverConfig, MODE_3},
        SpiDeviceDriver, SpiDriver, SPI2,
    },
    units::FromValueType,
};

// ST7789 commands
const SWRESET: u8 = 0x01;
const SLPOUT: u8 = 0x11;
const COLMOD: u8 = 0x3A;
const MADCTL: u8 = 0x36;
const CASET: u8 = 0x2A;
const RASET: u8 = 0x2B;
const RAMWR: u8 = 0x2C;
const INVON: u8 = 0x21;
const NORON: u8 = 0x13;
const DISPON: u8 = 0x29;

pub const WIDTH: u16 = 240;
pub const HEIGHT: u16 = 240;

// Row buffer size: one full row of 16-bit pixels
const ROW_BYTES: usize = WIDTH as usize * 2;

pub struct Display<'d> {
    spi: SpiDeviceDriver<'d, SpiDriver<'d>>,
    dc: PinDriver<'d, Gpio2, Output>,
}

impl<'d> Display<'d> {
    pub fn new(
        spi2: impl Peripheral<P = SPI2> + 'd,
        clk: impl Peripheral<P = Gpio18> + 'd,
        mosi: impl Peripheral<P = Gpio23> + 'd,
        cs: impl Peripheral<P = Gpio3> + 'd,
        dc: impl Peripheral<P = Gpio2> + 'd,
        rst: impl Peripheral<P = Gpio4> + 'd,
        bl: impl Peripheral<P = Gpio21> + 'd,
    ) -> anyhow::Result<Self> {
        // Backlight on (keep pin live by leaking — it lives for the program duration)
        let mut bl_pin = PinDriver::output(bl)?;
        bl_pin.set_high()?;
        Box::leak(Box::new(bl_pin));

        // Hardware reset
        let mut rst_pin = PinDriver::output(rst)?;
        rst_pin.set_high()?;
        FreeRtos::delay_ms(10);
        rst_pin.set_low()?;
        FreeRtos::delay_ms(10);
        rst_pin.set_high()?;
        FreeRtos::delay_ms(120);
        Box::leak(Box::new(rst_pin));

        let dc_pin = PinDriver::output(dc)?;

        let spi_dev = SpiDeviceDriver::new_single(
            spi2,
            clk,
            mosi,
            Option::<esp_idf_hal::gpio::AnyIOPin>::None, // MISO not needed
            Some(cs),
            &DriverConfig::new(),
            &SpiConfig::new()
                .baudrate(20_u32.MHz().into())
                .data_mode(MODE_3),
        )?;

        let mut disp = Self { spi: spi_dev, dc: dc_pin };
        disp.init()?;
        Ok(disp)
    }

    fn cmd(&mut self, cmd: u8) -> anyhow::Result<()> {
        self.dc.set_low()?;
        self.spi.write(&[cmd])?;
        Ok(())
    }

    fn data(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.dc.set_high()?;
        self.spi.write(data)?;
        Ok(())
    }

    fn init(&mut self) -> anyhow::Result<()> {
        self.cmd(SWRESET)?;
        FreeRtos::delay_ms(150);
        self.cmd(SLPOUT)?;
        FreeRtos::delay_ms(10);

        self.cmd(COLMOD)?;
        self.data(&[0x55])?; // 16-bit color (RGB565)

        self.cmd(MADCTL)?;
        self.data(&[0x00])?; // row/col order — adjust if display is rotated

        self.cmd(INVON)?; // ST7789 needs inversion
        self.cmd(NORON)?;
        FreeRtos::delay_ms(10);
        self.cmd(DISPON)?;
        FreeRtos::delay_ms(10);

        Ok(())
    }

    fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> anyhow::Result<()> {
        self.cmd(CASET)?;
        self.data(&[
            (x0 >> 8) as u8,
            x0 as u8,
            (x1 >> 8) as u8,
            x1 as u8,
        ])?;
        self.cmd(RASET)?;
        self.data(&[
            (y0 >> 8) as u8,
            y0 as u8,
            (y1 >> 8) as u8,
            y1 as u8,
        ])?;
        self.cmd(RAMWR)?;
        Ok(())
    }

    // Efficient bulk fill: send one pre-built row repeatedly
    fn fill_region(
        &mut self,
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
        color: Rgb565,
    ) -> anyhow::Result<()> {
        let raw = color.into_storage();
        let hi = (raw >> 8) as u8;
        let lo = raw as u8;
        let w = (x1 - x0 + 1) as usize;
        let h = (y1 - y0 + 1) as usize;

        self.set_window(x0, y0, x1, y1)?;
        self.dc.set_high()?;

        // Build one row in a stack buffer and write it h times
        let mut row = [0u8; ROW_BYTES];
        for i in (0..w * 2).step_by(2) {
            row[i] = hi;
            row[i + 1] = lo;
        }
        for _ in 0..h {
            self.spi.write(&row[..w * 2])?;
        }
        Ok(())
    }
}

impl DrawTarget for Display<'_> {
    type Color = Rgb565;
    type Error = anyhow::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Rgb565>>,
    {
        for Pixel(pt, color) in pixels {
            if pt.x < 0 || pt.y < 0 || pt.x >= WIDTH as i32 || pt.y >= HEIGHT as i32 {
                continue;
            }
            let x = pt.x as u16;
            let y = pt.y as u16;
            let raw = color.into_storage();
            self.set_window(x, y, x, y)?;
            self.dc.set_high()?;
            self.spi.write(&[(raw >> 8) as u8, raw as u8])?;
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Rgb565) -> Result<(), Self::Error> {
        let area = area.intersection(&self.bounding_box());
        if area.is_zero_sized() {
            return Ok(());
        }
        let x0 = area.top_left.x as u16;
        let y0 = area.top_left.y as u16;
        let x1 = x0 + area.size.width as u16 - 1;
        let y1 = y0 + area.size.height as u16 - 1;
        self.fill_region(x0, y0, x1, y1, color)
    }
}

impl OriginDimensions for Display<'_> {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}
