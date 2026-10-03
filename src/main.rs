mod driver;

use anyhow::Result;

use crate::driver::{button, hat};

fn main() -> Result<()> {
    let mut hat = hat::Driver::new_rpi_4b()?;
    hat.init()?;

    btn_test("select", &mut hat.select_button)?;
    btn_test("up", &mut hat.up_button)?;
    btn_test("down", &mut hat.down_button)?;
    btn_test("left", &mut hat.left_button)?;
    btn_test("right", &mut hat.right_button)?;

    Ok(())
}

fn btn_test(lbl: &str, btn: &mut button::Driver<linux_embedded_hal::CdevPin>) -> Result<()> {
    loop {
        let m_dur = btn.block_until_low()?;
        if let Some(dur) = m_dur {
            println!("{} btn pushed: {}", lbl, dur.as_millis());
            break Ok(());
        }
    }
}
