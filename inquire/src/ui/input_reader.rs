use crate::error::InquireResult;

use super::Key;

pub trait InputReader: Sized {
    /// Reads the next key event from the input.
    ///
    /// Under the `no-tty` feature, input arrives from an async channel
    /// (see [`crossterm::event::NoTtyEvent`]), so this method is `async`.
    #[cfg(not(feature = "no-tty"))]
    fn read_key(&mut self) -> InquireResult<Key>;

    /// Reads the next key event from the async input channel.
    ///
    /// The returned future is `Send` so prompts can be driven from within
    /// spawned tasks (e.g. an SSH session handler).
    #[cfg(feature = "no-tty")]
    fn read_key(&mut self) -> impl std::future::Future<Output = InquireResult<Key>> + Send;
}
