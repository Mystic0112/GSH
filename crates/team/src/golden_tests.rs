//! Paridade com o Flutter: os vetores de `tests/dart_golden.json` sao gerados por
//! `golden/team.dart` (copia verbatim de team_crypto/lan_team_farm_channel/lan_team_mail).
//! Cada caso do Dart (inclusive os que ele rejeita) tem que dar o mesmo resultado aqui.
//!
//! Sentido contrario (Rust -> Dart): com `GSH_TEAM_RUST_OUT=<arquivo>` o teste
//! `emite_frames_para_o_dart` grava frames feitos aqui; `team.dart check` os abre.

use crate::crypto::{derive_key, open, seal, seal_with_nonce};
use crate::knock::{decode_knock, encode_knock, Knock, KnockKind};
use crate::packet::{decode_packet, encode_packet, gunzip, packet_body, FarmLoop, Packet};
use crate::{ipv4_in_cidr, Activity, TeamAgent};
use serde_json::{json, Value};

const GOLDEN: &str = include_str!("../tests/dart_golden.json");

fn golden() -> Value {
    serde_json::from_str(GOLDEN).unwrap()
}

fn s(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap().to_owned()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex(h: &str) -> Vec<u8> {
    (0..h.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
        .collect()
}

fn opt(o: &Option<String>) -> Value {
    o.as_ref().map_or(Value::Null, |s| json!(s))
}

fn loop_json(l: &FarmLoop) -> Value {
    json!({
        "id": l.id, "parentId": opt(&l.parent_id), "teammate": opt(&l.teammate),
        "harness": l.harness, "project": l.project, "title": l.title,
        "agentType": opt(&l.agent_type), "activity": l.activity.name(), "tool": opt(&l.tool),
        "outputTokens": l.output_tokens, "contextTokens": l.context_tokens,
        "inputTokens": l.input_tokens, "cacheReadTokens": l.cache_read_tokens,
        "contextWindow": l.context_window, "lastSeenMs": l.last_seen_ms,
    })
}

/// O mesmo formato de `packetJson` do harness.
fn packet_json(p: Option<&Packet>) -> Value {
    let Some(p) = p else { return Value::Null };
    json!({
        "machineId": p.machine_id, "name": p.name, "team": p.team,
        "loops": p.loops.iter().map(loop_json).collect::<Vec<_>>(),
        "agents": p.agents.iter().map(|a| json!({
            "tabId": a.tab_id, "label": a.label, "harness": a.harness,
            "project": a.project, "sessionId": opt(&a.session_id),
        })).collect::<Vec<_>>(),
        "mailPort": p.mail_port.map_or(Value::Null, |m| json!(m)),
    })
}

fn knock_json(k: Option<&Knock>) -> Value {
    let Some(k) = k else { return Value::Null };
    json!({
        "id": k.id, "kind": match k.kind { KnockKind::Message => "message", KnockKind::File => "file" },
        "fromMachine": k.from_machine, "fromName": k.from_name, "fromAgent": k.from_agent,
        "fromSessionId": opt(&k.from_session_id), "toTabId": k.to_tab_id, "text": k.text,
        "fileName": opt(&k.file_name),
        "fileHex": k.file_bytes.as_ref().map_or(Value::Null, |b| json!(hex(b))),
    })
}

fn key_for(c: &Value) -> Option<crate::TeamKey> {
    let secret = s(c, "secret");
    (!secret.is_empty()).then(|| derive_key(&s(c, "team"), &secret))
}

#[test]
fn chave_pbkdf2_igual_a_do_dart() {
    for k in golden()["keys"].as_array().unwrap() {
        let key = derive_key(&s(k, "team"), &s(k, "secret"));
        assert_eq!(hex(key.as_bytes()), s(k, "keyHex"), "{k}");
    }
}

#[test]
fn frame_selado_byte_a_byte_igual_ao_do_dart() {
    let g = golden();
    let seals = g["seals"].as_array().unwrap();
    assert!(seals.len() >= 15);
    for v in seals {
        let key = derive_key(&s(v, "team"), &s(v, "secret"));
        let nonce: [u8; 12] = unhex(&s(v, "nonceHex")).try_into().unwrap();
        let plain = unhex(&s(v, "plainHex"));
        let frame = seal_with_nonce(&key, nonce, &plain).unwrap();
        assert_eq!(hex(&frame), s(v, "frameHex"), "Rust sela diferente do Dart");
        // e o Rust abre o frame que o Dart selou
        assert_eq!(open(&key, &unhex(&s(v, "frameHex"))), Some(plain));
    }
}

fn check_packets(list: &Value) {
    for p in list.as_array().unwrap() {
        let frame = unhex(&s(p, "frameHex"));
        let key = key_for(p);
        let got = decode_packet(&frame, key.as_ref(), None);
        assert_eq!(packet_json(got.as_ref()), p["expected"], "{}", p["name"]);
    }
}

#[test]
fn decodifica_os_pacotes_do_dart() {
    check_packets(&golden()["packets"]);
}

#[test]
fn bordas_do_decode_dao_o_mesmo_resultado_que_o_dart() {
    let g = golden();
    assert!(g["packetEdges"].as_array().unwrap().len() >= 19);
    check_packets(&g["packetEdges"]);
}

/// O que o Rust escreve no fio e o JSON que o Dart escreveria (comparado como valor: a ordem
/// das chaves nao importa para quem decodifica).
#[test]
fn corpo_do_pacote_igual_ao_do_dart() {
    for p in golden()["packets"].as_array().unwrap() {
        let frame = unhex(&s(p, "frameHex"));
        let key = key_for(p);
        let dart_gz = match &key {
            Some(k) => open(k, &frame).unwrap(),
            None => frame,
        };
        let dart_body: Value = serde_json::from_slice(&gunzip(&dart_gz).unwrap()).unwrap();
        let decoded = decode_packet(&unhex(&s(p, "frameHex")), key.as_ref(), None).unwrap();
        let ours: Value = serde_json::from_slice(&packet_body(&decoded)).unwrap();
        assert_eq!(ours, dart_body, "{}", p["name"]);
    }
}

#[test]
fn batidas_do_dart_decodificam_e_o_corpo_e_igual() {
    for k in golden()["knocks"].as_array().unwrap() {
        let now = k["nowMs"].as_i64().unwrap();
        let key = derive_key(&s(k, "team"), &s(k, "secret"));
        // a linha que o Dart mandou pelo TCP: {"e": base64(frame)}
        let line: Value = serde_json::from_str(&s(k, "lineJson")).unwrap();
        let sealed = base64_decode(line["e"].as_str().unwrap());
        let body = String::from_utf8(open(&key, &sealed).unwrap()).unwrap();
        assert_eq!(body, s(k, "bodyJson"));
        let got = decode_knock(&body, now, 300_000);
        assert_eq!(knock_json(got.as_ref()), k["expected"]);
        // re-encodar da o mesmo JSON (mesmo `ts`, time aparado)
        let again = encode_knock(&got.unwrap(), " gorila ", now);
        let a: Value = serde_json::from_str(&again).unwrap();
        let b: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(a, b);
    }
}

fn base64_decode(s: &str) -> Vec<u8> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(s).unwrap()
}

#[test]
fn bordas_da_batida() {
    let g = golden();
    assert!(g["knockEdges"].as_array().unwrap().len() >= 10);
    for e in g["knockEdges"].as_array().unwrap() {
        let got = decode_knock(&s(e, "bodyJson"), e["nowMs"].as_i64().unwrap(), 300_000);
        assert_eq!(knock_json(got.as_ref()), e["expected"], "{}", e["name"]);
    }
}

#[test]
fn cidr_igual_ao_do_dart() {
    let g = golden();
    for c in g["cidr"].as_array().unwrap() {
        assert_eq!(
            ipv4_in_cidr(&s(c, "ip"), &s(c, "cidr")),
            c["expected"].as_bool().unwrap(),
            "{c}"
        );
    }
}

fn sample_packet() -> Packet {
    let lp = |id: &str, a: Activity, ls: i64| FarmLoop {
        id: id.into(),
        parent_id: (id == "ag").then(|| "main".into()),
        teammate: None,
        harness: "claude".into(),
        project: "gsh".into(),
        title: "Portar \u{2713}".into(),
        agent_type: None,
        activity: a,
        tool: Some("Edit".into()),
        output_tokens: 1,
        context_tokens: 2,
        input_tokens: 3,
        cache_read_tokens: 4,
        context_window: 1_000_000,
        last_seen_ms: ls,
    };
    Packet {
        machine_id: "00112233445566ff".into(),
        name: "Rustaceo \u{2713}".into(),
        team: "gorila".into(),
        loops: vec![
            lp("main", Activity::Editing, 10),
            lp("ag", Activity::Waiting, 5),
        ],
        agents: vec![TeamAgent {
            tab_id: "t1".into(),
            label: "claude".into(),
            harness: "claude".into(),
            project: "gsh".into(),
            session_id: Some("main".into()),
        }],
        mail_port: Some(40123),
        address: None,
    }
}

/// Grava o que o Rust produz para o `team.dart check` abrir (so com a variavel de ambiente).
#[test]
fn emite_frames_para_o_dart() {
    let Ok(path) = std::env::var("GSH_TEAM_RUST_OUT") else {
        return;
    };
    let key = derive_key("gorila", "segredo");
    let p = sample_packet();
    let mut seals = Vec::new();
    for plain in [&b""[..], "olá, time ✓".as_bytes(), &[7_u8; 300][..]] {
        seals.push(json!({
            "team": "gorila", "secret": "segredo", "plainHex": hex(plain),
            "frameHex": hex(&seal(&key, plain).unwrap()),
        }));
    }
    let mut packets = Vec::new();
    for (name, secret) in [("rust-sealed", "segredo"), ("rust-plain", "")] {
        let k = (!secret.is_empty()).then(|| key.clone());
        let frame = encode_packet(&p, k.as_ref()).unwrap();
        let mut expected = packet_json(Some(&p));
        // o Dart devolve o loop mais recente primeiro e sem `address`
        expected["loops"]
            .as_array_mut()
            .unwrap()
            .sort_by_key(|l| std::cmp::Reverse(l["lastSeenMs"].as_i64()));
        packets.push(json!({
            "name": name, "team": "gorila", "secret": secret,
            "frameHex": hex(&frame), "expected": expected,
        }));
    }
    let k = Knock {
        id: "rk1".into(),
        kind: KnockKind::File,
        from_machine: "00ff".into(),
        from_name: "Rustaceo \u{2713}".into(),
        from_agent: "claude".into(),
        from_session_id: Some("main".into()),
        to_tab_id: "t9".into(),
        text: "segue \"o\" arquivo\n".into(),
        file_name: Some("a.bin".into()),
        file_bytes: Some(vec![0, 1, 2, 254, 255]),
    };
    let now = 1_790_000_000_000_i64;
    let body = encode_knock(&k, "gorila", now);
    let frame = seal(&key, body.as_bytes()).unwrap();
    use base64::Engine;
    let line = json!({"e": base64::engine::general_purpose::STANDARD.encode(frame)}).to_string();
    let knocks = vec![json!({
        "bodyJson": body, "lineJson": line, "team": "gorila", "secret": "segredo",
        "nowMs": now, "expected": knock_json(Some(&k)),
    })];
    let out = json!({"seals": seals, "packets": packets, "knocks": knocks});
    std::fs::write(path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
}

/// Rust -> Rust no mesmo fio: o que `encode_packet` faz, `decode_packet` desfaz, e a chave
/// errada (ou o modo errado) descarta.
#[test]
fn ida_e_volta_e_modos_nao_se_misturam() {
    let key = derive_key("gorila", "segredo");
    let p = sample_packet();
    let sealed = encode_packet(&p, Some(&key)).unwrap();
    let plain = encode_packet(&p, None).unwrap();
    let back = decode_packet(&sealed, Some(&key), None).unwrap();
    assert_eq!(back.loops.len(), 2);
    assert_eq!(back.loops.first().map(|l| l.id.as_str()), Some("main"));
    assert_eq!(back.mail_port, Some(40123));
    assert!(decode_packet(&plain, Some(&key), None).is_none());
    assert!(decode_packet(&sealed, None, None).is_none());
    assert!(decode_packet(&sealed, Some(&derive_key("gorila", "outra")), None).is_none());
    assert!(decode_packet(&plain, None, None).is_some());
}

#[test]
fn lixo_nunca_estoura() {
    let key = derive_key("gorila", "segredo");
    let junk: [&[u8]; 6] = [
        b"",
        b"\x00",
        b"\x02",
        &[2; 28],
        &[2; 29],
        &[0x1f, 0x8b, 8, 0, 0, 0, 0, 0],
    ];
    for j in junk {
        assert!(decode_packet(j, Some(&key), None).is_none());
        assert!(decode_packet(j, None, None).is_none());
    }
    // 1 MiB de zeros em gzip (bomba pequena): passa do teto de 2 MiB depois de expandir?
    // 3 MiB de zeros comprimem em ~3 KiB; tem que ser recusado sem alocar tudo.
    use flate2::{write::GzEncoder, Compression};
    use std::io::Write;
    let mut gz = GzEncoder::new(Vec::new(), Compression::best());
    gz.write_all(&vec![b' '; 3 * 1024 * 1024]).unwrap();
    let bomb = gz.finish().unwrap();
    assert!(bomb.len() < 10_000);
    assert!(decode_packet(&bomb, None, None).is_none());
    assert!(gunzip(&bomb).is_none());
}
