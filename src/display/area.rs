//! Screen area and related utilities

use std::borrow::BorrowMut;

use tokio::io::AsyncWrite;

use super::commands::{DrawCommand, DrawHandle};


/// Displayable entity
///
pub trait Entity {
    /// Type of the entity after placing
    ///
    /// In the most simple case, e.g. if only an initial state of the entity has
    /// to be drawn, this can be `()`.
    ///
    type PlacedEntity;

    /// Retriev ethe number of rows the entity covers
    ///
    fn rows(&self) -> u16;

    /// Retriev ethe number of columns the entity covers
    ///
    fn cols(&self) -> u16;

    /// Create an initialization of the placed entity
    ///
    /// The returned initialization contains instructions for drawing the
    /// entity's initial state. The function expects the first element of `pos`
    /// to contain the topmost row and the second to contain the leftmost
    /// column of the area reserved for the entity.
    ///
    fn init(&self, pos: (u16, u16)) -> PlacedInit;

    /// Place the entity
    ///
    /// This function places the entity on the given position and returns a
    /// value representing the entity after being placed. The function expects
    /// the first element of `pos` to contain the topmost row and the second to
    /// contain the leftmost column of the area reserved for the entity.
    ///
    fn place(self, pos: (u16, u16)) -> Self::PlacedEntity;
}


/// Instructions for drawing the entity's initial state
///
#[derive(Debug)]
pub struct PlacedInit<'a> {
    cmds: Vec<DrawCommand<'a>>,
}

impl<'a> From<Vec<DrawCommand<'a>>> for PlacedInit<'a> {
    fn from(cmds: Vec<DrawCommand<'a>>) -> Self {
        Self {cmds}
    }
}


/// Create an area
///
pub fn create_area<'a, W: AsyncWrite + Unpin>(
    handle: DrawHandle<'a, W>,
    rows: u16,
    cols: u16
) -> Area<'a, DrawHandle<'a, W>, W> {
    Area {handle, row_a: 0, col_a: 0, row_b: rows, col_b: cols, phantom: Default::default()}
}


/// Representation of an area
///
pub struct Area<'a, H, W>
where H: BorrowMut<DrawHandle<'a, W>>,
      W: AsyncWrite + Unpin,
{
    /// Handle used for drawing initial entites
    handle: H,
    /// First row of the area
    row_a: u16,
    /// First column of the area
    col_a: u16,
    /// First row outside the area
    row_b: u16,
    /// First column outside the area
    col_b: u16,
    phantom: std::marker::PhantomData<&'a W>
}

impl<'a, H, W> Area<'a, H, W>
where H: BorrowMut<DrawHandle<'a, W>>,
      W: AsyncWrite + Unpin,
{
    /// Retrieve the number of rows covered by the area
    ///
    pub fn rows(&self) -> u16 {
        self.row_b - self.row_a
    }

    /// Retrieve the number of columns covered by the area
    ///
    pub fn cols(&self) -> u16 {
        self.col_b - self.col_a
    }

    /// Split off rows from the top
    ///
    /// The returned sub-area will cover the given number of rows. Those rows
    /// will be taken from the area on which the function is called.
    ///
    pub fn split_top(&mut self, rows: u16) -> Area<'a, &'_ mut DrawHandle<'a, W>, W> {
        let row_a = self.row_a;
        let row_b = std::cmp::min(self.row_a.saturating_add(rows), self.row_b);
        self.row_a = std::cmp::min(row_b, self.row_b);

        Area {
            handle: self.handle.borrow_mut(),
            row_a,
            col_a: self.col_a,
            row_b,
            col_b: self.col_b,
            phantom: Default::default(),
        }
    }

    /// Split off columns from the left
    ///
    /// The returned sub-area will cover the given number of columns. Those
    /// columns will be taken from the area on which the function is called.
    ///
    pub fn split_left(&mut self, cols: u16) -> Area<'a, &'_ mut DrawHandle<'a, W>, W> {
        let col_a = self.col_a;
        let col_b = std::cmp::min(self.col_a.saturating_add(cols), self.col_b);
        self.col_a = std::cmp::min(col_b, self.col_b);

        Area {
            handle: self.handle.borrow_mut(),
            row_a: self.row_a,
            col_a,
            row_b: self.row_b,
            col_b,
            phantom: Default::default(),
        }
    }

    /// Remove the given number of rows from the top
    ///
    pub fn pad_top(self, rows: u16) -> Self {
        Self {row_a: std::cmp::min(self.row_a.saturating_add(rows), self.row_b), ..self}
    }

    /// Remove the given number of rows from the bottom
    ///
    pub fn pad_bottom(self, rows: u16) -> Self {
        Self {row_b: std::cmp::max(self.row_a, self.row_b.saturating_sub(rows)), ..self}
    }

    /// Remove the given number of columns from the left
    ///
    pub fn pad_left(self, cols: u16) -> Self {
        Self {col_a: std::cmp::min(self.col_a.saturating_add(cols), self.col_b), ..self}
    }

    /// Remove the given number of columns from the right
    ///
    pub fn pad_right(self, cols: u16) -> Self {
        Self {col_b: std::cmp::max(self.col_a, self.col_b.saturating_sub(cols)), ..self}
    }

    /// Place the given entity topmost inside the area
    ///
    /// The entity will be placed topmost inside the area, horizontally
    /// centered. The number of rows required by the entity will be removed,
    /// even in the case of an error.
    ///
    pub async fn place_top<E: Entity>(&mut self, entity: E) -> std::io::Result<E::PlacedEntity> {
        self.split_top(entity.rows()).place_center(entity).await
    }

    /// Place the given entity leftmost inside the area
    ///
    /// The entity will be placed leftmost inside the area, vertically centered.
    /// The number of columns required by the entity will be removed, even in
    /// the case of an error.
    ///
    pub async fn place_left<E: Entity>(&mut self, entity: E) -> std::io::Result<E::PlacedEntity> {
        self.split_left(entity.cols()).place_center(entity).await
    }

    /// Place the given entity in the area's center
    ///
    /// The entity will be centered both horizontally and vertically.
    ///
    pub async fn place_center<E: Entity>(mut self, entity: E) -> std::io::Result<E::PlacedEntity> {
        use std::io::ErrorKind;

        use futures::SinkExt;
        use futures::stream::iter;

        use super::commands::SinkProxy;

        let pos = (
            self.row_a + self.rows().checked_sub(entity.rows()).ok_or(ErrorKind::Other)? / 2,
            self.col_a + self.cols().checked_sub(entity.cols()).ok_or(ErrorKind::Other)? / 2,
        );

        let cmds: Vec<_> = entity.init(pos).cmds.into_iter().map(Ok).collect();
        self.handle.borrow_mut().as_sink().send_all(&mut iter(cmds)).await.map(|_| entity.place(pos))
    }
}

