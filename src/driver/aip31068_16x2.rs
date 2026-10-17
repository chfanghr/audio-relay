use std::{iter, thread, time::Duration};

use anyhow::{Context, Ok, Result, anyhow, ensure};
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

    fn send_commands(&mut self, cmds: &[Command]) -> Result<()> {
        let bytes_to_send = iter::once(0x00u8)
            .chain(cmds.iter().map(|c| c.to_byte()))
            .collect::<Vec<u8>>();
        self.i2c.write(self.addr, &bytes_to_send).map_err(|err| {
            anyhow!("failed to send commands: {:?}", err)
                .context(anyhow!("bytes_to_send: {:?}", bytes_to_send))
                .context(anyhow!("cmds: {:?}", cmds))
        })?;
        thread::sleep(Duration::from_millis(5));
        Ok(())
    }

    /// - Set two lines mode
    /// - 5x8 font
    /// - Clear display
    /// - Right to left without display shift
    /// - Enable cursor, blinking
    /// - Set cursor to row 0, column 0
    pub fn init(&mut self) -> Result<()> {
        let commands = [
            Command::FunctionSet {
                lines: Lines::TwoLines,
                font: Font::FiveByEight,
            },
            Command::DisplayControl {
                display: OnOff::Off,
                cursor: OnOff::Off,
                cursor_blink: OnOff::Off,
            },
            Command::ClearDisplay,
            Command::EntryModeSet {
                direction: Direction::Right,
                display_shift: DisplayShift::Disabled,
            },
            Command::DisplayControl {
                display: OnOff::On,
                cursor: OnOff::On,
                cursor_blink: OnOff::On,
            },
            Command::SetDdramAddress(0x00),
        ];

        for command in commands {
            self.send_commands(&[command])
                .context("unable to initialize display")?;
        }

        Ok(())
    }

    pub fn clear_display(&mut self) -> Result<()> {
        self.send_commands(&[Command::ClearDisplay])
    }

    pub fn move_cursor(&mut self, row: u8, col: u8) -> Result<()> {
        ensure!(row < ROWS, "{} exceeds maximum rows {}", row, ROWS);
        ensure!(col < COLS, "{} exceeds maximum cols {}", col, COLS);

        let row_to_offset: [u8; ROWS as usize] = [0x00, 0x40];
        let addr = row_to_offset[row as usize] | col;

        self.send_commands(&[Command::SetDdramAddress(addr)])
    }

    pub fn write_ascii_text(&mut self, s: &str) -> Result<()> {
        ensure!(s.is_ascii(), "{} is not a ascii string", s);
        ensure!(
            s.len() < COLS as usize,
            "string len {} exceeds maximum cols {}",
            s.len(),
            COLS
        );
        let bytes_to_send = iter::once(0x40u8)
            .chain(s.as_bytes().iter().copied())
            .collect::<Vec<u8>>();
        self.i2c
            .write(self.addr, &bytes_to_send)
            .map_err(|err| anyhow!("unable to write ascii text: {:?}", err))
            .context(anyhow!("bytes_to_send: {:?}", bytes_to_send))?;
        thread::sleep(Duration::from_millis(5));
        Ok(())
    }
}
