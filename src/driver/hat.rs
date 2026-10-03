use std::{path::Path, thread, time::Duration};

use super::{button, pca9633};
use anyhow::{Context, Ok, Result};
use embedded_hal::{digital::InputPin, i2c::I2c};
use log::info;

const RGB_DEV_ADDR: u8 = 0x2d;

pub struct Driver<TDev, Pin> {
    pub backlight: pca9633::Driver<TDev>,

    pub up_button: button::Driver<Pin>,
    pub down_button: button::Driver<Pin>,
    pub left_button: button::Driver<Pin>,
    pub right_button: button::Driver<Pin>,
    pub select_button: button::Driver<Pin>,
}

impl<I2CBusDev: I2c, Pin: InputPin> Driver<I2CBusDev, Pin> {
    pub fn new(
        i2c_bus: I2CBusDev,
        up_button: Pin,
        down_button: Pin,
        left_button: Pin,
        right_button: Pin,
        select_button: Pin,
    ) -> Result<Self> {
        let rgb = pca9633::Driver::new(RGB_DEV_ADDR, i2c_bus)?;

        Ok(Self {
            backlight: rgb,
            up_button: button::Driver::new(up_button),
            down_button: button::Driver::new(down_button),
            left_button: button::Driver::new(left_button),
            right_button: button::Driver::new(right_button),
            select_button: button::Driver::new(select_button),
        })
    }

    pub fn init(&mut self) -> Result<()> {
        self.backlight.init()?;
        self.set_backlight_rgb(0, 0, 0)?;
        Ok(())
    }

    pub fn set_backlight_rgb(&mut self, r: u8, g: u8, b: u8) -> Result<()> {
        self.backlight.set_reg(pca9633::Reg::Pwm2, r)?;
        self.backlight.set_reg(pca9633::Reg::Pwm1, g)?;
        self.backlight.set_reg(pca9633::Reg::Pwm0, b)?;
        Ok(())
    }
}

const RPI4B_I2C1_BUS_PATH: &str = "/dev/i2c-1";
const RPI4B_GPIO_CHIP_PATH: &str = "/dev/gpiochip0";

impl Driver<linux_embedded_hal::I2cdev, linux_embedded_hal::CdevPin> {
    pub fn new_linux<P: AsRef<Path>>(
        i2c_bus_path: P,
        gpio_chip_path: P,
        up_pin: u32,
        down_pin: u32,
        left_pin: u32,
        right_pin: u32,
        select_pin: u32,
    ) -> Result<Self> {
        info!("hat: using i2c bus {:?}", i2c_bus_path.as_ref());
        let i2c_bus = linux_embedded_hal::I2cdev::new(i2c_bus_path)
            .context("failed to initialize i2c bus")?;
        info!("hat: using gpiochip {:?}", gpio_chip_path.as_ref());
        let mut gpio_chip =
            gpio_cdev::Chip::new(gpio_chip_path).context("failed to initialize gpio chip")?;
        let up_pin = Self::setup_pin("up", &mut gpio_chip, up_pin)?;
        let down_pin = Self::setup_pin("down", &mut gpio_chip, down_pin)?;
        let left_pin = Self::setup_pin("left", &mut gpio_chip, left_pin)?;
        let right_pin = Self::setup_pin("right", &mut gpio_chip, right_pin)?;
        let select_pin = Self::setup_pin("select", &mut gpio_chip, select_pin)?;
        thread::sleep(Duration::from_secs(1));
        Self::new(i2c_bus, up_pin, down_pin, left_pin, right_pin, select_pin)
    }

    pub fn new_rpi_4b() -> Result<Self> {
        Self::new_linux(
            RPI4B_I2C1_BUS_PATH,
            RPI4B_GPIO_CHIP_PATH,
            17,
            18,
            19,
            20,
            16,
        )
    }

    fn setup_pin(
        lbl: &str,
        chip: &mut gpio_cdev::Chip,
        offset: u32,
    ) -> Result<linux_embedded_hal::CdevPin> {
        let mut inner = || {
            let line = chip.get_line(offset)?;
            let line_handle = line.request(
                gpio_cdev::LineRequestFlags::INPUT,
                0,
                &format!("audio-relay-hat-btn-{}", lbl),
            )?;
            let pin = linux_embedded_hal::CdevPin::new(line_handle)?;
            let pin = pin.into_input_pin()?;
            Ok(pin)
        };
        inner().context(format!("setup_pin: offset {}, label {}", offset, lbl))
    }
}
