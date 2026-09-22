use std::path::Path;

use super::pca9633;
use anyhow::{Ok, Result};
use embedded_hal::i2c::I2c;

const RGB_DEV_ADDR: u8 = 0x2d;

pub struct Driver<TDev> {
    rgb: pca9633::Driver<TDev>,
}

impl<TDev: I2c> Driver<TDev> {
    pub fn new(rgb_dev: TDev) -> Result<Self> {
        let rgb = pca9633::Driver::new(RGB_DEV_ADDR, rgb_dev)?;

        Ok(Self { rgb })
    }

    pub fn init(&mut self) -> Result<()> {
        self.rgb.init()?;
        Ok(())
    }
}

impl Driver<linux_embedded_hal::I2cdev> {
    pub fn new_linux<P: AsRef<Path>>(p: P) -> Result<Self> {
        let rgb_dev = linux_embedded_hal::I2cdev::new(p)?;
        Self::new(rgb_dev)
    }
}
