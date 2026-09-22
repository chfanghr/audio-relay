mod driver;

use anyhow::Result;

const RPI4_I2C1_BUS_PATH: &str = "/dev/i2c-1";
fn main() -> Result<()> {
    let mut hat = driver::hat::Driver::new_linux(RPI4_I2C1_BUS_PATH)?;
    hat.init()?;

    Ok(())
}
