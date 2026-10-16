use std::{thread, time::Duration};

use anyhow::{Ok, Result, anyhow};
use embedded_hal::i2c::I2c;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Font {
    FiveByEight,
    FiveByEleven,
}

impl Font {
    const fn bit(self) -> u8 {
        match self {
            Self::FiveByEight => 0,
            Self::FiveByEleven => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
}

impl Direction {
    const fn bit(self) -> u8 {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayShift {
    Disabled,
    Enabled,
}

impl DisplayShift {
    const fn bit(self) -> u8 {
        match self {
            Self::Disabled => 0,
            Self::Enabled => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftTarget {
    Cursor,
    Display,
}

impl ShiftTarget {
    const fn bit(self) -> u8 {
        match self {
            Self::Cursor => 0,
            Self::Display => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnOff {
    On,
    Off,
}

impl OnOff {
    const fn bit(self) -> u8 {
        match self {
            OnOff::On => 1,
            OnOff::Off => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lines {
    OneLine,
    TwoLines,
}

impl Lines {
    const fn bit(self) -> u8 {
        match self {
            Lines::OneLine => 0,
            Lines::TwoLines => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    ClearDisplay,
    ReturnHome,
    EntryModeSet {
        direction: Direction,
        display_shift: DisplayShift,
    },
    DisplayControl {
        display: OnOff,
        cursor: OnOff,
        cursor_blink: OnOff,
    },
    Shift {
        target: ShiftTarget,
        direction: Direction,
    },
    FunctionSet {
        lines: Lines,
        font: Font,
    },
    SetCgramAddress(u8),
    SetDdramAddress(u8),
}

impl Command {
    pub fn to_byte(self) -> u8 {
        match self {
            Command::ClearDisplay => 0x01,
            Command::ReturnHome => 0x02,
            Command::EntryModeSet {
                direction,
                display_shift,
            } => 0x04 | direction.bit() << 1 | display_shift.bit(),
            Command::DisplayControl {
                display,
                cursor,
                cursor_blink: blink,
            } => 0x08 | display.bit() << 2 | cursor.bit() << 1 | blink.bit(),
            Command::Shift { target, direction } => 0x10 | target.bit() << 3 | direction.bit() << 2,
            Command::FunctionSet { lines, font } => 0x20 | lines.bit() << 3 | font.bit() << 2,
            Command::SetCgramAddress(address) => 0x40 | address & 0x3f,
            Command::SetDdramAddress(address) => 0x80 | address & 0x7f,
        }
    }
}

const ROWS: u8 = 2;
const COLS: u8 = 16;

pub struct Driver<TDev> {
    addr: u8,
    i2c: TDev,
}

impl<TDev: I2c> Driver<TDev> {
    pub fn new(addr: u8, mut i2c: TDev) -> Result<Self> {
        // Sanity check: device presents?
        i2c.read(addr, &mut [0])
            .map_err(|err| anyhow!("aip31068 doesn't seem to present at {:#x}: {:?}", addr, err))?;
        Ok(Self { addr, i2c })
    }

    fn send_standalone_command(&mut self, cmd: Command) -> Result<()> {
        self.i2c
            .write(self.addr, &[0x00, cmd.to_byte()])
            .map_err(|err| anyhow!("failed to send command {:?}: {:?}", cmd, err))?;
        thread::sleep(Duration::from_millis(5));
        Ok(())
    }

    pub fn init(&mut self) -> Result<()> {
        assert_eq!(ROWS, 2); // we set two lines below
        self.send_standalone_command(Command::FunctionSet {
            lines: Lines::TwoLines,
            font: Font::FiveByEight,
        })?;
        // Turn off display
        self.send_standalone_command(Command::DisplayControl {
            display: OnOff::Off,
            cursor: OnOff::Off,
            cursor_blink: OnOff::Off,
        })?;
        self.send_standalone_command(Command::ClearDisplay)?;
        self.send_standalone_command(Command::EntryModeSet {
            direction: Direction::Right,
            display_shift: DisplayShift::Disabled,
        })?;
        self.send_standalone_command(Command::DisplayControl {
            display: OnOff::On,
            cursor: OnOff::On,
            cursor_blink: OnOff::On,
        })?;
        self.send_standalone_command(Command::SetDdramAddress(0x00))?;
        self.i2c
            .write(
                self.addr,
                &[
                    0x40, b'H', b'e', b'l', b'l', b'o', b' ', b'W', b'o', b'r', b'l', b'd',
                ],
            )
            .map_err(|err| anyhow!("failed to write: {:?}", err))?;
        Ok(())
    }
}
