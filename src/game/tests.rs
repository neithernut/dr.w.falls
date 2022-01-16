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

