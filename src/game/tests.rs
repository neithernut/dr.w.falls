//! Game tests

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
    orig: crate::player::tests::Name,
    addr: std::net::SocketAddr,
    registrtion_success: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    use futures::StreamExt;

    use crate::player::{Data, Handle};

    let mut input: String = orig.clone().into();
    input.push('\n');

    tokio::runtime::Runtime::new()?.block_on(async {
        let (ports, mut control) = lobby::ports();
        let (phase_sender, phase) = tokio::sync::watch::channel(false);
        let orig_token: lobby::ConnectionToken = addr.into();

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
            let (notifier, _) = tokio::sync::mpsc::unbounded_channel();
            let handle = tokio::spawn(futures::future::pending());
            Some(Handle::new(Arc::new(Data::new(orig.clone().into(), addr, handle)), notifier))
        } else {
            None
        };
        let tag = handle.as_ref().map(Handle::tag);

        let (name, token) = control
            .receive_registration(handle)
            .await
            .ok_or(crate::error::NoneError)?;
        phase_sender.send(true)?;
        let res = lobby.await??.map(|h| h.tag()) == tag &&
            name == orig.as_ref() &&
            token == orig_token;
        Ok(res)
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

