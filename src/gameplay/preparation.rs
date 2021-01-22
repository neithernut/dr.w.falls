//! Field preparation utilities

use crate::util;


/// Prepare a random distribution of coloured tiles
///
/// This function returns an iterator over positions and colours intended for
/// initializing a field with viruses. The iterator will yield at most
/// `number_of_virs` entries, all with positions in rows from the given
/// `top_row` to the bottom row. The positions and colours as well as their
/// ordering will be random.
///
/// The returned positions and colours will not contain horizontal or vertical
/// configurations of for or more tiles of the same colour.
///
pub fn prepare_field(
    rng: &mut impl rand_core::RngCore,
    top_row: util::RowIndex,
    number_of_virs: u8,
) -> impl Iterator<Item = (util::Position, util::Colour)> + '_ {
    // We'll use a field of `Option<Colour>` detecting rows of our
    let mut field: PreparationField = Default::default();

    let rows = util::RangeInclusive::new(top_row, util::RowIndex::BOTTOM_ROW);
    let area = rows.len() * (util::FIELD_WIDTH as usize);

    (0..number_of_virs).filter_map(move |virus_count| {
        let random = rng.next_u32();

        // Select an unoccupied position
        let unfilled = (area - (virus_count as usize)) as u32;
        let pos = rows
            .clone()
            .flat_map(util::complete_row)
            .filter(|p| field[*p].is_none())
            .take((random % unfilled) as usize)
            .last();

        // Select a colour. If colouring the tile would result in a row of four,
        // we'll select another colour through rotation. Since we only have two
        // dimensitons but three colours, we are guranteed to reach a solution.
        let random = random / unfilled;
        let colour_shift = random % 2 + 1;
        let mut random = random / 2;
        pos.map(|pos| loop {
            let colour = match random % 3 {
                0 => util::Colour::Red,
                1 => util::Colour::Yellow,
                _ => util::Colour::Blue,
            };
            field[pos] = Some(colour);
            if super::items::row_of_four(&field, pos).is_none() {
                break (pos, colour)
            }
            random = random + colour_shift;
        })
    })
}


/// Field of `Option<Colour>`
///
#[derive(Default)]
struct PreparationField {
    data: [super::row::Row<Option<util::Colour>>; util::FIELD_HEIGHT as usize],
}

impl std::ops::IndexMut<util::Position> for PreparationField {
    fn index_mut(&mut self, index: util::Position) -> &mut Self::Output {
        &mut self.data[usize::from(index.0)][index.1]
    }
}

impl std::ops::Index<util::Position> for PreparationField {
    type Output = Option<util::Colour>;

    fn index(&self, index: util::Position) -> &Self::Output {
        &self.data[usize::from(index.0)][index.1]
    }
}

