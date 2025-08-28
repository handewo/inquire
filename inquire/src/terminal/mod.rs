use std::{fmt::Display, io::Result};

use crate::{
    error::InquireResult,
    ui::{dimension::Dimension, InputReader, Styled},
};

#[cfg(all(feature = "crossterm", not(feature = "no-tty")))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "crossterm", not(feature = "no-tty")))))]
pub mod crossterm;

#[cfg(all(feature = "crossterm", feature = "no-tty"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "crossterm", feature = "no-tty"))))]
pub mod no_tty;

#[cfg(feature = "termion")]
#[cfg_attr(docsrs, doc(cfg(feature = "termion")))]
pub mod termion;

#[cfg(feature = "console")]
#[cfg_attr(docsrs, doc(cfg(feature = "console")))]
pub mod console;

#[cfg(test)]
pub(crate) mod test;

pub type TerminalSize = Dimension;

pub trait Terminal: Sized {
    fn get_size(&self) -> Result<TerminalSize>;

    fn write<T: Display>(&mut self, val: T) -> Result<()>;
    fn write_styled<T: Display>(&mut self, val: &Styled<T>) -> Result<()>;

    fn clear_line(&mut self) -> Result<()>;
    fn clear_until_new_line(&mut self) -> Result<()>;

    fn cursor_hide(&mut self) -> Result<()>;
    fn cursor_show(&mut self) -> Result<()>;
    fn cursor_up(&mut self, cnt: u16) -> Result<()>;
    fn cursor_down(&mut self, cnt: u16) -> Result<()>;
    fn cursor_left(&mut self, cnt: u16) -> Result<()>;
    fn cursor_right(&mut self, cnt: u16) -> Result<()>;
    #[allow(unused)]
    fn cursor_move_to_column(&mut self, idx: u16) -> Result<()>;

    fn flush(&mut self) -> Result<()>;
}

pub fn get_default_terminal(
    #[cfg(feature = "no-tty")] event: crossterm::event::NoTtyEvent,
    #[cfg(feature = "no-tty")] sender: tokio::sync::mpsc::Sender<Vec<u8>>,
) -> InquireResult<(impl InputReader, impl Terminal)> {
    #[cfg(all(feature = "crossterm", not(feature = "no-tty")))]
    return Ok((
        crossterm::CrosstermKeyReader::new(),
        crossterm::CrosstermTerminal::new()?,
    ));

    #[cfg(all(feature = "crossterm", feature = "no-tty"))]
    return Ok((
        no_tty::CrosstermKeyReader::new(event.clone()),
        no_tty::CrosstermTerminal::new(sender, event)?,
    ));

    #[cfg(all(feature = "termion", not(feature = "crossterm")))]
    return Ok((
        termion::TermionKeyReader::new()?,
        termion::TermionTerminal::new()?,
    ));

    #[cfg(all(
        feature = "console",
        not(feature = "termion"),
        not(feature = "crossterm")
    ))]
    {
        let console_terminal = console::ConsoleTerminal::new();
        let console_key_reader = console_terminal.clone();
        return Ok((console_key_reader, console_terminal));
    }

    #[cfg(all(
        not(feature = "crossterm"),
        not(feature = "termion"),
        not(feature = "console")
    ))]
    {
        compile_error!("At least one of crossterm, termion or console must be enabled");

        // this is here to silence an additional compilation error
        // when no terminals are enabled. it complains about mismatched
        // return types.
        Err(crate::error::InquireError::InvalidConfiguration(
            "Missing terminal backend".into(),
        ))
    }
}
