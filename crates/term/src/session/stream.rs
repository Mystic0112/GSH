use std::io::{self, Read, Write};

use super::{validate_dimensions, TermError, TermSession};

type Resize = dyn Fn(u16, u16) -> io::Result<()> + Send + Sync;

pub(super) struct Control {
    pub(super) resize: Box<Resize>,
    pub(super) close: Box<dyn FnOnce() + Send + Sync>,
}

impl TermSession {
    /// Emula um terminal ligado a um fluxo de bytes de outra sessão.
    ///
    /// O leitor recebe a saída do PTY e o escritor envia teclas e respostas VT.
    /// O transporte deve interromper as leituras e escritas bloqueadas quando
    /// `close` for chamado no encerramento ou em caso de erro de inicialização.
    pub fn from_stream(
        reader: impl Read + Send + 'static,
        writer: impl Write + Send + 'static,
        resize: impl Fn(u16, u16) -> io::Result<()> + Send + Sync + 'static,
        close: impl FnOnce() + Send + Sync + 'static,
        cols: u16,
        rows: u16,
        scrollback_lines: usize,
    ) -> Result<Self, TermError> {
        if let Err(err) = validate_dimensions(cols, rows) {
            close();
            return Err(err);
        }

        let mut session = Self::new(cols, rows, scrollback_lines);
        session.stream = Some(Control {
            resize: Box::new(resize),
            close: Box::new(close),
        });
        session.start_reader(reader, Box::new(writer))?;
        Ok(session)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::net::Shutdown;
    use std::os::unix::net::UnixStream;
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;

    #[test]
    fn emulates_stream_and_shuts_down_reader() {
        let (socket, mut peer) = UnixStream::pair().unwrap();
        let reader = socket.try_clone().unwrap();
        let shutdown = socket.try_clone().unwrap();
        let (resized, sizes) = mpsc::sync_channel(1);
        let session = TermSession::from_stream(
            reader,
            socket,
            move |cols, rows| {
                resized.send((cols, rows)).unwrap();
                Ok(())
            },
            move || shutdown.shutdown(Shutdown::Both).unwrap(),
            80,
            24,
            100,
        )
        .unwrap();

        let timeout = Some(Duration::from_secs(2));
        peer.set_read_timeout(timeout).unwrap();
        peer.write_all(b"remote output\x1b[5n").unwrap();
        let mut reply = [0; 4];
        peer.read_exact(&mut reply).unwrap();
        assert_eq!(&reply, b"\x1b[0n");
        assert!(session.snapshot().text().contains("remote output"));
        assert_eq!(session.child_pid(), None);

        session.send(b"input").unwrap();
        let mut input = [0; 5];
        peer.read_exact(&mut input).unwrap();
        assert_eq!(&input, b"input");

        session.resize(100, 30).unwrap();
        assert_eq!(sizes.try_recv().unwrap(), (100, 30));
        assert_eq!(session.size(), (100, 30));
        drop(session);
        assert_eq!(peer.read(&mut reply).unwrap(), 0);
    }
}
