use std::io::{Result, Write};
use tokio::sync::mpsc::Sender;

use crossterm::{
    cursor,
    event::{self, KeyCode, KeyEvent, KeyModifiers, NoTtyEvent, SenderWriter},
    queue,
    style::{Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType},
    Command,
};

use crate::{
    error::InquireResult,
    ui::{Attributes, InputReader, Key, Styled},
};

use super::Terminal;

pub struct CrosstermTerminal {
    sender: SenderWriter,
    event: NoTtyEvent,
}

pub struct CrosstermKeyReader {
    event: NoTtyEvent,
}

impl CrosstermKeyReader {
    pub fn new(event: NoTtyEvent) -> Self {
        Self { event }
    }
}

impl InputReader for CrosstermKeyReader {
    fn read_key(&mut self) -> InquireResult<Key> {
        loop {
            if let event::Event::Key(key_event) = event::read(&self.event)? {
                return Ok(key_event.into());
            }
        }
    }
}

impl CrosstermTerminal {
    pub fn new(sender: Sender<Vec<u8>>, event: NoTtyEvent) -> InquireResult<Self> {
        terminal::enable_raw_mode()?;

        Ok(Self {
            sender: SenderWriter::new(sender),
            event,
        })
    }

    fn get_writer(&mut self) -> &mut dyn Write {
        &mut self.sender
    }

    fn write_command<C: Command>(&mut self, command: C) -> Result<()> {
        queue!(&mut self.get_writer(), command)
    }

    fn set_attributes(&mut self, attributes: Attributes) -> Result<()> {
        if attributes.contains(Attributes::BOLD) {
            self.write_command(SetAttribute(Attribute::Bold))?;
        }
        if attributes.contains(Attributes::ITALIC) {
            self.write_command(SetAttribute(Attribute::Italic))?;
        }

        Ok(())
    }

    fn reset_attributes(&mut self) -> Result<()> {
        self.write_command(SetAttribute(Attribute::Reset))
    }

    fn set_fg_color(&mut self, color: crate::ui::Color) -> Result<()> {
        self.write_command(SetForegroundColor(color.into()))
    }

    fn reset_fg_color(&mut self) -> Result<()> {
        self.write_command(SetForegroundColor(Color::Reset))
    }

    fn set_bg_color(&mut self, color: crate::ui::Color) -> Result<()> {
        self.write_command(SetBackgroundColor(color.into()))
    }

    fn reset_bg_color(&mut self) -> Result<()> {
        self.write_command(SetBackgroundColor(Color::Reset))
    }
}

impl Terminal for CrosstermTerminal {
    fn cursor_up(&mut self, cnt: u16) -> Result<()> {
        match cnt {
            0 => Ok(()),
            cnt => self.write_command(cursor::MoveUp(cnt)),
        }
    }

    fn cursor_down(&mut self, cnt: u16) -> Result<()> {
        match cnt {
            0 => Ok(()),
            cnt => self.write_command(cursor::MoveDown(cnt)),
        }
    }

    fn cursor_left(&mut self, cnt: u16) -> Result<()> {
        match cnt {
            0 => Ok(()),
            cnt => self.write_command(cursor::MoveLeft(cnt)),
        }
    }

    fn cursor_right(&mut self, cnt: u16) -> Result<()> {
        match cnt {
            0 => Ok(()),
            cnt => self.write_command(cursor::MoveRight(cnt)),
        }
    }

    fn cursor_move_to_column(&mut self, idx: u16) -> Result<()> {
        self.write_command(cursor::MoveToColumn(idx))
    }

    fn flush(&mut self) -> Result<()> {
        self.get_writer().flush()
    }

    fn get_size(&self) -> Result<super::TerminalSize> {
        terminal::size(&self.event).map(|(width, height)| super::TerminalSize::new(width, height))
    }

    fn write<T: std::fmt::Display>(&mut self, val: T) -> Result<()> {
        self.write_command(Print(val))
    }

    fn write_styled<T: std::fmt::Display>(&mut self, val: &Styled<T>) -> Result<()> {
        if let Some(color) = val.style.fg {
            self.set_fg_color(color)?;
        }
        if let Some(color) = val.style.bg {
            self.set_bg_color(color)?;
        }
        if !val.style.att.is_empty() {
            self.set_attributes(val.style.att)?;
        }

        self.write(&val.content)?;

        if val.style.fg.as_ref().is_some() {
            self.reset_fg_color()?;
        }
        if val.style.bg.as_ref().is_some() {
            self.reset_bg_color()?;
        }
        if !val.style.att.is_empty() {
            self.reset_attributes()?;
        }

        Ok(())
    }

    fn clear_line(&mut self) -> Result<()> {
        self.write_command(terminal::Clear(ClearType::CurrentLine))
    }

    fn clear_until_new_line(&mut self) -> Result<()> {
        self.write_command(terminal::Clear(ClearType::UntilNewLine))
    }

    fn cursor_hide(&mut self) -> Result<()> {
        self.write_command(cursor::Hide)
    }

    fn cursor_show(&mut self) -> Result<()> {
        self.write_command(cursor::Show)
    }
}

impl Drop for CrosstermTerminal {
    fn drop(&mut self) {
        let _unused = self.flush();
        let _unused = terminal::disable_raw_mode();
    }
}

impl From<crate::ui::Color> for Color {
    fn from(c: crate::ui::Color) -> Self {
        use crate::ui::Color as C;
        match c {
            C::Black => Color::Black,
            C::LightRed => Color::Red,
            C::DarkRed => Color::DarkRed,
            C::LightGreen => Color::Green,
            C::DarkGreen => Color::DarkGreen,
            C::LightYellow => Color::Yellow,
            C::DarkYellow => Color::DarkYellow,
            C::LightBlue => Color::Blue,
            C::DarkBlue => Color::DarkBlue,
            C::LightMagenta => Color::Magenta,
            C::DarkMagenta => Color::DarkMagenta,
            C::LightCyan => Color::Cyan,
            C::DarkCyan => Color::DarkCyan,
            C::White => Color::White,
            C::Grey => Color::Grey,
            C::DarkGrey => Color::DarkGrey,
            C::Rgb { r, g, b } => Color::Rgb { r, g, b },
            C::AnsiValue(b) => Color::AnsiValue(b),
        }
    }
}

impl From<KeyModifiers> for crate::ui::KeyModifiers {
    fn from(m: KeyModifiers) -> Self {
        let mut modifiers = Self::empty();

        if m.contains(KeyModifiers::NONE) {
            modifiers |= crate::ui::KeyModifiers::NONE;
        }
        if m.contains(KeyModifiers::ALT) {
            modifiers |= crate::ui::KeyModifiers::ALT;
        }
        if m.contains(KeyModifiers::CONTROL) {
            modifiers |= crate::ui::KeyModifiers::CONTROL;
        }
        if m.contains(KeyModifiers::SHIFT) {
            modifiers |= crate::ui::KeyModifiers::SHIFT;
        }
        if m.contains(KeyModifiers::SUPER) {
            modifiers |= crate::ui::KeyModifiers::SUPER;
        }
        if m.contains(KeyModifiers::HYPER) {
            modifiers |= crate::ui::KeyModifiers::HYPER;
        }
        if m.contains(KeyModifiers::META) {
            modifiers |= crate::ui::KeyModifiers::META;
        }

        modifiers
    }
}

impl From<KeyEvent> for Key {
    fn from(event: KeyEvent) -> Self {
        match event {
            KeyEvent {
                code: KeyCode::Esc, ..
            } => Self::Escape,
            KeyEvent {
                code: KeyCode::Enter | KeyCode::Char('\n' | '\r'),
                ..
            } => Self::Enter,
            KeyEvent {
                code: KeyCode::Tab | KeyCode::Char('\t'),
                ..
            } => Self::Tab,
            KeyEvent {
                code: KeyCode::Backspace,
                ..
            } => Self::Backspace,
            KeyEvent {
                code: KeyCode::Delete,
                modifiers: m,
                ..
            } => Self::Delete(m.into()),
            KeyEvent {
                code: KeyCode::Home,
                ..
            } => Self::Home,
            KeyEvent {
                code: KeyCode::End, ..
            } => Self::End,
            KeyEvent {
                code: KeyCode::PageUp,
                modifiers: m,
                ..
            } => Self::PageUp(m.into()),
            KeyEvent {
                code: KeyCode::PageDown,
                modifiers: m,
                ..
            } => Self::PageDown(m.into()),
            KeyEvent {
                code: KeyCode::Up,
                modifiers: m,
                ..
            } => Self::Up(m.into()),
            KeyEvent {
                code: KeyCode::Down,
                modifiers: m,
                ..
            } => Self::Down(m.into()),
            KeyEvent {
                code: KeyCode::Left,
                modifiers: m,
                ..
            } => Self::Left(m.into()),
            KeyEvent {
                code: KeyCode::Right,
                modifiers: m,
                ..
            } => Self::Right(m.into()),
            KeyEvent {
                code: KeyCode::Char(c),
                modifiers: m,
                ..
            } => Self::Char(c, m.into()),
            #[allow(deprecated)]
            _ => Self::Any,
        }
    }
}
