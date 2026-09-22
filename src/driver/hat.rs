use std::{path::Path, thread, time::Duration};

use super::pca9633;
use anyhow::{Ok, Result};
use embedded_hal::i2c::I2c;

const RGB_DEV_ADDR: u8 = 0x2d;

pub struct Driver<TDev> {
    backlight: pca9633::Driver<TDev>,
}

impl<TDev: I2c> Driver<TDev> {
    pub fn new(rgb_dev: TDev) -> Result<Self> {
        let rgb = pca9633::Driver::new(RGB_DEV_ADDR, rgb_dev)?;

        Ok(Self { backlight: rgb })
    }

    pub fn init(&mut self) -> Result<()> {
        self.backlight.init()?;
        self.set_backlight_rgb(0, 0, 0)?;
        Ok(())
    }

    pub fn set_backlight_brightness(&mut self, b: u8) -> Result<()> {
        self.backlight.set_reg(pca9633::Reg::GrpPwm, b)
    }

    pub fn set_backlight_rgb(&mut self, r: u8, g: u8, b: u8) -> Result<()> {
        self.backlight.set_reg(pca9633::Reg::Pwm2, r)?;
        self.backlight.set_reg(pca9633::Reg::Pwm1, g)?;
        self.backlight.set_reg(pca9633::Reg::Pwm0, b)?;
        Ok(())
    }
}

impl Driver<linux_embedded_hal::I2cdev> {
    pub fn new_linux<P: AsRef<Path>>(p: P) -> Result<Self> {
        let rgb_dev = linux_embedded_hal::I2cdev::new(p)?;
        thread::sleep(Duration::from_secs(1));
        Self::new(rgb_dev)
    }
}
