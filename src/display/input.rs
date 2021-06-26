//! Line input entity

use std::num::NonZeroU16;

use tokio::io::AsyncWrite;

use super::area;
use super::commands::{self, DrawCommand as DC, DrawHandle, SGR};


/// Representation of a line input field
///
/// An instance of this type itself is useless unless it is placed in an `Area`.
///
pub struct LineInput {
    max_length: NonZeroU16,
}

impl LineInput {
    /// Create a line input field with the given width
    ///
    /// The line input will accept at most `max_length` characters.
    ///
    pub fn new(max_length: NonZeroU16) -> Self {
        Self {max_length}
    }
}

impl area::Entity for LineInput {
    type PlacedEntity = InputUpdater;

    fn rows(&self) -> u16 {
        1
    }

    fn cols(&self) -> u16 {
        self.max_length.get()
    }

    fn init(&self, pos: (u16, u16)) -> area::PlacedInit {
        vec![pos.into(), SGR::Blink(true).into(), "_".into()].into()
    }

    fn place(self, (base_row, base_col): (u16, u16)) -> Self::PlacedEntity {
        InputUpdater {base_row, base_col, max_length: self.max_length, value: Default::default()}
    }
}


/// Handle for updating a line input field
///
pub struct InputUpdater {
    base_row: u16,
    base_col: u16,
    max_length: NonZeroU16,
    value: String,
}

impl InputUpdater {
    /// Update the field with a given input character
    ///
    /// The function will update both the internal value and the representation
    /// from the given `input`. Only non-control ASCII characters are accepted
    /// into the value. However, a backspace character will remove the last
    /// character from the value.
    ///
    /// A new line (`0x0A`) or carriage return (`0x0D`) will cause the function
    /// to return the current value. Otherwise, the returned result will contain
    /// only `None` on success.
    ///
    pub async fn update(
        &mut self,
        draw_handle: &mut DrawHandle<'_, impl AsyncWrite + Unpin>,
        input: char,
    ) -> std::io::Result<Option<&str>> {
        use futures::SinkExt;
        use futures::stream::iter;

        use commands::SinkProxy;

        match input {
            '\x0A' | '\x0D' => return Ok(Some(self.value.as_ref())),
            '\x08' => {
                self.value.pop();
                let len = self.value.len() as u16;
                let cmds = [
                    DC::SetPos(self.base_row, self.base_col + len),
                    SGR::Blink(true).into(),
                    "_".into(),
                ];
                let cmds = cmds
                    .iter()
                    .cloned()
                    .chain(if len + 1 < self.max_length.get() { Some(" ".into()) } else { None })
                    .map(Ok);
                draw_handle.as_sink().send_all(&mut iter(cmds)).await?
            },
            c if c.is_ascii() && !c.is_control() => {
                let old_len = self.value.len() as u16;
                let max_len = self.max_length.get();
                if old_len < max_len {
                    self.value.push(c);
                    let cmds = [
                        DC::SetPos(self.base_row, self.base_col + old_len),
                        String::from(c).into(),
                        SGR::Blink(true).into(),
                    ];
                    let cmds = cmds
                        .iter()
                        .cloned()
                        .chain(if self.value.len() < max_len.into() { Some("_".into()) } else { None })
                        .map(Ok);
                    draw_handle.as_sink().send_all(&mut iter(cmds)).await?
                }
            },
            _ => (),
        }

        Ok(None)
    }

    /// Clear the input field
    ///
    /// This function clears both the internal value and its display. The caller
    /// receives the previous value via the return value.
    ///
    pub async fn clear(
        &mut self,
        draw_handle: &mut DrawHandle<'_, impl AsyncWrite + Unpin>,
    ) -> std::io::Result<String> {

        use futures::SinkExt;
        use futures::stream::iter;

        use commands::SinkProxy;

        let cmds = [
            DC::SetPos(self.base_row, self.base_col),
            SGR::Blink(true).into(),
            "_".into(),
            SGR::Blink(false).into(),
        ];
        let cmds = cmds
            .iter()
            .cloned()
            .chain(std::iter::repeat(" ".into())
            .take(self.max_length.get().into()))
            .map(Ok);

        draw_handle.as_sink().send_all(&mut iter(cmds)).await.map(|_| std::mem::take(&mut self.value))
    }

    /// Retrieve the current value
    ///
    pub fn value(&self) -> &str {
        self.value.as_ref()
    }
}

