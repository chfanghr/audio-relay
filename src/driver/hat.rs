use std::{path::Path, thread, time::Duration};

use crate::driver::button::Button;

use super::pca9633;
use anyhow::{Context, Ok, Result};
use derive_getters::Getters;
use embedded_hal::{digital::InputPin, i2c::I2c};

const RGB_DEV_ADDR: u8 = 0x2d;

#[derive(Getters)]
pub struct Driver<TDev, Pin> {
    backlight: pca9633::Driver<TDev>,

    up_button: Button<Pin>,
    down_button: Button<Pin>,
    left_button: Button<Pin>,
    right_button: Button<Pin>,
    select_button: Button<Pin>,
}

impl<TDev: I2c, Pin: InputPin> Driver<TDev, Pin> {
    pub fn new(
        rgb_dev: TDev,
        up_button: Pin,
        down_button: Pin,
        left_button: Pin,
        right_button: Pin,
        select_button: Pin,
    ) -> Result<Self> {
        let rgb = pca9633::Driver::new(RGB_DEV_ADDR, rgb_dev)?;

        Ok(Self {
            backlight: rgb,
            up_button: Button::new(up_button),
            down_button: Button::new(down_button),
            left_button: Button::new(left_button),
            right_button: Button::new(right_button),
            select_button: Button::new(select_button),
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

impl Driver<linux_embedded_hal::I2cdev, linux_embedded_hal::SysfsPin> {
    pub fn new_linux<P: AsRef<Path>>(
        p: P,
        up_pin: u64,
        down_pin: u64,
        left_pin: u64,
        right_pin: u64,
        select_pin: u64,
    ) -> Result<Self> {
        let rgb_dev = linux_embedded_hal::I2cdev::new(p)?;
        let up_pin = Self::setup_pin(up_pin)?;
        let down_pin = Self::setup_pin(down_pin)?;
        let left_pin = Self::setup_pin(left_pin)?;
        let right_pin = Self::setup_pin(right_pin)?;
        let select_pin = Self::setup_pin(select_pin)?;
        thread::sleep(Duration::from_secs(1));
        Self::new(rgb_dev, up_pin, down_pin, left_pin, right_pin, select_pin)
    }

    pub fn new_rpi_4b() -> Result<Self> {
        Self::new_linux(RPI4B_I2C1_BUS_PATH, 17, 18, 19, 20, 16)
    }

    fn setup_pin(n: u64) -> Result<linux_embedded_hal::SysfsPin> {
        let p = linux_embedded_hal::SysfsPin::new(n);
        p.export().context(format!("failed to export pin {}", n))?;
        let p = p
            .into_input_pin()
            .context(format!("failed to set pin {} to input mode", n))?;
        Ok(p)
    }
}
