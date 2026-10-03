use anyhow::{Result, anyhow};
use embedded_hal::digital::InputPin;
use std::{
    thread,
    time::{Duration, Instant},
};

#[derive(Debug)]
struct Debounce {
    t: Instant,
    reading: bool,
}

#[derive(Debug)]
pub struct Button<P> {
    p: P,
    d: Option<Debounce>,
}

const DEBOUNCE_THRESHOLD: Duration = Duration::from_millis(500);

impl<P> Button<P> {
    pub fn new(p: P) -> Self {
        Self { p, d: None }
    }
}

impl<P: InputPin> Button<P> {
    pub fn block_until_low(&mut self) -> Result<Option<Duration>> {
        let begin = Instant::now();
        if self.read()? {
            loop {
                thread::sleep(DEBOUNCE_THRESHOLD);
                if !self.read()? {
                    break Ok(Some(begin.elapsed()));
                }
            }
        } else {
            Ok(None)
        }
    }

    pub fn read(&mut self) -> Result<bool> {
        match &self.d {
            Some(d) => {
                if d.t.elapsed() < DEBOUNCE_THRESHOLD {
                    Ok(d.reading)
                } else {
                    self.force_read_and_upadate_debounce()
                }
            }
            None => self.force_read_and_upadate_debounce(),
        }
    }

    fn force_read_and_upadate_debounce(&mut self) -> Result<bool> {
        let reading = self
            .p
            .is_high()
            .map_err(|e| anyhow!("unable to read gpio: {:?}", e))?;
        self.d = Some(Debounce {
            t: Instant::now(),
            reading,
        });
        Ok(reading)
    }
}
