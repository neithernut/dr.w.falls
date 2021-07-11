//! Dynamic text entity

use std::num::NonZeroU16;

use tokio::io::AsyncWrite;

use super::area;
use super::commands::{self, DrawCommand as DC, DrawHandle};


/// Representation of a field for arbitrary text to display
///
/// An instance of this type itself is useless unless it is placed in an `Area`.
///
pub struct DynamicText {
    rows: NonZeroU16,
    cols: NonZeroU16,
}

impl DynamicText {
    /// Create a new text field covering the given number of columns and rows
    ///
    pub fn new(rows: NonZeroU16, cols: NonZeroU16) -> Self {
        Self {rows, cols}
    }

    /// Create a new text field covering a single line with the given width
    ///
    pub fn new_line(cols: NonZeroU16) -> Self {
        Self {rows: unsafe { NonZeroU16::new_unchecked(1) }, cols}
    }
}

impl area::Entity for DynamicText {
    type PlacedEntity = TextUpdater;

    fn rows(&self) -> u16 {
        self.rows.get()
    }

    fn cols(&self) -> u16 {
        self.cols.get()
    }

    fn init(&self, _: (u16, u16)) -> area::PlacedInit {
        Vec::new().into()
    }

    fn place(self, (base_row, base_col): (u16, u16)) -> Self::PlacedEntity {
        TextUpdater {base_row, base_col, rows: self.rows, cols: self.cols}
    }
}


/// Handle for updating a specific text field
///
pub struct TextUpdater {
    base_row: u16,
    base_col: u16,
    rows: NonZeroU16,
    cols: NonZeroU16,
}

impl TextUpdater {
    /// Clear the entire field
    ///
    pub async fn clear(
        &self,
        draw_handle: &mut DrawHandle<'_, impl AsyncWrite + Unpin>,
    ) -> std::io::Result<()> {
        self.update(draw_handle, std::iter::empty::<&'static str>()).await
    }

    /// Update the text field with the given contents
    ///
    /// The given lines will be put in the text field's top tows. Any lines for
    /// which no content was supplied will be cleared.
    ///
    /// A line must not contain any control characters. In particular, it must
    /// not contain `'\r'` or `'\n'`.
    ///
    pub async fn update(
        &self,
        draw_handle: &mut DrawHandle<'_, impl AsyncWrite + Unpin>,
        lines: impl IntoIterator<Item = impl std::fmt::Display>,
    ) -> std::io::Result<()> {
        use std::iter::once;

        use futures::SinkExt;
        use futures::stream::iter;

        use commands::SinkProxy;

        let mut rows = self.row_pos();

        let cmds = rows
            .by_ref()
            .zip(lines)
            .flat_map(|(p, l)| once(p).chain(once(format!("{0:^1$}", l, self.cols.get() as usize).into())))
            .map(Ok);
        draw_handle.as_sink().send_all(&mut iter(cmds)).await?;

        let cmds = rows.flat_map(|p| once(p).chain(self.empty_row())).map(Ok);
        draw_handle.as_sink().send_all(&mut iter(cmds)).await
    }

    /// Update the text field with the given single row content
    ///
    /// The given contents will be placed in the text field's top row. Any
    /// remaining rows are cleared.
    ///
    /// The line must not contain any control characters. In particular, it must
    /// not contain `'\r'` or `'\n'`.
    ///
    pub async fn update_single(
        &self,
        draw_handle: &mut DrawHandle<'_, impl AsyncWrite + Unpin>,
        line: impl std::fmt::Display,
    ) -> std::io::Result<()> {
        self.update(draw_handle, std::iter::once(line)).await
    }

    /// Generate draw commands setting the cursor to rows' starting position
    ///
    /// The returned iterator will yield `SetPos` draw commands with coordniates
    /// of the text field rows' starting positions. The positions are yielded
    /// ordered from the top to bottom row.
    ///
    fn row_pos(&self) -> impl Iterator<Item = DC> {
        let base_row = self.base_row;
        let base_col = self.base_col;
        (0..self.rows.get()).map(move |r| DC::SetPos(r + base_row, base_col))
    }

    /// Generate draw commands for filling a row with space characters
    ///
    fn empty_row(&self) -> impl Iterator<Item = DC> {
        std::iter::repeat(" ".into()).take(self.cols.get() as usize)
    }
}

