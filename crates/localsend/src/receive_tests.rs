#[cfg(test)]
mod tests {
    use super::*;
    use crate::filename::unique_filename;
    use crate::network::{client::https_request, send_files, NetworkError, Peer};
    use crate::protocol::VERSION;
    use rcgen::generate_simple_self_signed;
    use sha2::{Digest, Sha256};
    use std::net::{Ipv4Addr, SocketAddr};

    struct TestReceiver {
        decision: ReceiveDecision,
    }
    impl Receiver for TestReceiver {
        fn accept(
            &self,
            _sender: &DeviceInfo,
            _sender_address: SocketAddr,
            _files: &[FileMetadata],
        ) -> ReceiveDecision {
            self.decision.clone()
        }
    }

    fn identity(name: &str) -> Identity {
        let generated = generate_simple_self_signed(vec![name.into()]).unwrap();
        let cert = generated.cert.der().clone();
        let fingerprint = Sha256::digest(cert.as_ref())
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(":");
        Identity {
            info: DeviceInfo {
                alias: name.into(),
                version: VERSION.into(),
                device_model: Some("test".into()),
                device_type: Some("desktop".into()),
                fingerprint,
                port: Some(53317),
                protocol: Some("https".into()),
                download: false,
            },
            cert,
            key: rustls::pki_types::PrivateKeyDer::Pkcs8(
                rustls::pki_types::PrivatePkcs8KeyDer::from(generated.key_pair.serialize_der()),
            ),
            directory: std::env::temp_dir(),
        }
    }

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "gsh-localsend-{label}-{}-{}",
            std::process::id(),
            Instant::now().elapsed().as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_server(decision: ReceiveDecision, limits: Limits) -> (LocalSendServer, Identity) {
        let identity = identity("receiver");
        let server = LocalSendServer::bind(
            identity.clone(),
            "127.0.0.1:0".parse::<SocketAddr>().unwrap(),
            Arc::new(TestReceiver { decision }),
            limits,
        )
        .unwrap();
        (server, identity)
    }

    fn peer(server: &LocalSendServer, identity: &Identity) -> Peer {
        Peer {
            info: DeviceInfo {
                port: Some(server.address().port()),
                ..identity.info.clone()
            },
            ip: Ipv4Addr::LOCALHOST,
            seen_at: 1,
        }
    }

    #[test]
    fn rejects_unsafe_and_reserved_names_and_keeps_unicode() {
        for name in ["../x", "/etc/passwd", "C:\\a", "CON", ""] {
            assert!(sanitize_filename(name).is_err(), "{name}");
        }
        assert_eq!(sanitize_filename("olá 世界.txt").unwrap(), "olá 世界.txt");
    }
    #[test]
    fn collision_gets_numbered_suffix() {
        let dir = std::env::temp_dir().join(format!("gsh-localsend-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("x.txt"), b"existing").unwrap();
        assert_eq!(unique_filename(&dir, "x.txt").unwrap(), "x (1).txt");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn send_receive_succeeds_after_acceptance() {
        let destination = temp_dir("accepted");
        let source_dir = temp_dir("source");
        let source = source_dir.join("hello.txt");
        fs::write(&source, b"hello LocalSend").unwrap();
        let source_dir_two = temp_dir("source-two");
        let source_two = source_dir_two.join("hello.txt");
        fs::write(&source_two, b"second copy").unwrap();
        let (server, server_identity) = make_server(
            ReceiveDecision::Accept {
                dir: destination.clone(),
            },
            Limits::default(),
        );
        let sender = identity("sender");
        send_files(
            &peer(&server, &server_identity),
            &sender,
            &[source, source_two],
            1024,
            |_, _, _| {},
            || false,
        )
        .unwrap();
        assert_eq!(
            fs::read(destination.join("hello.txt")).unwrap(),
            b"hello LocalSend"
        );
        assert_eq!(
            fs::read(destination.join("hello (1).txt")).unwrap(),
            b"second copy"
        );
        let _ = fs::remove_dir_all(destination);
        let _ = fs::remove_dir_all(source_dir);
        let _ = fs::remove_dir_all(source_dir_two);
    }

    #[test]
    fn rejection_and_oversize_create_no_received_file() {
        let destination = temp_dir("rejected");
        let source_dir = temp_dir("large");
        let source = source_dir.join("large.bin");
        fs::write(&source, b"12345").unwrap();
        let (server, server_identity) = make_server(
            ReceiveDecision::Accept {
                dir: destination.clone(),
            },
            Limits {
                max_file_size: 4,
                max_files: 1,
                max_total_size: 4,
            },
        );
        let sender = identity("sender");
        let error = send_files(
            &peer(&server, &server_identity),
            &sender,
            std::slice::from_ref(&source),
            10,
            |_, _, _| {},
            || false,
        )
        .unwrap_err();
        assert!(matches!(error, NetworkError::Http(413)));
        assert!(fs::read_dir(&destination).unwrap().next().is_none());
        assert!(matches!(
            https_request(
                server.address(),
                &server_identity.info.fingerprint,
                "POST",
                &format!("{API}/upload"),
                &[]
            ),
            Err(NetworkError::Http(400))
        ));
        drop(server);
        let (rejected_server, rejected_identity) =
            make_server(ReceiveDecision::Reject, Limits::default());
        let rejected = send_files(
            &peer(&rejected_server, &rejected_identity),
            &sender,
            std::slice::from_ref(&source),
            10,
            |_, _, _| {},
            || false,
        );
        assert!(matches!(rejected, Err(NetworkError::Http(403))));
        assert!(fs::read_dir(&destination).unwrap().next().is_none());
        let _ = fs::remove_dir_all(destination);
        let _ = fs::remove_dir_all(source_dir);
    }

    #[test]
    fn upload_without_session_or_token_is_forbidden() {
        let destination = temp_dir("unauthorized");
        let (server, server_identity) = make_server(
            ReceiveDecision::Accept {
                dir: destination.clone(),
            },
            Limits::default(),
        );
        let path = format!("{API}/upload?sessionId=missing&fileId=missing&token=bad");
        assert!(matches!(
            https_request(
                server.address(),
                &server_identity.info.fingerprint,
                "POST",
                &path,
                &[]
            ),
            Err(NetworkError::Http(403))
        ));
        assert!(fs::read_dir(&destination).unwrap().next().is_none());
        let _ = fs::remove_dir_all(destination);
    }

    #[test]
    fn protocol_v2_handwritten_https_requests_match_official_payload_shape() {
        let destination = temp_dir("manual-protocol");
        let (server, server_identity) = make_server(
            ReceiveDecision::Accept {
                dir: destination.clone(),
            },
            Limits::default(),
        );
        let sender = identity("official-client");
        let address = server.address();
        let fingerprint = &server_identity.info.fingerprint;

        let info: DeviceInfo = serde_json::from_slice(
            &https_request(address, fingerprint, "GET", &format!("{API}/info"), &[]).unwrap(),
        )
        .unwrap();
        assert_eq!(info.alias, server_identity.info.alias);
        let registration = https_request(
            address,
            fingerprint,
            "POST",
            &format!("{API}/register"),
            &serde_json::to_vec(&sender.info).unwrap(),
        )
        .unwrap();
        assert_eq!(serde_json::from_slice::<DeviceInfo>(&registration).unwrap(), info);

        let request = serde_json::json!({
            "info": sender.info,
            "files": {
                "manual-file": {
                    "id": "manual-file",
                    "fileName": "official-shape.txt",
                    "size": 16,
                    "fileType": "application/octet-stream"
                }
            }
        });
        let prepared: UploadSession = serde_json::from_slice(
            &https_request(
                address,
                fingerprint,
                "POST",
                &format!("{API}/prepare-upload"),
                &serde_json::to_vec(&request).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let token = prepared.files.get("manual-file").unwrap();
        let path = format!(
            "{API}/upload?sessionId={}&fileId=manual-file&token={token}",
            prepared.session_id
        );
        https_request(address, fingerprint, "POST", &path, b"official payload")
            .unwrap();
        assert_eq!(
            fs::read(destination.join("official-shape.txt")).unwrap(),
            b"official payload"
        );
        let _ = fs::remove_dir_all(destination);
    }
}
