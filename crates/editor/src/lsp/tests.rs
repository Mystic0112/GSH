use super::*;
use document::{edit_text, end_position, Position, Positions, TextEdit, TextRange};
use serde_json::json;
use std::time::Duration;
use tokio::io::{duplex, AsyncWriteExt};
use tokio::{io::BufReader, time};
use wire::{notification, read_frame, write_frame};

#[tokio::test]
async fn framed_rpc_uses_byte_length_and_rejects_malformed_messages() {
    let (mut writer, reader) = duplex(512);
    let expected = notification("test", json!({"value": "ação 🎵"}));
    write_frame(&mut writer, expected.clone()).await.unwrap();
    assert_eq!(
        read_frame(&mut BufReader::new(reader)).await.unwrap(),
        expected
    );
    for input in [
        "Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}",
        "Content-Length: 999999999\r\n\r\n",
        "Content-Length: 2\r\n\r\n{}",
        "Content-Length: 1\r\n",
    ] {
        assert!(read_frame(&mut BufReader::new(input.as_bytes()))
            .await
            .is_err());
    }
    let oversized = format!("X: {}\r\nContent-Length: 2\r\n\r\n{{}}", "x".repeat(9000));
    assert!(read_frame(&mut BufReader::new(oversized.as_bytes()))
        .await
        .is_err());
    let (mut writer, reader) = duplex(512);
    writer
        .write_all(b"Content-Length: 8\r\n\r\n{}")
        .await
        .unwrap();
    drop(writer);
    assert!(read_frame(&mut BufReader::new(reader)).await.is_err());
}

fn pos(line: u32, character: u32) -> Position {
    Position { line, character }
}

fn edit(start: Position, end: Position, new_text: &str) -> TextEdit {
    TextEdit {
        range: TextRange { start, end },
        new_text: new_text.into(),
    }
}

#[test]
fn utf16_ranges_handle_surrogates_crlf_and_trailing_empty_lines() {
    let source = "a🎵é\r\nβ\r\nx\n";
    let positions = Positions::new(source);
    assert_eq!(positions.offset(pos(0, 3)), Some(5));
    assert_eq!(positions.offset(pos(0, 2)), None);
    assert_eq!(positions.offset(pos(1, 0)), Some(9));
    assert_eq!(positions.offset(pos(3, 0)), Some(source.len()));
    assert_eq!(positions.offset(pos(3, 1)), None);
    assert_eq!(end_position(source), (3, 0));
    assert_eq!(end_position("a\r🎵"), (1, 2));
}

#[test]
fn applies_disjoint_edits_in_original_coordinates_and_rejects_overlap() {
    let source = "a🎵é\r\nβ";
    let edits = vec![
        edit(pos(1, 0), pos(1, 1), "X"),
        edit(pos(0, 1), pos(0, 3), "Y"),
        edit(pos(0, 1), pos(0, 1), "1"),
        edit(pos(0, 1), pos(0, 1), "2"),
    ];
    assert_eq!(edit_text(source, edits).unwrap(), "a12Yé\r\nX");
    assert!(edit_text(source, vec![edit(pos(0, 2), pos(0, 3), "X")]).is_err());
    assert!(edit_text(
        source,
        vec![
            edit(pos(0, 0), pos(0, 3), "X"),
            edit(pos(0, 1), pos(0, 4), "Y"),
        ]
    )
    .is_err());
}

#[tokio::test]
async fn fake_server_handles_initialize_edit_diagnostics_format_and_shutdown() {
    let root = tempfile::tempdir().unwrap();
    let script = root.path().join("fake server.py");
    std::fs::write(&script, r#"
import json, sys, pathlib
def read(expected):
    headers = {}
    while (line := sys.stdin.buffer.readline()) != b'\r\n':
        assert line
        k, v = line.split(b':', 1)
        headers[k] = v.strip()
    frame = json.loads(sys.stdin.buffer.read(int(headers[b'Content-Length'])))
    assert frame.get('method') == expected, frame
    return frame
def send(**body):
    raw = json.dumps(dict(jsonrpc='2.0', **body)).encode()
    sys.stdout.buffer.write(f'Content-Length: {len(raw)}\r\n\r\n'.encode() + raw)
    sys.stdout.buffer.flush()
def diag(uri, version, character):
    pos = dict(line=0, character=character)
    send(method='textDocument/publishDiagnostics', params=dict(
        uri=uri, version=version, diagnostics=[dict(range=dict(start=pos, end=pos),
        severity=2, message='warning')]))
init = read('initialize')
assert init['params']['rootUri'] == pathlib.Path.cwd().as_uri()
send(id='config', method='workspace/configuration', params=dict(items=[{}, {}]))
reply = read(None)
assert reply['id'] == 'config' and reply['result'] == [None, None]
send(id='edit', method='workspace/applyEdit', params=dict(edit={}))
reply = read(None)
assert reply['id'] == 'edit' and reply['error']['code'] == -32601
send(id=init['id'], result=dict(capabilities=dict(textDocumentSync=dict(openClose=True, change=2, save=True),
    documentFormattingProvider=True)))
read('initialized')
doc = read('textDocument/didOpen')['params']['textDocument']
assert doc['languageId'] == 'rust' and doc['version'] == 5 and doc['text'] == 'a🎵'
uri = doc['uri']
change = read('textDocument/didChange')['params']
assert change['contentChanges'][0]['range']['end'] == dict(line=0, character=3)
assert change['contentChanges'][0]['text'] == 'a🎵b'
assert change['textDocument']['version'] == 6
diag(uri, 5, 3)
diag(uri, 6, 2)
diag(uri, 6, 3)
read('textDocument/didSave')
request = read('textDocument/formatting')
assert request['params']['options']['tabSize'] == 4
send(id=request['id'], result=[dict(newText='X', range=dict(start=dict(line=0, character=1),
    end=dict(line=0, character=3)))])
read('textDocument/didClose')
request = read('shutdown')
send(id=request['id'], result=None)
read('exit')
pathlib.Path('clean-exit').touch()
"#).unwrap();
    let command = shlex::try_join(["python3", "-u", script.to_str().unwrap()]).unwrap();
    let mut client = Client::start(command, root.path().to_path_buf(), "rust".into()).unwrap();
    let uri = file_uri(&root.path().join("hello #.rs")).unwrap();
    assert!(uri.ends_with("/hello%20%23.rs"));
    assert!(client.send(Command::Open {
        uri: uri.clone(),
        version: 5,
        text: Arc::from("a🎵")
    }));
    assert!(client.send(Command::Change {
        uri: uri.clone(),
        version: 6,
        text: Arc::from("a🎵b")
    }));
    assert!(client.send(Command::Save { uri: uri.clone() }));
    assert!(client.send(Command::Format {
        uri: uri.clone(),
        version: 6,
        tab_size: 4
    }));
    assert!(matches!(
        next_event(&mut client).await,
        Event::Ready { formatting: true }
    ));
    match next_event(&mut client).await {
        Event::Diagnostics {
            version: 6,
            diagnostics,
            ..
        } => assert!(diagnostics.is_empty()),
        event => panic!("unexpected {event:?}"),
    }
    match next_event(&mut client).await {
        Event::Diagnostics {
            version: 6,
            diagnostics,
            ..
        } => {
            assert_eq!(diagnostics[0].message, "warning");
        }
        event => panic!("unexpected {event:?}"),
    }
    match next_event(&mut client).await {
        Event::Formatted {
            version: 6, result, ..
        } => assert_eq!(result.unwrap(), "aXb"),
        event => panic!("unexpected {event:?}"),
    }
    assert!(client.send(Command::Close { uri }));
    drop(client);
    time::timeout(Duration::from_secs(3), async {
        while !root.path().join("clean-exit").exists() {
            time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn external_formatter_uses_snapshot_and_preserves_original_on_failure() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("real file.js");
    std::fs::write(&original, "on disk").unwrap();
    let script = root.path().join("formatter.py");
    std::fs::write(
        &script,
        r#"
from pathlib import Path
import sys
p = Path(sys.argv[1])
assert p.suffix == '.js' and p.name != 'real file.js' and p.read_text() == 'unsaved'
p.write_text('formatted')
sys.exit(int(sys.argv[2]))
"#,
    )
    .unwrap();
    for code in ["0", "1"] {
        let command =
            shlex::try_join(["python3", script.to_str().unwrap(), "%FILE%", code]).unwrap();
        let result = format_external(&command, root.path(), &original, "unsaved").await;
        if code == "0" {
            assert_eq!(result.unwrap(), "formatted");
        } else {
            assert!(result.is_err());
        }
        assert_eq!(std::fs::read_to_string(&original).unwrap(), "on disk");
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 2);
    }
}

async fn next_event(client: &mut Client) -> Event {
    time::timeout(Duration::from_secs(8), async {
        loop {
            if let Some(event) = client.next_event() {
                return event;
            }
            time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}
