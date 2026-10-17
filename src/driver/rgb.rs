use anyhow::{Ok, Result, anyhow};
use embedded_hal::i2c::I2c;
use log::warn;

/// By kimi k3:
///
/// Backlight controller of the DFR0514 keypad HAT (V2.0 LCD1602 RGB module).
///
/// Although the DFR0514 datasheet claims "PCA9633DP2 or Equivalent", modules
/// that answer at 0x2d are NOT PCA9633s: it's a small MCU exposing a minimal
/// I2C register file:
///
/// ```text
/// 0x00  R   device id, always reads back the slave address (0x2d)
/// 0x01  RW  red   PWM duty (0x00 = off .. 0xff = max)
/// 0x02  RW  green PWM duty
/// 0x03  RW  blue  PWM duty
/// else  R   unimplemented, reads 0xff
/// ```
///
/// (Verified via a power-on register dump and a per-register color test on a
/// DFR0514 HAT; matches the V2.0 mapping in DFRobot's DFRobot_RGBLCD1602
/// Arduino library.)
mod reg {
    pub const ID: u8 = 0x00;
    pub const RED: u8 = 0x01;
    pub const GREEN: u8 = 0x02;
    pub const BLUE: u8 = 0x03;
}

pub struct Driver<TDev> {
    addr: u8,
    i2c: TDev,
}

impl<TDev: I2c> Driver<TDev> {
    pub fn new(addr: u8, mut i2c: TDev) -> Result<Self> {
        // Sanity check: device presents?
        let mut id = [0u8; 1];
        i2c.write_read(addr, &[reg::ID], &mut id).map_err(|err| {
            anyhow!(
                "backlight controller doesn't seem to present at {:#x}: {:?}",
                addr,
                err
            )
        })?;
        if id[0] != addr {
            warn!(
                "backlight controller at {:#x} reports unexpected id {:#x}",
                addr, id[0]
            );
        }
        Ok(Self { addr, i2c })
    }

    fn set_reg(&mut self, reg: u8, val: u8) -> Result<()> {
        self.i2c
            .write(self.addr, &[reg, val])
            .map_err(|err| anyhow!("failed to set reg {:#04x}: {:?}", reg, err))?;
        Ok(())
    }

    pub fn set_rgb(&mut self, r: u8, g: u8, b: u8) -> Result<()> {
        self.set_reg(reg::RED, r)?;
        self.set_reg(reg::GREEN, g)?;
        self.set_reg(reg::BLUE, b)?;
        Ok(())
    }

    pub fn init(&mut self) -> Result<()> {
        // No init sequence needed; just start with the backlight off.
        self.set_rgb(0, 0, 0)
    }
}
