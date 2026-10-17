mod audio;
mod driver;

use anyhow::{Result, anyhow};

use crate::{audio::demo_main, driver::hat};

fn main() -> Result<()> {
    env_logger::init();

    // let mut hat = hat::Driver::new_rpi_4b()?;
    // hat.init()?;

    // Ok(())
    demo_main()?;

    Ok(())
}
