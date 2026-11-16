//! Servidor **SOCKS5 CONNECT** local sobre um túnel SSH (`local_socks_server.dart`).
//!
//! O MongoDB descobre os membros do replica set pelo `hello` e passa a conectar nos hostnames reais
//! anunciados pelo servidor — um port-forward fixo só alcançaria o primeiro nó. Com SOCKS5 local,
//! o driver escolhe o destino em cada conexão e o túnel só roteia (`direct-tcpip`), inclusive para
//! `mongodb+srv://`.

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use russh::client::Msg;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

/// Proxy SOCKS5 vivo em `127.0.0.1:<porta>`.
pub struct LocalSocksServer {
    pub host: String,
    pub port: u16,
    alive: Arc<AtomicBool>,
    shutdown: Option<oneshot::Sender<()>>,
}

impl LocalSocksServer {
    pub async fn start(
        handle: Arc<russh::client::Handle<crate::ssh::ClientHandler>>,
    ) -> std::io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        let port = listener.local_addr()?.port();
        let alive = Arc::new(AtomicBool::new(true));
        let alive_task = Arc::clone(&alive);
        let (tx, mut rx) = oneshot::channel();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut rx => break,
                    accepted = listener.accept() => {
                        match accepted {
                            Ok((stream, _)) => {
                                let h = Arc::clone(&handle);
                                tokio::spawn(async move {
                                    let _ = handle_client(stream, h).await;
                                });
                            }
                            Err(_) => {
                                // Erro transitório de accept não derruba o proxy (lição do Flutter).
                            }
                        }
                    }
                }
            }
            alive_task.store(false, Ordering::SeqCst);
        });

        Ok(Self {
            host: "127.0.0.1".to_string(),
            port,
            alive,
            shutdown: Some(tx),
        })
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    pub fn close(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for LocalSocksServer {
    fn drop(&mut self) {
        self.close();
    }
}

async fn handle_client(
    mut stream: TcpStream,
    handle: Arc<russh::client::Handle<crate::ssh::ClientHandler>>,
) -> Result<(), ()> {
    let _ = stream.set_nodelay(true);

    // 1. Greeting: VER=5, NMETHODS, METHODS...
    let mut hdr = [0u8; 2];
    stream.read_exact(&mut hdr).await.map_err(|_| ())?;
    if hdr[0] != 5 {
        return Err(());
    }
    let nmethods = usize::from(hdr[1]);
    let mut methods = vec![0u8; nmethods];
    stream.read_exact(&mut methods).await.map_err(|_| ())?;
    if !methods.contains(&0) {
        let _ = stream.write_all(&[5, 0xFF]).await;
        return Err(());
    }
    stream.write_all(&[5, 0]).await.map_err(|_| ())?;

    // 2. Request: VER=5, CMD=1 (CONNECT), RSV=0, ATYP, DST.ADDR, DST.PORT
    let mut req = [0u8; 4];
    stream.read_exact(&mut req).await.map_err(|_| ())?;
    if req[0] != 5 || req[1] != 1 {
        let _ = stream.write_all(&[5, 7, 0, 1, 0, 0, 0, 0, 0, 0]).await;
        return Err(());
    }
    let host = match req[3] {
        1 => {
            let mut ip = [0u8; 4];
            stream.read_exact(&mut ip).await.map_err(|_| ())?;
            Ipv4Addr::from(ip).to_string()
        }
        3 => {
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await.map_err(|_| ())?;
            let mut name = vec![0u8; usize::from(len[0])];
            stream.read_exact(&mut name).await.map_err(|_| ())?;
            String::from_utf8(name).map_err(|_| ())?
        }
        4 => {
            let mut ip = [0u8; 16];
            stream.read_exact(&mut ip).await.map_err(|_| ())?;
            Ipv6Addr::from(ip).to_string()
        }
        _ => {
            let _ = stream.write_all(&[5, 8, 0, 1, 0, 0, 0, 0, 0, 0]).await;
            return Err(());
        }
    };
    let mut port_buf = [0u8; 2];
    stream.read_exact(&mut port_buf).await.map_err(|_| ())?;
    let port = u16::from_be_bytes(port_buf);

    let channel: russh::Channel<Msg> = match handle
        .channel_open_direct_tcpip(host, u32::from(port), "127.0.0.1", 0)
        .await
    {
        Ok(ch) => ch,
        Err(_) => {
            let _ = stream.write_all(&[5, 4, 0, 1, 0, 0, 0, 0, 0, 0]).await;
            return Err(());
        }
    };

    let bound = stream
        .local_addr()
        .unwrap_or_else(|_| SocketAddr::from((Ipv4Addr::LOCALHOST, 0)));
    let bound_port = bound.port().to_be_bytes();
    stream
        .write_all(&[5, 0, 0, 1, 127, 0, 0, 1, bound_port[0], bound_port[1]])
        .await
        .map_err(|_| ())?;

    let mut ssh_stream = channel.into_stream();
    let _ = tokio::io::copy_bidirectional(&mut stream, &mut ssh_stream).await;
    Ok(())
}
