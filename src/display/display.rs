//! Display type

use tokio::io::AsyncWrite;

use super::area;
use super::commands::{self, DrawCommand, DrawHandle, SGR};


/// Representation of a display
///
/// Instances of this type represent the output component of a (possibly remote)
/// terminal.
///
pub struct Display<W: AsyncWrite + Send + Unpin> {
    write: W,
    rows: u16,
    cols: u16,
    termination_seq: [DrawCommand<'static>; 3],
}

impl<W: AsyncWrite + Send + Unpin> Display<W> {
    /// Create a new display
    ///
    /// Create a new display using the given writer, with the given geometry.
    ///
    /// The last two lines of the display will be reserved. For example, the
    /// second to last row will host the resting position.
    ///
    pub fn new(write: W, rows: u16, cols: u16) -> Self {
        let termination_seq = [
            SGR::Reset.into(),
            DrawCommand::SetPos(rows.saturating_sub(2), 0),
            DrawCommand::ShowCursor(true),
        ];
        Self {write, rows, cols, termination_seq}
    }

    /// Retrieve an area covering the non-reserved portion of the display
    ///
    /// The returned area will cover the entire width of the screen. However,
    /// it will only cover the rows from the top up to, and including, the third
    /// lowest row.
    ///
    /// Prior to returning the area, this function will clear the entire screen.
    ///
    pub async fn area(&mut self) -> std::io::Result<area::Area<'_, DrawHandle<'_, &mut W>, &mut W>> {
        use futures::SinkExt;

        use commands::SinkProxy;

        let rows = self.rows().saturating_sub(2);
        let cols = self.cols();

        let mut handle = self.handle().await?;
        handle.as_sink().send(DrawCommand::ClearScreen).await.map(|_| area::create_area(handle, rows, cols))
    }

    /// Retrieve a star handle for updating screen contents
    ///
    /// The returned handle may be used to update items placed on a previously
    /// retrieved area for the same display.
    ///
    pub async fn handle(&mut self) -> std::io::Result<DrawHandle<'_, &mut W>> {
        use futures::SinkExt;

        use commands::SinkProxy;

        let mut handle = commands::draw_handle(&mut self.write, self.termination_seq.as_ref());
        handle.as_sink().send(DrawCommand::ShowCursor(false)).await.map(|_| handle)
    }

    /// Retrieve the number of rows
    ///
    /// This includes the two reserved rows at the bottom of the display.
    ///
    pub fn rows(&self) -> u16 {
        self.rows
    }

    /// Retrieve the number of columns
    ///
    pub fn cols(&self) -> u16 {
        self.cols
    }
}

