use super::*;
use std::io;

fn assert_closed(conn: &mut Conn) {
    let timeout = Some(WAIT);
    match conn {
        #[cfg(unix)]
        Conn::Unix(stream) => stream.set_read_timeout(timeout).unwrap(),
        Conn::Tcp(stream) => stream.set_read_timeout(timeout).unwrap(),
    }
    match conn.read(&mut [0]) {
        Ok(0) => {}
        Err(e) if e.kind() == io::ErrorKind::ConnectionReset => {}
        other => panic!("esperava conexão fechada, veio {other:?}"),
    }
}

#[test]
fn conexoes_acima_do_limite_fecham_e_uma_vaga_pode_ser_reutilizada() {
    for transport in transports() {
        let fx = Fixture::start(transport);
        let cmd = fx.authed(wire::cmd_request("permission-ask", None, None));
        let mut pending = Vec::new();
        for _ in 0..64 {
            let mut client = Conn::connect(&fx.server);
            writeln!(client, "{cmd}").unwrap();
            let Delivery::Command(command) = fx.rx.recv_timeout(WAIT).unwrap() else {
                panic!("esperava comando pendente");
            };
            pending.push((client, command));
        }
        assert_closed(&mut Conn::connect(&fx.server));

        let (mut client, command) = pending.pop().unwrap();
        drop(command.reply);
        let mut reply = String::new();
        client.read_to_string(&mut reply).unwrap();
        assert_eq!(reply, "{\"error\":\"command timed out\",\"ok\":false}\n");

        let msg = fx.authed(serde_json::json!({"paneId": "recuperado", "st": "idle"}));
        let deadline = std::time::Instant::now() + WAIT;
        loop {
            // EOF pode chegar ao cliente antes de o accept recolher o worker.
            let mut probe = Conn::connect(&fx.server);
            let _ = writeln!(probe, "{msg}");
            match fx.rx.recv_timeout(QUIET) {
                Ok(Delivery::Status(status)) => {
                    assert_eq!(status.pane_id, "recuperado");
                    break;
                }
                Err(RecvTimeoutError::Timeout) if std::time::Instant::now() < deadline => {}
                other => panic!("esperava reutilizar a vaga, veio {other:?}"),
            }
        }
    }
}

#[test]
fn parar_fecha_conexoes_em_leitura_e_comandos_pendentes() {
    for transport in transports() {
        let home = unique_dir("gi");
        let (server, rx) = StatusServer::start(&config(&home, transport)).unwrap();
        let mut incomplete = Conn::connect(&server);
        incomplete.write_all(b"{").unwrap();

        let mut waiting = Conn::connect(&server);
        let mut cmd = wire::cmd_request("permission-ask", None, None);
        if let Some((_, token)) = server
            .hook_env()
            .iter()
            .find(|(k, _)| *k == wire::ENV_STATUS_TOKEN)
        {
            cmd = wire::with_token(cmd, token);
        }
        writeln!(waiting, "{cmd}").unwrap();
        let Delivery::Command(command) = rx.recv_timeout(WAIT).unwrap() else {
            panic!("esperava comando pendente");
        };

        let (stopped_tx, stopped_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            server.stop();
            stopped_tx.send(()).unwrap();
        });
        stopped_rx.recv_timeout(WAIT).expect("stop travou");
        assert_closed(&mut incomplete);
        assert_closed(&mut waiting);
        assert!(command.reply.send(serde_json::json!({"ok": true})).is_err());
        assert!(matches!(
            rx.recv_timeout(Duration::ZERO),
            Err(RecvTimeoutError::Disconnected)
        ));
        let _ = std::fs::remove_dir_all(home);
    }
}

#[test]
fn muitas_conexoes_simultaneas_chegam_todas() {
    for t in transports() {
        let fx = Fixture::start(t);
        let n = 40;
        let lines: Vec<String> = (0..n)
            .map(|i| {
                let msg = serde_json::json!({"paneId": format!("p{i}"), "st": "working"});
                format!("{}\n", fx.authed(msg))
            })
            .collect();
        std::thread::scope(|s| {
            for line in &lines {
                let server = &fx.server;
                s.spawn(move || send_to(server, line));
            }
        });
        let mut got: Vec<String> = (0..n).map(|_| fx.status().pane_id).collect();
        got.sort();
        let mut want: Vec<String> = (0..n).map(|i| format!("p{i}")).collect();
        want.sort();
        assert_eq!(got, want, "{t:?}");
    }
}

#[test]
fn flavors_dividem_o_gsh_dir_sem_se_apagar() {
    // Dev e instalado rodam lado a lado sobre o mesmo `~/.gsh`.
    let home = unique_dir("gi");
    let mk = |debug: bool| {
        let mut c = config(&home, Transport::native());
        c.debug = debug;
        c
    };
    let (a, _ra) = StatusServer::start(&mk(false)).unwrap();
    let (b, rb) = StatusServer::start(&mk(true)).unwrap();
    assert_ne!(a.endpoint(), b.endpoint());
    drop(a);
    // Derrubar um não derruba o do outro.
    let mut c = Conn::connect(&b);
    let tok = b
        .hook_env()
        .into_iter()
        .find(|(k, _)| *k == wire::ENV_STATUS_TOKEN)
        .map(|(_, v)| v);
    let msg = serde_json::json!({"paneId": "dev", "st": "idle"});
    let line = match tok {
        Some(tok) => wire::with_token(msg, &tok),
        None => msg,
    };
    c.write_all(format!("{line}\n").as_bytes()).unwrap();
    assert!(matches!(
        rb.recv_timeout(WAIT).unwrap(),
        Delivery::Status(_)
    ));
    drop(c);
    drop(b);
    let _ = std::fs::remove_dir_all(&home);
}

#[cfg(unix)]
#[test]
fn parar_nao_trava_se_o_socket_foi_apagado_por_fora() {
    // Outra instância do mesmo flavor apaga e refaz o arquivo: o nosso `stop`
    // não pode depender do caminho para terminar.
    let home = unique_dir("gi");
    let (server, _rx) = StatusServer::start(&config(&home, Transport::Unix)).unwrap();
    let Endpoint::Unix(path) = server.endpoint().clone() else {
        panic!("esperava Unix");
    };
    std::fs::remove_file(&path).unwrap();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        server.stop();
        let _ = done_tx.send(());
    });
    done_rx.recv_timeout(WAIT).expect("stop travou");
    let _ = std::fs::remove_dir_all(&home);
}
