mod driver;

use anyhow::Result;

use crate::driver::hat;

fn main() -> Result<()> {
    env_logger::init();

    let mut hat = hat::Driver::new_rpi_4b()?;
    hat.init()?;

    Ok(())
}
