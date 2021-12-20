//! Play field entity

use futures::SinkExt;
use tokio::io::AsyncWrite;

use crate::util;
use super::area;
use super::commands::{Colour, DrawCommand as DC, DrawHandle, SinkProxy};


/// Representation of a play field entity
///
/// An instance of this type itself is useless unless it is placed in an `Area`.
///
#[derive(Default)]
pub struct PlayField;

impl PlayField {
    /// Create a new play field
    ///
    pub fn new() -> Self {
        Self {}
    }
}

impl area::Entity for PlayField {
    type PlacedEntity = FieldUpdater;

    fn rows(&self) -> u16 {
        util::FIELD_HEIGHT as u16 + 3
    }

    fn cols(&self) -> u16 {
        2 * util::FIELD_WIDTH as u16 + 2
    }

    fn init(&self, (base_row, base_col): (u16, u16)) -> area::PlacedInit {
        let inlet = "/    \\";
        let inlet_col = util::FIELD_WIDTH as u16 - (inlet.len() as u16 / 2);

        // Upper part of inlet
        let mut res = vec![
            DC::SetPos(base_row, base_col + 1 + inlet_col),
            "\\    /".into(),
        ];

        let left_wall = base_col;
        let right_wall = base_col + 1 + 2*(util::FIELD_WIDTH as u16);

        let element_top_row = base_row + 2;

        // Bottle ceiling, with lower part of inlet
        res.push(DC::SetPos(base_row + 1, base_col + 1));
        res.extend((0..inlet_col).map(|_| "_".into()));
        res.push(inlet.into());
        res.extend(((inlet_col + inlet.len() as u16)..(2 * util::FIELD_WIDTH as u16)).map(|_| "_".into()));
        res.push(DC::SetPos(element_top_row, base_col));
        res.push("/".into());
        res.push(DC::SetPos(element_top_row, right_wall));
        res.push("\\".into());

        // Left and right wall
        (1..util::FIELD_HEIGHT.into())
            .map(|row| row + element_top_row)
            .for_each(|row| res.extend([
                DC::SetPos(row, left_wall),
                "|".into(),
                DC::SetPos(row, right_wall),
                "|".into(),
            ].iter().cloned()));

        // Bottle floor
        res.push(DC::SetPos(base_row + 2 + util::FIELD_HEIGHT as u16, base_col));
        res.push("\\".into());
        res.extend((0..util::FIELD_WIDTH).map(|_| "__".into()));
        res.push("/".into());

        res.into()
    }

    fn place(self, (base_row, base_col): (u16, u16)) -> Self::PlacedEntity {
        FieldUpdater {base_row, base_col}
    }
}


/// Handle for updating the play field entity
///
pub struct FieldUpdater {
    base_row: u16,
    base_col: u16,
}

impl FieldUpdater {
    /// Place viruses in the field
    ///
    /// For each of the items in `viruses`, one virus will be placed in the
    /// field, at the given position and with the given colour.
    ///
    pub async fn place_viruses(
        &self,
        draw_handle: &mut DrawHandle<'_, impl AsyncWrite + Send + Unpin>,
        viruses: impl IntoIterator<Item=(util::Position, util::Colour)>,
        vir_sym: VirusSym,
    ) -> std::io::Result<()> {
        use std::iter::once;

        use futures::stream::iter;

        let cmds: Vec<_> = viruses.into_iter().flat_map(|(pos, col)|
            once(self.transform(pos))
                .chain(once(Colour::from(col).into()))
                .chain(once(vir_sym.symbol().into()))
        ).map(Ok).collect();
        draw_handle.as_sink().send_all(&mut iter(cmds)).await
    }

    /// Place the next capsule elements in the appropriate position
    ///
    pub async fn place_next_elements(
        &self,
        draw_handle: &mut DrawHandle<'_, impl AsyncWrite + Send + Unpin>,
        capsule: &[util::Colour; 2],
    ) -> std::io::Result<()> {
        let row = self.base_row + 1;
        let col = self.base_col + 1 + util::FIELD_WIDTH as u16 - 2;

        let sink = draw_handle.as_sink();

        sink.feed(DC::SetPos(row, col)).await?;
        sink.feed(Colour::from(capsule[0]).into()).await?;
        sink.feed("()".into()).await?;
        sink.feed(Colour::from(capsule[1]).into()).await?;
        sink.feed("()".into()).await
    }

    /// Process field updates
    ///
    /// Each item in `updates` will be processed in order: if the update carries
    /// a colour, a capsule element of the given colour will be placed at the
    /// given position. Otherwise, any element at the given position will be
    /// erased.
    ///
    pub async fn update(
        &self,
        draw_handle: &mut DrawHandle<'_, impl AsyncWrite + Send + Unpin>,
        updates: impl IntoIterator<Item=crate::field::Update>,
    ) -> std::io::Result<()> {
        use std::iter::once;

        use futures::stream::iter;

        let cmds: Vec<_> = updates.into_iter().flat_map(move |(pos, col)| {
            let sym = if col.is_some() {
                "()"
            } else {
                "  "
            };
            once(self.transform(pos)).chain(col.map(|c| Colour::from(c).into())).chain(once(sym.into()))
        }).map(Ok).collect();

        draw_handle.as_sink().send_all(&mut iter(cmds)).await
    }

    /// Transform field positions to display positions
    ///
    fn transform<'t>(&self, (row, col): util::Position) -> DC<'static> {
        DC::SetPos(
            self.base_row + 2 + usize::from(row) as u16,
            self.base_col + 1 + 2 * usize::from(col) as u16,
        )
    }
}


/// Helper for graphical representations of viruses
///
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum VirusSym {A, B}

impl VirusSym {
    /// Return this virus representation flipped
    ///
    pub fn flipped(&self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }

    /// Return this graphical representation as a str
    ///
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::A => "><",
            Self::B => "--",
        }
    }
}

impl Default for VirusSym {
    fn default() -> Self {
        Self::A
    }
}

