//! Display tests

use std::num::NonZeroU8;
use std::pin::Pin;
use std::sync::Arc;
use std::task;

use quickcheck::{Arbitrary, Gen, TestResult};

use super::*;


#[quickcheck]
fn score_board_initial_content(
    rows: u8,
    cols: u8,
    base_row: u8,
    base_col: u8,
    orig: Vec<ScoreBoardEntry>,
    max_rows: u8,
    show_scores: bool,
) -> std::io::Result<TestResult> {
    use area::Entity;
    use scores::Entry;

    let rows: u16 = rows.into();
    let cols: u16 = cols.into();
    let base_row: u16 = base_row.into();
    let base_col: u16 = base_col.into();

    let board = scores::ScoreBoard::new(max_rows.into()).show_scores(show_scores);
    let area = Area {
        row_a: base_row,
        col_a: base_col,
        row_b: base_row.saturating_add(board.rows()),
        col_b: base_col.saturating_add(board.cols()),
    };

    if area.row_b <= rows && area.col_b <= cols && orig.iter().all(ScoreBoardEntry::acceptable_for_tests) {
        tokio::runtime::Runtime::new()?.block_on(async {
            let (writer, vt_state) = tokio::sync::watch::channel(VT::new(rows, cols));
            let mut handle = handle_from_bare(VTWriter::from(writer), &[]).await;
            area.instantiate(&mut handle)
                .place_center(board)
                .await?
                .update(&mut handle, orig.iter(), |_| false)
                .await?;
            let state = ((area.row_a + 1)..area.row_b)
                .map(|r| vt_state.borrow().chars_at(r, area.col_a).collect::<String>())
                .filter_map(|r| {
                    let mut parts = r.split_whitespace();
                    let num: usize = parts.next()?.parse().ok()?;
                    let name: String = parts.next()?.to_owned();
                    let total_score: Option<u32> = parts.next().and_then(|s| s.parse().ok());
                    let round_score: Option<u32> = parts.next().and_then(|s| s.parse().ok());
                    Some((num, name, total_score, round_score))
                });
            let res = orig
                .iter()
                .take(max_rows.into())
                .enumerate()
                .map(|(n, e)| {
                    let (total_score, round_score) = if show_scores {
                        (Some(e.tag().score()), Some(e.round_score()))
                    } else {
                        (None, None)
                    };
                    (n + 1, e.tag().name().trim().to_owned(), total_score, round_score)
                })
                .eq(state);
            Ok(TestResult::from_bool(res))
        })
    } else {
        Ok(TestResult::discard())
    }
}


#[quickcheck]
fn score_board_updated_content(
    rows: u8,
    cols: u8,
    base_row: u8,
    base_col: u8,
    orig1: Vec<ScoreBoardEntry>,
    orig2: Vec<ScoreBoardEntry>,
    max_rows: u8,
    show_scores: bool,
) -> std::io::Result<TestResult> {
    use area::Entity;
    use scores::Entry;

    let rows: u16 = rows.into();
    let cols: u16 = cols.into();
    let base_row: u16 = base_row.into();
    let base_col: u16 = base_col.into();

    let board = scores::ScoreBoard::new(max_rows.into()).show_scores(show_scores);
    let area = Area {
        row_a: base_row,
        col_a: base_col,
        row_b: base_row.saturating_add(board.rows()),
        col_b: base_col.saturating_add(board.cols()),
    };

    let acceptable = orig1.iter().all(ScoreBoardEntry::acceptable_for_tests) &&
        orig2.iter().all(ScoreBoardEntry::acceptable_for_tests);
    if area.row_b <= rows && area.col_b <= cols && acceptable {
        tokio::runtime::Runtime::new()?.block_on(async {
            let (writer, vt_state) = tokio::sync::watch::channel(VT::new(rows, cols));
            let mut handle = handle_from_bare(VTWriter::from(writer), &[]).await;
            let mut board = area.instantiate(&mut handle).place_center(board).await?;
            board.update(&mut handle, orig1.iter(), |_| false).await?;
            board.update(&mut handle, orig2.iter(), |_| false).await?;
            let state = ((area.row_a + 1)..area.row_b)
                .map(|r| vt_state.borrow().chars_at(r, area.col_a).collect::<String>())
                .filter_map(|r| {
                    let mut parts = r.split_whitespace();
                    let num: usize = parts.next()?.parse().ok()?;
                    let name: String = parts.next()?.to_owned();
                    let total_score: Option<u32> = parts.next().and_then(|s| s.parse().ok());
                    let round_score: Option<u32> = parts.next().and_then(|s| s.parse().ok());
                    Some((num, name, total_score, round_score))
                });
            let res = orig2
                .iter()
                .take(max_rows.into())
                .enumerate()
                .map(|(n, e)| {
                    let (total_score, round_score) = if show_scores {
                        (Some(e.tag().score()), Some(e.round_score()))
                    } else {
                        (None, None)
                    };
                    (n + 1, e.tag().name().trim().to_owned(), total_score, round_score)
                })
                .eq(state);
            Ok(TestResult::from_bool(res))
        })
    } else {
        Ok(TestResult::discard())
    }
}


#[quickcheck]
fn dynamic_text(
    rows: NonZeroU8,
    cols: NonZeroU8,
    mut area: Area,
    mut orig: Vec<crate::tests::ASCIIString>,
) -> std::io::Result<TestResult> {
    use std::convert::TryInto;

    let rows = rows.get().into();
    let cols = cols.get().into();

    area.constrain(rows, cols);
    if !area.is_empty() {
        // make the text fit into the area
        orig.truncate(area.rows() as usize);
        orig.iter_mut().for_each(|r| r.0.truncate(area.cols() as usize));

        tokio::runtime::Runtime::new()?.block_on(async {
            let (writer, vt_state) = tokio::sync::watch::channel(VT::new(rows, cols));
            let mut handle = handle_from_bare(VTWriter::from(writer), &[]).await;
            let text = area.instantiate(&mut handle).place_center(
                dynamic_text::DynamicText::new(
                    area.rows().try_into().unwrap(),
                    area.cols().try_into().unwrap(),
                ),
            ).await?;
            text.update(&mut handle, orig.iter()).await?;
            let state: Vec<_> = (area.row_a..area.row_b)
                .take(orig.len())
                .map(|r| vt_state.borrow().chars_at(r, area.col_a).collect::<String>())
                .collect();
            let res = Iterator::eq(orig.iter().map(|r| r.0.trim()), state.iter().map(|r| r.trim()));
            Ok(TestResult::from_bool(res))
        })
    } else {
        Ok(TestResult::discard())
    }
}


#[quickcheck]
fn play_field_init(rows: u8, cols: u8, base_row: u8, base_col: u8) -> std::io::Result<TestResult> {
    use area::Entity;

    let rows: u16 = rows.into();
    let cols: u16 = cols.into();
    let base_row: u16 = base_row.into();
    let base_col: u16 = base_col.into();

    let field = field::PlayField::new();
    let area = Area {
        row_a: base_row,
        col_a: base_col,
        row_b: base_row.saturating_add(field.rows()),
        col_b: base_col.saturating_add(field.cols()),
    };

    if area.row_b <= rows && area.col_b <= cols {
        tokio::runtime::Runtime::new()?.block_on(async {
            let (writer, vt_state) = tokio::sync::watch::channel(VT::new(rows, cols));
            area.instantiate(handle_from_bare(VTWriter::from(writer), &[]).await)
                .place_center(field)
                .await?;
            let res = (area.row_a..area.row_b)
                .map(|r| vt_state
                    .borrow()
                    .chars_at(r, area.col_a)
                    .take(area.cols().into())
                    .collect::<String>()
                )
                .eq(vec![
                    "      \\    /      ",
                    " _____/    \\_____ ",
                    "/                \\",
                    "|                |",
                    "|                |",
                    "|                |",
                    "|                |",
                    "|                |",
                    "|                |",
                    "|                |",
                    "|                |",
                    "|                |",
                    "|                |",
                    "|                |",
                    "|                |",
                    "|                |",
                    "|                |",
                    "|                |",
                    "\\________________/",
                ]);
            Ok(TestResult::from_bool(res))
        })
    } else {
        Ok(TestResult::discard())
    }
}


#[quickcheck]
fn play_field_virs(
    rows: u8,
    cols: u8,
    base_row: u8,
    base_col: u8,
    viruses: std::collections::HashMap<crate::util::Position, crate::util::Colour>,
    vir_sym: field::VirusSym,
) -> std::io::Result<TestResult> {
    use std::convert::TryInto;

    use area::Entity;

    let rows: u16 = rows.into();
    let cols: u16 = cols.into();
    let base_row: u16 = base_row.into();
    let base_col: u16 = base_col.into();

    let field = field::PlayField::new();
    let area = Area {
        row_a: base_row,
        col_a: base_col,
        row_b: base_row.saturating_add(field.rows()),
        col_b: base_col.saturating_add(field.cols()),
    };

    if area.row_b <= rows && area.col_b <= cols {
        tokio::runtime::Runtime::new()?.block_on(async {
            let (writer, vt_state) = tokio::sync::watch::channel(VT::new(rows, cols));
            let mut handle = handle_from_bare(VTWriter::from(writer), &[]).await;
            area.instantiate(&mut handle)
                .place_center(field)
                .await?
                .place_viruses(&mut handle, viruses.clone(), vir_sym)
                .await?;

            // Read back viruses into a map of positions
            let tiles = tile_contents(&vt_state.borrow(), area);

            let correct_syms = tiles
                .values()
                .all(|[a, b]| vir_sym.symbol().chars().eq([a.data as char, b.data as char]) &&
                    a.format == b.format
                );
            let virus_match = viruses == tiles
                .into_iter()
                .filter_map(|(p, [a, ..])| a.format.fg_colour.and_then(|(c, _)| c.try_into().ok()).map(|c| (p, c)))
                .collect();
            Ok(TestResult::from_bool(correct_syms && virus_match))
        })
    } else {
        Ok(TestResult::discard())
    }
}


#[quickcheck]
fn play_field_next(
    rows: u8,
    cols: u8,
    base_row: u8,
    base_col: u8,
    colour_a: crate::util::Colour,
    colour_b: crate::util::Colour,
) -> std::io::Result<TestResult> {
    use area::Entity;

    let rows: u16 = rows.into();
    let cols: u16 = cols.into();
    let base_row: u16 = base_row.into();
    let base_col: u16 = base_col.into();

    let field = field::PlayField::new();
    let area = Area {
        row_a: base_row,
        col_a: base_col,
        row_b: base_row.saturating_add(field.rows()),
        col_b: base_col.saturating_add(field.cols()),
    };

    if area.row_b <= rows && area.col_b <= cols {
        tokio::runtime::Runtime::new()?.block_on(async {
            let (writer, vt_state) = tokio::sync::watch::channel(VT::new(rows, cols));
            let mut handle = handle_from_bare(VTWriter::from(writer), &[]).await;
            area.instantiate(&mut handle)
                .place_center(field)
                .await?
                .place_next_elements(&mut handle, &[colour_a, colour_b])
                .await?;

            let res = vt_state
                .borrow()
                .chars_at(area.row_a + 1, area.col_a)
                .take(area.cols().into())
                .eq(" _____/()()\\_____ ".chars());
            Ok(TestResult::from_bool(res))
        })
    } else {
        Ok(TestResult::discard())
    }
}


#[quickcheck]
fn play_field_update(
    rows: u8,
    cols: u8,
    base_row: u8,
    base_col: u8,
    updates: Vec<crate::field::Update>,
) -> std::io::Result<TestResult> {
    use std::convert::TryInto;

    use area::Entity;

    let rows: u16 = rows.into();
    let cols: u16 = cols.into();
    let base_row: u16 = base_row.into();
    let base_col: u16 = base_col.into();

    let field = field::PlayField::new();
    let area = Area {
        row_a: base_row,
        col_a: base_col,
        row_b: base_row.saturating_add(field.rows()),
        col_b: base_col.saturating_add(field.cols()),
    };

    if area.row_b <= rows && area.col_b <= cols {
        tokio::runtime::Runtime::new()?.block_on(async {
            let (writer, vt_state) = tokio::sync::watch::channel(VT::new(rows, cols));
            let mut handle = handle_from_bare(VTWriter::from(writer), &[]).await;
            area.instantiate(&mut handle)
                .place_center(field)
                .await?
                .update(&mut handle, updates.clone())
                .await?;

            let elements: std::collections::HashMap<_, _> = updates
                .into_iter()
                .fold(Default::default(), |mut a, (p, c)| {
                    if let Some(c) = c {
                        a.insert(p, c);
                    } else {
                        a.remove(&p);
                    }
                    a
                });

            let tiles = tile_contents(&vt_state.borrow(), area);

            let correct_syms = tiles
                .values()
                .all(|[a, b]| a.data == 0x28 && b.data == 0x29 && a.format == b.format);
            let element_match = elements == tiles
                .into_iter()
                .filter_map(|(p, [a, ..])| a.format.fg_colour.and_then(|(c, _)| c.try_into().ok()).map(|c| (p, c)))
                .collect();
            Ok(TestResult::from_bool(correct_syms && element_match))
        })
    } else {
        Ok(TestResult::discard())
    }
}


#[quickcheck]
fn line_input_update(
    rows: u8,
    cols: u8,
    base_row: u8,
    base_col: u8,
    input_len: NonZeroU8,
    inputs: Vec<u8>,
) -> std::io::Result<TestResult> {
    use area::Entity;

    let rows: u16 = rows.into();
    let cols: u16 = cols.into();
    let base_row: u16 = base_row.into();
    let base_col: u16 = base_col.into();

    let line_input = input::LineInput::new(input_len.into());
    let area = Area {
        row_a: base_row,
        col_a: base_col,
        row_b: base_row.saturating_add(line_input.rows()),
        col_b: base_col.saturating_add(line_input.cols()),
    };

    if area.row_b <= rows && area.col_b <= cols {
        tokio::runtime::Runtime::new()?.block_on(async {
            let (writer, vt_state) = tokio::sync::watch::channel(VT::new(rows, cols));
            let mut handle = handle_from_bare(VTWriter::from(writer), &[]).await;
            let mut placed = area.instantiate(&mut handle).place_center(line_input).await?;

            for i in inputs.iter() {
                let is_commit = placed.update(&mut handle, *i as char).await?.is_some();

                let val: String = vt_state.borrow().chars_at(area.row_a, area.col_a).collect();
                let val = val.trim_end();

                let internal_len = placed.value().len();

                let failure = (is_commit && (*i != 0x0A && *i != 0x0D)) ||
                    val.len() > area.cols() as usize ||
                    internal_len > area.cols() as usize ||
                    !val.starts_with(placed.value().trim_end()) ||
                    (internal_len < area.cols() as usize &&
                        val.chars().nth(internal_len).map(|c| c != '_').unwrap_or(true));
                if failure {
                    return Ok(TestResult::failed())
                }
            }

            Ok(TestResult::passed())
        })
    } else {
        Ok(TestResult::discard())
    }
}


#[quickcheck]
fn display_handle_init(rows: NonZeroU8, cols: NonZeroU8) -> std::io::Result<bool> {
    let rows = rows.get().into();
    let cols = cols.get().into();
    tokio::runtime::Runtime::new()?.block_on(async {
        let (writer, mut vt_state) = tokio::sync::watch::channel(VT::new(rows, cols));
        let mut display = display::Display::new(VTWriter::from(writer), rows, cols);
        let h = display.handle().await?;
        let res = vt_state.borrow_and_update().show_cursor == false;
        drop(h);
        Ok(res)
    })
}


#[quickcheck]
fn display_handle_drop(
    rows: NonZeroU8,
    cols: NonZeroU8,
    commands: Vec<commands::DrawCommand<'static>>,
) -> std::io::Result<TestResult> {
    use futures::SinkExt;

    use crate::display::commands::SinkProxy;

    let rows = rows.get().into();
    let cols = cols.get().into();
    tokio::runtime::Runtime::new()?.block_on(async {
        let (writer, mut vt_state) = tokio::sync::watch::channel(VT::new(rows, cols));
        let mut display = display::Display::new(VTWriter::from(writer), rows, cols);
        let mut handle = display.handle().await?;
        let res = handle
            .as_sink()
            .send_all(&mut futures::stream::iter(commands.iter().cloned().map(Ok)))
            .await;
        if res.is_ok() {
            vt_state.borrow_and_update();
            drop(handle);
            vt_state.changed().await.map_err(|_| std::io::ErrorKind::Other)?;
            let state = vt_state.borrow();
            let res = state.show_cursor &&
                state.cursor_row == rows.saturating_sub(2) &&
                state.cursor_col == 0;
            Ok(TestResult::from_bool(res))
        } else {
            Ok(TestResult::discard())
        }
    })
}


#[quickcheck]
fn area_split_top(area: Area, split_rows: u16) -> std::io::Result<bool> {
    Ok(tokio::runtime::Runtime::new()?.block_on(async {
        let mut area = area.instantiate(handle_from_bare(tokio::io::sink(), &[]).await);
        let rows = area.rows();

        let sub = area.split_top(split_rows);
        let sub_rows = sub.rows();
        let sub_cols = sub.cols();

        sub_rows == std::cmp::min(rows, split_rows) &&
            area.rows() + sub_rows == rows &&
            area.cols() == sub_cols
    }))
}


#[quickcheck]
fn area_split_left(area: Area, split_cols: u16) -> std::io::Result<bool> {
    Ok(tokio::runtime::Runtime::new()?.block_on(async {
        let mut area = area.instantiate(handle_from_bare(tokio::io::sink(), &[]).await);
        let cols = area.cols();

        let sub = area.split_left(split_cols);
        let sub_rows = sub.rows();
        let sub_cols = sub.cols();

        sub_cols == std::cmp::min(cols, split_cols) &&
            area.cols() + sub_cols == cols &&
            area.rows() == sub_rows
    }))
}


#[quickcheck]
fn area_pad_top(area: Area, padding: u16) -> std::io::Result<bool> {
    Ok(tokio::runtime::Runtime::new()?.block_on(async {
        let area = area.instantiate(handle_from_bare(tokio::io::sink(), &[]).await);
        let rows = area.rows();
        let cols = area.cols();

        let area = area.pad_top(padding);

        area.rows() == rows.saturating_sub(padding) && cols == area.cols()
    }))
}


#[quickcheck]
fn area_pad_bottom(area: Area, padding: u16) -> std::io::Result<bool> {
    Ok(tokio::runtime::Runtime::new()?.block_on(async {
        let area = area.instantiate(handle_from_bare(tokio::io::sink(), &[]).await);
        let rows = area.rows();
        let cols = area.cols();

        let area = area.pad_bottom(padding);

        area.rows() == rows.saturating_sub(padding) && cols == area.cols()
    }))
}


#[quickcheck]
fn area_pad_left(area: Area, padding: u16) -> std::io::Result<bool> {
    Ok(tokio::runtime::Runtime::new()?.block_on(async {
        let area = area.instantiate(handle_from_bare(tokio::io::sink(), &[]).await);
        let rows = area.rows();
        let cols = area.cols();

        let area = area.pad_left(padding);

        area.cols() == cols.saturating_sub(padding) && rows == area.rows()
    }))
}


#[quickcheck]
fn area_pad_right(area: Area, padding: u16) -> std::io::Result<bool> {
    Ok(tokio::runtime::Runtime::new()?.block_on(async {
        let area = area.instantiate(handle_from_bare(tokio::io::sink(), &[]).await);
        let rows = area.rows();
        let cols = area.cols();

        let area = area.pad_right(padding);

        area.cols() == cols.saturating_sub(padding) && rows == area.rows()
    }))
}


#[quickcheck]
fn area_place_top(area: Area, entity: DummyEntity) -> std::io::Result<bool> {
    let res = tokio::runtime::Runtime::new()?.block_on(async {
        let mut area = area.instantiate(handle_from_bare(tokio::io::sink(), &[]).await);
        area.place_top(entity).await.map(|p| (p, area.rows(), area.cols()))
    });

    match res {
        Ok((placed, new_rows, new_cols)) => Ok(
            placed.base_row == area.row_a &&
            placed.base_col >= area.col_a &&
            placed.rows + new_rows == area.rows() &&
            placed.cols <= area.cols() &&
            new_cols == area.cols()
        ),
        Err(_) => Ok(entity.rows > area.rows() || entity.cols > area.cols()),
    }
}


#[quickcheck]
fn area_place_left(area: Area, entity: DummyEntity) -> std::io::Result<bool> {
    let res = tokio::runtime::Runtime::new()?.block_on(async {
        let mut area = area.instantiate(handle_from_bare(tokio::io::sink(), &[]).await);
        area.place_left(entity).await.map(|p| (p, area.rows(), area.cols()))
    });

    match res {
        Ok((placed, new_rows, new_cols)) => Ok(
            placed.base_row >= area.row_a &&
            placed.base_col == area.col_a &&
            placed.rows <= area.rows() &&
            placed.cols + new_cols == area.cols() &&
            new_rows == area.rows()
        ),
        Err(_) => Ok(entity.rows > area.rows() || entity.cols > area.cols()),
    }
}


#[quickcheck]
fn area_place_center(area: Area, entity: DummyEntity) -> std::io::Result<bool> {
    let res = tokio::runtime::Runtime::new()?.block_on(async {
        area.instantiate(handle_from_bare(tokio::io::sink(), &[]).await).place_center(entity).await
    });

    match res {
        Ok(placed) => Ok(
            placed.base_row >= area.row_a &&
            placed.base_col >= area.col_a &&
            placed.rows <= area.rows() &&
            placed.cols <= area.cols()
        ),
        Err(_) => Ok(entity.rows > area.rows() || entity.cols > area.cols()),
    }
}


#[quickcheck]
fn draw_handle_drop(
    mut data: Vec<commands::DrawCommand<'static>>,
    term: Vec<commands::DrawCommand<'static>>,
) -> std::io::Result<TestResult> {
    use futures::SinkExt;

    use commands::{DrawCommand as DC, SinkProxy};

    let rt = tokio::runtime::Runtime::new()?;

    let inner: Arc<tokio::sync::Mutex<_>> = Arc::new(
        tokio_util::codec::FramedWrite::new(Vec::new(), commands::ANSIEncoder::new()).into()
    );

    rt.block_on(async {
        let mut handle = commands::draw_handle(inner.clone().lock_owned().await, term.as_ref());
        let res = handle.as_sink().send_all(&mut futures::stream::iter(data.iter().cloned().map(Ok))).await;
        drop(handle);
        res
    })?;

    data.extend(term);
    if data.windows(2).any(|w| if let [DC::Text(_), DC::Text(_)] = w { true } else { false }) {
        Ok(TestResult::discard())
    } else {
        let buf = inner.blocking_lock();
        let res = draw_commands_from(buf.get_ref().as_ref())
            .try_fold(Vec::new(), |mut a, c| { a.push(c?); Ok(a) })
            .map(|r| TestResult::from_bool(data == r));
        res
    }
}


#[quickcheck]
fn ansi_encode_decode(orig: Vec<commands::DrawCommand<'static>>) -> std::io::Result<TestResult> {
    use futures::SinkExt;

    use commands::DrawCommand as DC;

    if orig.windows(2).any(|w| if let [DC::Text(_), DC::Text(_)] = w { true } else { false }) {
        return Ok(TestResult::discard())
    }

    let rt = tokio::runtime::Runtime::new()?;

    let mut buf = Vec::new();

    let mut write = tokio_util::codec::FramedWrite::new(&mut buf, super::commands::ANSIEncoder::new());
    rt.block_on(write.send_all(&mut futures::stream::iter(orig.iter().cloned().map(Ok))))?;

    let res = draw_commands_from(buf.as_ref())
        .try_fold(Vec::new(), |mut a, c| { a.push(c?); Ok(a) })
        .map(|r| TestResult::from_bool(orig == r));
    res
}


/// Utility function for retrieving all non-empty tiles in a field from a VT
///
/// This function retrieves the tile contents of a play field given via its
/// placement `area` on the given `vt`
///
fn tile_contents(
    vt: &VT,
    area: Area,
) -> std::collections::HashMap<crate::util::Position, [FormattedChar; 2]> {
    use std::convert::TryFrom;

    use crate::util;

    let base_row = area.row_a + 2;
    (base_row..(area.row_b - 1)).flat_map(|r| vt
        .data[r as usize]
        .split_at((area.col_a + 1).into())
        .1
        .chunks_exact(2)
        .enumerate()
        .filter_map(|(c, v)| if let [a, b] = v { Some((c, [*a, *b])) } else { None })
        .filter(|(_, [a, b])| a.data != 0x20 || b.data != 0x20)
        .filter_map(move |(c, s)| Some(((
            util::RowIndex::try_from((r - base_row) as usize).ok()?,
            util::ColumnIndex::try_from(c as usize).ok()?,
        ), s)))
    ).collect()
}


/// A mock up score board entry
///
#[derive(Clone, Debug)]
struct ScoreBoardEntry {
    pub tag: crate::player::Tag,
    pub round_score: u32,
    pub active: bool,
}

impl ScoreBoardEntry {
    /// Check whether the entry is usable for (relatively simple) tests
    ///
    pub fn acceptable_for_tests(&self) -> bool {
        !self.tag.name().trim().is_empty() &&
            self.tag.score() < 10000000 &&
            self.round_score < 10000000
    }
}

impl scores::Entry for ScoreBoardEntry {
    fn tag(&self) -> &crate::player::Tag {
        &self.tag
    }

    fn round_score(&self) -> u32 {
        self.round_score
    }

    fn active(&self) -> bool {
        self.active
    }
}

impl Arbitrary for ScoreBoardEntry {
    fn arbitrary(g: &mut Gen) -> Self {
        Self {
            tag: Arbitrary::arbitrary(g),
            round_score: Arbitrary::arbitrary(g),
            active: Arbitrary::arbitrary(g),
        }
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let res = (self.tag.clone(), self.round_score, self.active)
            .shrink()
            .map(|(tag, round_score, active)| Self {tag, round_score, active});
        Box::new(res)
    }
}


/// Utility for generating random [area::Area]s
///
#[derive(Copy, Clone, Debug)]
struct Area {
    row_a: u16,
    col_a: u16,
    row_b: u16,
    col_b: u16,
}

impl Area {
    pub fn instantiate<H: std::borrow::BorrowMut<DrawHandle<'static, W>>, W: tokio::io::AsyncWrite + Send + Unpin>(
        self,
        handle: H,
    ) -> area::Area<'static, H, W> {
        area::create_area_full(handle, self.row_a, self.col_a, self.row_b, self.col_b)
    }

    pub fn rows(&self) -> u16 {
        self.row_b - self.row_a
    }

    pub fn cols(&self) -> u16 {
        self.col_b - self.col_a
    }

    pub fn is_empty(&self) -> bool {
        self.rows() <= 0 || self.cols() <= 0
    }

    pub fn constrain(&mut self, rows: u16, cols: u16) {
        self.row_b = std::cmp::min(self.row_b, rows);
        self.col_b = std::cmp::min(self.col_b, cols);
        self.row_a = std::cmp::min(self.row_a, self.row_b);
        self.col_a = std::cmp::min(self.col_a, self.col_b);
    }
}

impl Arbitrary for Area {
    fn arbitrary(g: &mut Gen) -> Self {
        let row_x = Arbitrary::arbitrary(g);
        let row_y = Arbitrary::arbitrary(g);
        let col_x = Arbitrary::arbitrary(g);
        let col_y = Arbitrary::arbitrary(g);

        Self {
            row_a: std::cmp::min(row_x, row_y),
            col_a: std::cmp::min(col_x, col_y),
            row_b: std::cmp::max(row_x, row_y),
            col_b: std::cmp::max(col_x, col_y),
        }
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let res = (self.row_a, self.col_a, self.row_b, self.col_b)
            .shrink()
            .filter(|(row_a, col_a, row_b, col_b)| row_a <= row_b && col_a <= col_b)
            .map(|(row_a, col_a, row_b, col_b)| Self{row_a, col_a, row_b, col_b});
        Box::new(res)
    }
}


/// Dummy [area::Entity] for testing entity placement
///
#[derive(Copy, Clone, Debug)]
struct DummyEntity {
    rows: u16,
    cols: u16,
}

impl area::Entity for DummyEntity {
    type PlacedEntity = DummyPlaced;

    fn rows(&self) -> u16 {
        self.rows
    }

    fn cols(&self) -> u16 {
        self.cols
    }

    fn init(&self, _pos: (u16, u16)) -> area::PlacedInit {
        Vec::new().into()
    }

    fn place(self, pos: (u16, u16)) -> Self::PlacedEntity {
        let (base_row, base_col) = pos;
        DummyPlaced {base_row, base_col, rows: self.rows, cols: self.cols}
    }
}

impl Arbitrary for DummyEntity {
    fn arbitrary(g: &mut Gen) -> Self {
        Self {rows: Arbitrary::arbitrary(g), cols: Arbitrary::arbitrary(g)}
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new((self.rows, self.cols).shrink().map(|(rows, cols)| Self{rows, cols}))
    }
}


struct DummyPlaced {
    pub base_row: u16,
    pub base_col: u16,
    pub rows: u16,
    pub cols: u16,
}


/// [AsyncWrite] modelling an VT
///
pub struct VTWriter(tokio::sync::watch::Sender<VT>);

impl From<tokio::sync::watch::Sender<VT>> for VTWriter {
    fn from(sender: tokio::sync::watch::Sender<VT>) -> Self {
        Self(sender)
    }
}

impl tokio::io::AsyncWrite for VTWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut task::Context<'_>,
        buf: &[u8],
    ) -> task::Poll<std::io::Result<usize>> {
        let mut current: VT = self.0.borrow().clone();
        draw_commands_from(buf).try_for_each(|c| current.apply(c?))?;
        self.0.send(current).map_err(|_| std::io::Error::from(std::io::ErrorKind::Other))?;
        Ok(buf.len()).into()
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut task::Context<'_>,
    ) -> task::Poll<std::io::Result<()>> {
        Ok(()).into()
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut task::Context<'_>,
    ) -> task::Poll<std::io::Result<()>> {
        Ok(()).into()
    }
}


/// Simplified model of a virtual terminal
///
#[derive(Clone, Debug, PartialEq)]
pub struct VT {
    cursor_row: u16,
    cursor_col: u16,
    rendition: GraphicRendition,
    show_cursor: bool,
    data: Vec<Vec<FormattedChar>>,
}

impl VT {
    /// Create a new VT with the given number of rows and columns
    ///
    pub fn new(rows: u16, cols: u16) -> Self {
        let mut row = Vec::new();
        row.resize(cols as usize, Default::default());

        let mut res: Self = Default::default();
        res.data.resize(rows as usize, row);
        res
    }

    /// Clear the "screen"
    ///
    pub fn clear(&mut self) {
        self.data.iter_mut().for_each(|r| r.fill(Default::default()))
    }

    /// Apply a [commands::DrawCommand] to the VT
    ///
    pub fn apply(&mut self, command: commands::DrawCommand) -> std::io::Result<()> {
        use commands::DrawCommand as DC;

        match command {
            DC::ClearScreen     => Ok(self.clear()),
            DC::SetPos(r, c)    => if (r as usize) < self.data.len() && (c as usize) < self.data[0].len() {
                self.cursor_row = r;
                self.cursor_col = c;
                Ok(())
            } else {
                Err(std::io::ErrorKind::Other.into())
            },
            DC::Format(sgr)     => Ok(self.rendition.apply(sgr)),
            DC::Text(txt)       => txt.chars().try_for_each(|c| {
                self.data
                    .get_mut(self.cursor_row as usize)
                    .ok_or(std::io::ErrorKind::Other)?
                    .get_mut(self.cursor_col as usize)
                    .ok_or(std::io::ErrorKind::Other)?
                    .set_from_char(c, self.rendition)?;
                self.cursor_col = self.cursor_col.checked_add(1).ok_or(std::io::ErrorKind::Other)?;
                Ok(())
            }),
            DC::ShowCursor(v)   => Ok(self.show_cursor = v),
        }
    }

    /// View the data displayed at a given position as characters
    ///
    pub fn chars_at(&self, row: u16, col: u16) -> impl Iterator<Item = char> + '_ {
        self.data
            .get(row as usize)
            .map(|r| r.split_at(col as usize).1)
            .unwrap_or_default()
            .into_iter()
            .cloned()
            .map(Into::into)
    }
}

impl Default for VT {
    fn default() -> Self {
        Self {
            cursor_row: 0,
            cursor_col: 0,
            rendition: Default::default(),
            show_cursor: true,
            data: Default::default(),
        }
    }
}


/// Representation of a formatted character on a [VT]
///
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FormattedChar {
    pub data: u8,
    pub format: GraphicRendition,
}

impl FormattedChar {
    pub fn set_from_char(&mut self, data: char, format: GraphicRendition) -> std::io::Result<()> {
        if data.is_ascii_graphic() || data == '\x20' {
            self.data = data as u8;
            self.format = format;
            Ok(())
        } else {
            Err(std::io::ErrorKind::Other.into())
        }
    }
}

impl From<FormattedChar> for char {
    fn from(c: FormattedChar) -> Self {
        c.data.into()
    }
}

impl Default for FormattedChar {
    fn default() -> Self {
        Self {data: 0x20, format: Default::default()}
    }
}


/// Representation of a graphic rendition
///
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GraphicRendition {
    pub intensity: Option<commands::Intensity>,
    pub underline: bool,
    pub blink: bool,
    pub strike: bool,
    pub fg_colour: Option<(commands::Colour, commands::Brightness)>,
    pub bg_colour: Option<(commands::Colour, commands::Brightness)>,
}

impl GraphicRendition {
    /// Apply a change in the form of an SGR to this rendition
    ///
    pub fn apply(&mut self, sgr: commands::SGR) {
        use commands::SGR;

        match sgr {
            SGR::Reset          => *self = Default::default(),
            SGR::Intensity(v)   => self.intensity = v,
            SGR::Underline(v)   => self.underline = v,
            SGR::Blink(v)       => self.blink = v,
            SGR::Strike(v)      => self.strike = v,
            SGR::FGColour(v)    => self.fg_colour = v,
            SGR::BGColour(v)    => self.bg_colour = v,
        }
    }
}

impl Default for GraphicRendition {
    fn default() -> Self {
        Self {
            intensity: None,
            underline: false,
            blink: false,
            strike: false,
            fg_colour: None,
            bg_colour: None,
        }
    }
}


/// Create a DrawHandle from some bare parts
///
async fn handle_from_bare<'a, W: tokio::io::AsyncWrite + Send + Unpin + 'static>(
    write: W,
    termination_seq: &'a [commands::DrawCommand<'static>],
) -> DrawHandle<'a, W> {
    let inner: Arc<tokio::sync::Mutex<_>> = Arc::new(
        tokio_util::codec::FramedWrite::new(write, commands::ANSIEncoder::new()).into()
    );
    commands::draw_handle(inner.lock_owned().await, termination_seq)
}


/// Decode all `DrawCommand`s from a given input
///
fn draw_commands_from(mut src: &[u8]) -> impl Iterator<Item = std::io::Result<commands::DrawCommand<'static>>> + '_ {
    std::iter::from_fn(move || match decode_ansi(src) {
        Ok((res, rem))  => { src = rem; res.map(Ok) },
        Err(e)          => Some(Err(e))
    })
}


/// Decode a `DrawCommand`
///
/// Decode a single [commands::DrawCommand] from encoded ANSI provided as a
/// slice of bytes. The function resturns a tuple containing the decoded unit
/// and the remaining buffer. If the provided slice is empty, this function
/// returns `None` for the draw command. If an ANSI sequence could not be
/// decoded, an error will be returned.
///
fn decode_ansi(src: &[u8]) -> std::io::Result<(Option<commands::DrawCommand<'static>>, &[u8])> {
    use std::io::ErrorKind as EK;

    use commands::{Brightness, Colour, DrawCommand, Intensity, SGR};

    fn extract_num(s: &[u8]) -> Option<(&[u8], &[u8])> {
        s.iter().position(|c| !c.is_ascii_digit()).map(|p| s.split_at(p))
    }

    fn parse_u16(s: &[u8]) -> Option<u16> {
        std::str::from_utf8(s).ok().and_then(|s| s.parse().ok())
    }

    if src.is_empty() {
        Ok((None, src))
    } else if let Some(src) = src.strip_prefix(b"\x1b[") {
        let (n, rem) = extract_num(src).ok_or(EK::InvalidData)?;
        if !n.is_empty() {
            let n: u16 = parse_u16(n).ok_or(EK::InvalidData)?;
            let (com, rem) = rem.split_first().ok_or(EK::InvalidData)?;
            let data = match com {
                0x4a if n == 2  => DrawCommand::ClearScreen,
                0x3b            => {
                    let (m, rem) = extract_num(rem).ok_or(EK::InvalidData)?;
                    let m: u16 = parse_u16(m).ok_or(EK::InvalidData)?;
                    let (com, rem) = rem.split_first().ok_or(EK::InvalidData)?;
                    if *com == 0x48 {
                        let n = n.checked_sub(1).ok_or(EK::InvalidData)?;
                        let m = m.checked_sub(1).ok_or(EK::InvalidData)?;
                        return Ok((Some(DrawCommand::SetPos(n, m)), rem))
                    } else {
                        Err(EK::InvalidData)?
                    }
                },
                0x6d            => match n {
                      0 => SGR::Reset,
                      1 => SGR::Intensity(Some(Intensity::Bold)),
                      2 => SGR::Intensity(Some(Intensity::Faint)),
                      4 => SGR::Underline(true),
                      5 => SGR::Blink(true),
                      9 => SGR::Strike(true),
                     22 => SGR::Intensity(None),
                     24 => SGR::Underline(false),
                     25 => SGR::Blink(false),
                     29 => SGR::Strike(false),
                     30 => SGR::FGColour(Some((Colour::Black,   Brightness::Dark))),
                     31 => SGR::FGColour(Some((Colour::Red,     Brightness::Dark))),
                     32 => SGR::FGColour(Some((Colour::Green,   Brightness::Dark))),
                     33 => SGR::FGColour(Some((Colour::Yellow,  Brightness::Dark))),
                     34 => SGR::FGColour(Some((Colour::Blue,    Brightness::Dark))),
                     35 => SGR::FGColour(Some((Colour::Magenta, Brightness::Dark))),
                     36 => SGR::FGColour(Some((Colour::Cyan,    Brightness::Dark))),
                     37 => SGR::FGColour(Some((Colour::White,   Brightness::Dark))),
                     39 => SGR::FGColour(None),
                     40 => SGR::BGColour(Some((Colour::Black,   Brightness::Dark))),
                     41 => SGR::BGColour(Some((Colour::Red,     Brightness::Dark))),
                     42 => SGR::BGColour(Some((Colour::Green,   Brightness::Dark))),
                     43 => SGR::BGColour(Some((Colour::Yellow,  Brightness::Dark))),
                     44 => SGR::BGColour(Some((Colour::Blue,    Brightness::Dark))),
                     45 => SGR::BGColour(Some((Colour::Magenta, Brightness::Dark))),
                     46 => SGR::BGColour(Some((Colour::Cyan,    Brightness::Dark))),
                     47 => SGR::BGColour(Some((Colour::White,   Brightness::Dark))),
                     49 => SGR::BGColour(None),
                     90 => SGR::FGColour(Some((Colour::Black,   Brightness::Light))),
                     91 => SGR::FGColour(Some((Colour::Red,     Brightness::Light))),
                     92 => SGR::FGColour(Some((Colour::Green,   Brightness::Light))),
                     93 => SGR::FGColour(Some((Colour::Yellow,  Brightness::Light))),
                     94 => SGR::FGColour(Some((Colour::Blue,    Brightness::Light))),
                     95 => SGR::FGColour(Some((Colour::Magenta, Brightness::Light))),
                     96 => SGR::FGColour(Some((Colour::Cyan,    Brightness::Light))),
                     97 => SGR::FGColour(Some((Colour::White,   Brightness::Light))),
                    100 => SGR::BGColour(Some((Colour::Black,   Brightness::Light))),
                    101 => SGR::BGColour(Some((Colour::Red,     Brightness::Light))),
                    102 => SGR::BGColour(Some((Colour::Green,   Brightness::Light))),
                    103 => SGR::BGColour(Some((Colour::Yellow,  Brightness::Light))),
                    104 => SGR::BGColour(Some((Colour::Blue,    Brightness::Light))),
                    105 => SGR::BGColour(Some((Colour::Magenta, Brightness::Light))),
                    106 => SGR::BGColour(Some((Colour::Cyan,    Brightness::Light))),
                    107 => SGR::BGColour(Some((Colour::White,   Brightness::Light))),
                    _ => Err(EK::InvalidData)?
                }.into(),
                _ => Err(EK::InvalidData)?
            };
            Ok((Some(data), rem))
        } else {
            let (c, rem) = src.strip_prefix(b"?25").and_then(|s| s.split_first()).ok_or(EK::InvalidData)?;
            let show = match c {
                0x68    => true,
                0x6c    => false,
                _ => Err(EK::InvalidData)?
            };
            Ok((Some(DrawCommand::ShowCursor(show)), rem))
        }
    } else {
        let pos = src.iter().position(|c| *c == 0x1b).unwrap_or(src.len());
        let (data, rem) = src.split_at(pos);
        Ok((Some(String::from_utf8(data.to_vec()).map_err(|_| EK::InvalidData)?.into()), rem))
    }
}

