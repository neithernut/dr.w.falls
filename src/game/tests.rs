//! Game tests

use quickcheck::TestResult;

use super::*;


#[quickcheck]
fn lobby_serve_instant_transition(
    input: crate::tests::ASCIIString,
    addr: std::net::SocketAddr,
) -> Result<bool, ConnTaskError> {
    use futures::StreamExt;

    tokio::runtime::Runtime::new()?.block_on(async {
        let (ports, _) = lobby::ports();
        let mut display = sink_display();
        let input = ascii_stream(input.as_ref()).chain(futures::stream::pending());
        let (_, phase) = tokio::sync::watch::channel(());
        lobby::serve(ports, &mut display, input, TransitionWatcher::new(phase, |_| true), addr.into())
            .await
            .map(|h| h.is_none())
    })
}


#[quickcheck]
fn lobby_serve_input_eof(
    input: crate::tests::ASCIIString,
    addr: std::net::SocketAddr,
) -> Result<bool, ConnTaskError> {
    tokio::runtime::Runtime::new()?.block_on(async {
        let (ports, _) = lobby::ports();
        let mut display = sink_display();
        let (phase_sender, phase) = tokio::sync::watch::channel(());
        let res = lobby::serve(
            ports,
            &mut display,
            ascii_stream(input.as_ref()),
            TransitionWatcher::new(phase, |_| false),
            addr.into(),
        ).await;
        drop(phase_sender);
        match res {
            Ok(_)                           => Ok(false),
            Err(ConnTaskError::Terminated)  => Ok(true),
            Err(e)                          => Err(e),
        }
    })
}


#[quickcheck]
fn lobby_serve_registration(
    orig: crate::player::tests::TestHandle,
    registrtion_success: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    use futures::StreamExt;

    let input = format!("{}\n", orig.name());

    tokio::runtime::Runtime::new()?.block_on(async {
        let (ports, mut control) = lobby::ports();
        let (phase_sender, phase) = tokio::sync::watch::channel(false);
        let orig_token: lobby::ConnectionToken = orig.addr().into();

        let lobby = {
            let orig_token = orig_token.clone();
            tokio::spawn(async move {
                let mut display = sink_display();
                lobby::serve(
                    ports,
                    &mut display,
                    ascii_stream(input.as_ref()).chain(futures::stream::pending()),
                    TransitionWatcher::new(phase, |t| *t),
                    orig_token.clone(),
                ).await
            })
        };

        let handle = if registrtion_success {
            Some(orig.clone().into())
        } else {
            None
        };
        let tag = handle.as_ref().map(crate::player::Handle::tag);

        let (name, token) = control
            .receive_registration(handle)
            .await
            .ok_or(crate::error::NoneError)?;
        phase_sender.send(true)?;
        let res = lobby.await??.map(|h| h.tag()) == tag &&
            name == orig.name() &&
            token == orig_token;
        Ok(res)
    })
}


#[tokio::test]
async fn waiting_serve_instant_transition() {
    let me = dummy_handle();

    let (ports, _) = waiting::ports(std::iter::once(me.tag()));
    let mut display = sink_display();
    let input = futures::stream::pending();
    let (_, phase) = tokio::sync::watch::channel(());
    waiting::serve(ports, &mut display, input, TransitionWatcher::new(phase, |_| true), &me)
        .await
        .expect("Waiting returned an error")
}


#[tokio::test]
async fn waiting_serve_input_eof() {
    let me = dummy_handle();

    let (ports, _) = waiting::ports(std::iter::once(me.tag()));
    let mut display = sink_display();
    let input = futures::stream::empty();
    let (phase_sender, phase) = tokio::sync::watch::channel(());
    let res = waiting::serve(ports, &mut display, input, TransitionWatcher::new(phase, |_| false), &me).await;
    drop(phase_sender);
    match res.unwrap_err() {
        ConnTaskError::Terminated => (),
        e => Err(e).expect("Expected ConnTaskError::Terminated"),
    }
}


#[quickcheck]
fn waiting_serve_ready(
    input: crate::tests::ASCIIString,
    me: crate::player::tests::TestHandle,
) -> Result<bool, Box<dyn std::error::Error>> {
    use futures::StreamExt;

    tokio::runtime::Runtime::new()?.block_on(async {
        let me: crate::player::Handle = me.into();
        let tag = me.tag();

        let (ports, mut control) = waiting::ports(std::iter::once(tag.clone()));
        let (phase_sender, phase) = tokio::sync::watch::channel(false);

        let waiting = {
            tokio::spawn(async move {
                let mut display = sink_display();
                waiting::serve(
                    ports,
                    &mut display,
                    ascii_stream(input.as_ref()).chain(futures::stream::pending()),
                    TransitionWatcher::new(phase, |t| *t),
                    &me,
                ).await
            })
        };

        let res = control.ready().recv().await.ok_or(crate::error::NoneError)? == tag;
        phase_sender.send(true)?;
        waiting.await??;
        Ok(res)
    })
}


#[quickcheck]
fn waiting_control_end_of_game(
    players: Vec<crate::player::tests::TestHandle>,
) -> std::io::Result<TestResult> {
    if players.is_empty() {
        return Ok(TestResult::discard())
    }

    tokio::runtime::Runtime::new()?.block_on(async {
        let (notifier, mut disconnects) = tokio::sync::mpsc::unbounded_channel();

        let handles: Vec<_> = players
            .into_iter()
            .map(|h| h.with_notifier(notifier.clone()))
            .collect();
        let tags: Vec<_> = handles.iter().map(crate::player::Handle::tag).collect();

        let (_, ports) = waiting::ports(tags.clone());
        let (_, game_control) = tokio::sync::watch::channel(super::GameControl::EndOfGame);

        waiting::control(ports, game_control, Arc::new(tags.into()), &mut disconnects).await
    });

    Ok(TestResult::passed())
}


#[quickcheck]
fn waiting_control_players(
    players: Vec<(crate::player::tests::TestHandle, bool)>,
    viruses: u8,
    tick: std::time::Duration,
) -> Result<TestResult, Box<dyn std::error::Error>> {
    if players.is_empty() {
        return Ok(TestResult::discard())
    }

    tokio::runtime::Runtime::new()?.block_on(async {
        let (notifier, disconnects) = tokio::sync::mpsc::unbounded_channel();

        let mut handles: Vec<_> = players
            .into_iter()
            .map(|(h, k)| (h.with_notifier(notifier.clone()), k))
            .collect();
        let tags: Vec<_> = handles.iter().map(|(h, _)| h.tag()).collect();

        let (mut ports, control_ports) = waiting::ports(tags.clone());
        let (_, game_control) = tokio::sync::watch::channel(super::GameControl::Settings{viruses, tick});

        let waiting = tokio::spawn(async move {
            let mut disconnects = disconnects;
            waiting::control(control_ports, game_control, Arc::new(tags.into()), &mut disconnects).await
        });

        handles.retain(|(_, k)| *k);
        for (h, _) in handles.iter() {
            ports.ready().send(h.tag()).await?
        }

        waiting.await?;

        // Let's be ultry paranoid and make sure handles lives until now
        drop(handles);
        Ok(TestResult::passed())
    })
}


#[quickcheck]
fn actor_move_output(
    static_field: crate::field::tests::StaticField,
    moves: Vec<crate::field::Movement>,
    row: util::RowIndex,
    a: util::Colour,
    b: util::Colour,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::display::tests::{Area, VT, VTWriter, handle_from_bare};

    let field = crate::display::PlayField::new();
    let area = Area::new_for_placement(0u16, 0u16, &field);

    tokio::runtime::Runtime::new()?.block_on(async {
        let (writer, vt_state) = tokio::sync::watch::channel(VT::new(area.rows(), area.cols()));
        let mut handle = handle_from_bare(VTWriter::from(writer), &[]).await;
        let field = area.instantiate(&mut handle).place_center(field).await?;
        let (event_sender, _) = tokio::sync::mpsc::channel(1);

        let mut actor = round::Actor::new_with_capsule(
            event_sender,
            Default::default(),
            dummy_handle().tag(),
            static_field.into(),
            row,
            [a, b],
        );

        populate_field_display(&mut handle, &field, actor.static_field(), actor.moving_field()).await?;

        for movement in moves {
            actor.r#move(&mut handle, &field, movement).await?;
            check_field_display(&vt_state.borrow(), area, actor.static_field(), actor.moving_field())?;
        }
        Ok(())
    })
}


#[quickcheck]
fn ascii_stream_smoke(orig: crate::tests::ASCIIString) -> Result<bool, ConnTaskError> {
    use futures::TryStreamExt;

    tokio::runtime::Runtime::new()?.block_on(async {
        let orig: String = orig.into();
        let read: String = ASCIIStream::new(orig.as_ref(), Default::default()).try_collect().await?;
        Ok(orig == read)
    })
}


/// Populate a field's display from given static and moving fields
///
async fn populate_field_display(
    handle: &mut crate::display::DrawHandle<'_, impl tokio::io::AsyncWrite + Send + Unpin>,
    field: &crate::display::FieldUpdater,
    static_field: &crate::field::StaticField,
    moving_field: &crate::field::MovingField,
) -> std::io::Result<()> {
    let whole_field = crate::util::ROWS.flat_map(crate::util::complete_row);

    field.place_viruses(
        handle,
        whole_field.clone().filter_map(|p| static_field[p].as_virus().map(|v| (p, v.colour()))),
        Default::default(),
    ).await?;
    field.update(
        handle,
        whole_field.clone().filter_map(|p| static_field[p].as_element().map(|v| (p, Some(v.colour())))),
    ).await?;
    field.update(
        handle,
        whole_field.filter_map(|p| moving_field[p].as_ref().map(|v| (p, Some(v.colour())))),
    ).await?;
    Ok(())
}


/// Check whether the display conveys the contents of the given fields
///
fn check_field_display(
    vt: &crate::display::tests::VT,
    area: crate::display::tests::Area,
    static_field: &crate::field::StaticField,
    moving_field: &crate::field::MovingField,
) -> Result<(), FieldDiscrepancy> {
    use std::convert::TryInto;

    type STC = <crate::field::StaticField as std::ops::Index<util::Position>>::Output;
    use crate::util::PotentiallyColoured;

    use TileContents as TC;

    let v: Vec<_> = crate::display::tests::tile_contents(vt, area).map(|(p, [a, _])| {
        let colour = a.format.fg_colour.map(|(c, _)| c).and_then(|c| c.try_into().ok());
        let displayed = match a.data {
            0x2D | 0x3E => colour.map(TC::Virus).unwrap_or(TC::Invalid),
            0x28        => colour.map(TC::Element).unwrap_or(TC::Invalid),
            0x20        => TC::None,
            _           => TC::Invalid,
        };
        let r#static = match &static_field[p] {
            STC::None               => TC::None,
            STC::CapsuleElement(e)  => TC::Element(e.colour()),
            STC::Virus(v)           => TC::Virus(v.colour()),
        };
        let moving = moving_field[p].colour().map(TC::Element).unwrap_or(TC::None);
        (p, displayed, r#static, moving)
    }).filter(|(_, d, r, s)| !((d == r && *s == TC::None) || (d == s && *r == TC::None))).collect();

    if v.is_empty() {
        Ok(())
    } else {
        Err(FieldDiscrepancy(v))
    }
}


/// Representation of discrepancies found between fields and display
///
#[derive(Debug)]
struct FieldDiscrepancy(Vec<(crate::util::Position, TileContents, TileContents, TileContents)>);

impl std::error::Error for FieldDiscrepancy {}

impl std::fmt::Display for FieldDiscrepancy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Discrepancy: ")?;
        self.0.iter().try_for_each(|((row, col), d, r, s)| write!(
            f,
            " {},{}{}{}{}",
            usize::from(*row),
            usize::from(*col),
            d,
            r,
            s,
        ))
    }
}


/// Representation of a tile contents in a field
///
#[derive(Debug, PartialEq)]
enum TileContents {
    None,
    Element(crate::util::Colour),
    Virus(crate::util::Colour),
    Invalid,
}

impl std::fmt::Display for TileContents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::util::Colour as C;

        f.write_str(match self {
            Self::None                  => "N",
            Self::Element(C::Red)       => "ER",
            Self::Element(C::Yellow)    => "EY",
            Self::Element(C::Blue)      => "EB",
            Self::Virus(C::Red)         => "VR",
            Self::Virus(C::Yellow)      => "VY",
            Self::Virus(C::Blue)        => "VB",
            Self::Invalid               => "I",
        })
    }
}


/// Create a dumb [crate::display::Display] from a sink
///
fn sink_display() -> crate::display::Display<impl tokio::io::AsyncWrite + Send + Unpin + 'static> {
    crate::display::Display::new(tokio::io::sink(), DISPLAY_HEIGHT, DISPLAY_WIDTH)
}


/// Create an [ASCIIStream] from the given input
///
fn ascii_stream(input: &str) -> impl futures::stream::Stream<Item = Result<char, super::ConnTaskError>> + '_ {
    ASCIIStream::new(input.as_ref(), Default::default())
}


/// Construct a pseudo [crate::player::Handle]
///
fn dummy_handle() -> crate::player::Handle {
    use crate::player::{Data, Handle};

    let (notifier, _) = tokio::sync::mpsc::unbounded_channel();
    let addr = std::net::SocketAddrV6::new(std::net::Ipv6Addr::UNSPECIFIED, 0, 0, 0).into();
    let handle = tokio::spawn(futures::future::pending());
    Handle::new(Arc::new(Data::new(Default::default(), addr, handle)), notifier)
}

