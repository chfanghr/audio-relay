mod driver;

use std::{thread, time::Duration};

use anyhow::Result;
use log::info;

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
                    for bn in color_channel_range.clone() {
                        _ = hat.set_backlight_rgb(r, g, b);
                        _ = hat.set_backlight_brightness(bn);
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        }
    }
}
