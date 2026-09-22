use anyhow::{Ok, Result, anyhow};
use embedded_hal::i2c::I2c;

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum Reg {
    Mode1 = 0x00,
    Mode2 = 0x01,
    Pwm0 = 0x02,
    Pwm1 = 0x03,
    Pwm2 = 0x04,
    Pwm3 = 0x05,
    GrpPwm = 0x06,
    GrpFreq = 0x07,
    LedOut = 0x08,
    SubAdr1 = 0x09,
    SubAdr2 = 0x0a,
    SubAdr3 = 0x0b,
    AllCallAdr = 0x0c,
}

impl std::fmt::Display for Reg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Reg::Mode1 => "MODE1",
            Reg::Mode2 => "MODE2",
            Reg::Pwm0 => "PWM0",
            Reg::Pwm1 => "PWM1",
            Reg::Pwm2 => "PWM2",
            Reg::Pwm3 => "PWM3",
            Reg::GrpPwm => "GRPPWM",
            Reg::GrpFreq => "GRPFREQ",
            Reg::LedOut => "LEDOUT",
            Reg::SubAdr1 => "SUBADR1",
            Reg::SubAdr2 => "SUBADR2",
            Reg::SubAdr3 => "SUBADR3",
            Reg::AllCallAdr => "ALLCALLADR",
        };
        write!(f, "{name} ({:#04x})", *self as u8)
    }
}

pub(super) struct Driver<TDev> {
    addr: u8,
    i2c: TDev,
}

impl<TDev: I2c> Driver<TDev> {
    pub(super) fn new(addr: u8, mut i2c: TDev) -> Result<Self> {
        // Sanity check: device presents?
        i2c.write_read(addr, &[Reg::Mode1 as u8], &mut [0])
            .map_err(|err| anyhow!("PCA9633 doesn't seem to present at {:#x}: {:?}", addr, err))?;
        Ok(Self { addr, i2c })
    }

    fn set_reg(&mut self, reg: Reg, val: u8) -> Result<()> {
        self.i2c
            .write(self.addr, &[reg as u8, val])
            .map_err(|err| anyhow!("failed to set reg {}: {:?}", reg, err))?;
        Ok(())
    }

    fn set_rgb(&mut self, r: u8, g: u8, b: u8) -> Result<()> {
        self.set_reg(Reg::SubAdr1, r)?;
        self.set_reg(Reg::SubAdr2, g)?;
        self.set_reg(Reg::SubAdr2, b)?;
        Ok(())
    }

    pub fn init(&mut self) -> Result<()> {
        self.set_reg(Reg::Mode1, 0x00)?;
        self.set_reg(Reg::LedOut, 0xFF)?; // set controlable by both PWM and GRPPWN registers
        self.set_reg(Reg::Mode1, 0x20 | 0x05)?; // DMBLNK, OUTNE
        self.set_rgb(0, 0, 0)?;
        Ok(())
    }
}
