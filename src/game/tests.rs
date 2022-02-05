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
fn ascii_stream_smoke(orig: crate::tests::ASCIIString) -> Result<bool, ConnTaskError> {
    use futures::TryStreamExt;

    tokio::runtime::Runtime::new()?.block_on(async {
        let orig: String = orig.into();
        let read: String = ASCIIStream::new(orig.as_ref(), Default::default()).try_collect().await?;
        Ok(orig == read)
    })
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

