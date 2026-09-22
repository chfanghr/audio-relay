mod driver;

use std::{thread, time::Duration};

use anyhow::Result;
use backon::{BlockingRetryableWithContext, ConstantBuilder};
use log::info;

use crate::driver::hat;

const RPI4_I2C1_BUS_PATH: &str = "/dev/i2c-1";

fn main() -> Result<()> {
    let mut hat = driver::hat::Driver::new_linux(RPI4_I2C1_BUS_PATH)?;
    hat.init()?;

    info!("hat initialized");

    let color_channel_range = u8::MIN..u8::MAX;
    loop {
        for r in color_channel_range.clone() {
            for g in color_channel_range.clone() {
                for b in color_channel_range.clone() {
                    stupid
                        .retry(ConstantBuilder::with_delay(
                            ConstantBuilder::new(),
                            Duration::from_millis(100),
                        ))
                        .context((&mut hat, r, g, b))
                        .call()
                        .1?;
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }
}

type StupidInput<'a> = (&'a mut hat::Driver<linux_embedded_hal::I2cdev>, u8, u8, u8);

fn stupid<'a>((hat, r, g, b): StupidInput<'a>) -> (StupidInput<'a>, Result<()>) {
    let res = hat.set_backlight_rgb(r, g, b);
    ((hat, r, g, b), res)
}
