//! Testes de integração do **wire**: sobem um listener Unix num diretório
//! temporário, rodam o binário `gsh` de verdade contra ele e conferem, byte a
//! byte, a linha que chega ao "app" e o que sai no stdout/stderr/exit code.
//!
//! É o contrato que o app do outro lado enxerga. Os vetores esperados foram
//! conferidos contra a CLI original (repo Flutter, `cli/`).

#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_gsh");

/// Diretório temporário exclusivo (os testes rodam em paralelo).
fn unique_dir(prefix: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "{prefix}-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir.canonicalize().unwrap()
}

struct Run {
    /// Linhas recebidas pelo "app", cruas (com a quebra de linha final).
    lines: Vec<String>,
    stdout: String,
    stderr: String,
    code: i32,
    /// HOME isolado do processo (para conferir a fazenda).
    home: PathBuf,
}

impl Run {
    fn line(&self) -> &str {
        assert_eq!(
            self.lines.len(),
            1,
            "esperava uma conexão: {:?}",
            self.lines
        );
        &self.lines[0]
    }
}

/// Sobe o "app": aceita uma conexão por item de `responses`, lê UMA linha de
/// cada e responde (ou não, quando `None`, como o socket de status do hook).
fn spawn_app(
    listener: UnixListener,
    responses: Vec<Option<String>>,
) -> std::thread::JoinHandle<Vec<String>> {
    std::thread::spawn(move || {
        let mut lines = Vec::new();
        for response in responses {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if let Some(resp) = response {
                let mut stream = reader.into_inner();
                stream.write_all(resp.as_bytes()).unwrap();
                stream.write_all(b"\n").unwrap();
                stream.flush().unwrap();
            }
            lines.push(line);
        }
        lines
    })
}

struct Setup<'a> {
    args: &'a [&'a str],
    stdin: Option<&'a str>,
    /// Envs extras (`None` remove a variável).
    env: &'a [(&'a str, Option<&'a str>)],
    /// Respostas do app, uma por conexão esperada.
    responses: Vec<Option<String>>,
    /// Arquivos a criar sob o HOME antes de rodar (caminho relativo, conteúdo).
    files: &'a [(&'a str, &'a str)],
}

impl<'a> Setup<'a> {
    fn new(args: &'a [&'a str]) -> Self {
        Setup {
            args,
            stdin: None,
            env: &[],
            responses: Vec::new(),
            files: &[],
        }
    }
    fn reply(mut self, resp: &str) -> Self {
        self.responses.push(Some(resp.to_string()));
        self
    }
    fn no_reply(mut self) -> Self {
        self.responses.push(None);
        self
    }
    fn stdin(mut self, s: &'a str) -> Self {
        self.stdin = Some(s);
        self
    }
    fn env(mut self, env: &'a [(&'a str, Option<&'a str>)]) -> Self {
        self.env = env;
        self
    }
    fn files(mut self, files: &'a [(&'a str, &'a str)]) -> Self {
        self.files = files;
        self
    }

    fn run(self) -> Run {
        let home = unique_dir("gsh-wire");
        for (rel, content) in self.files {
            let path = home.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
        let sock = home.join("app.sock");
        let listener = UnixListener::bind(&sock).unwrap();
        let app = spawn_app(listener, self.responses);

        let mut cmd = Command::new(BIN);
        cmd.args(self.args)
            .current_dir(&home)
            .env("HOME", &home)
            .env("GSH_STATUS_SOCK", &sock)
            .env("GSH_TAB_ID", "t7")
            .env_remove("GSH_PANE_ID")
            .env_remove("GSH_STATUS_PORT")
            .env_remove("GSH_STATUS_TOKEN")
            .stdin(if self.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (k, v) in self.env {
            match v {
                Some(v) => cmd.env(k, v),
                None => cmd.env_remove(k),
            };
        }
        let mut child = cmd.spawn().unwrap();
        if let Some(input) = self.stdin {
            child
                .stdin
                .take()
                .unwrap()
                .write_all(input.as_bytes())
                .unwrap();
        }
        let out = child.wait_with_output().unwrap();
        let lines = app.join().unwrap();
        Run {
            lines,
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            code: out.status.code().unwrap_or(-1),
            home,
        }
    }
}

impl Drop for Run {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

/// Roda sem nenhum app (nenhuma conexão esperada).
fn run_offline(args: &[&str], env: &[(&str, Option<&str>)]) -> (String, String, i32, PathBuf) {
    let home = unique_dir("gsh-offline");
    let mut cmd = Command::new(BIN);
    cmd.args(args)
        .current_dir(&home)
        .env("HOME", &home)
        .env_remove("GSH_STATUS_SOCK")
        .env_remove("GSH_STATUS_PORT")
        .env_remove("GSH_STATUS_TOKEN")
        .env_remove("GSH_TAB_ID")
        .env_remove("GSH_PANE_ID")
        .stdin(Stdio::null());
    for (k, v) in env {
        match v {
            Some(v) => cmd.env(k, v),
            None => cmd.env_remove(k),
        };
    }
    let out = cmd.output().unwrap();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
        home,
    )
}

fn path_str(p: &Path) -> &str {
    p.to_str().unwrap()
}

// ---- --version / --help -----------------------------------------------------

#[test]
fn version_imprime_gsh_e_a_versao() {
    let want = format!("gsh {}\n", env!("CARGO_PKG_VERSION"));
    for flag in ["--version", "-v"] {
        let (out, err, code, home) = run_offline(&[flag], &[]);
        assert_eq!((out.as_str(), err.as_str(), code), (want.as_str(), "", 0));
        let _ = std::fs::remove_dir_all(home);
    }
}

#[test]
fn sem_argumentos_manda_a_ajuda_para_o_stderr_com_exit_2() {
    let (out, err, code, home) = run_offline(&[], &[]);
    assert_eq!(out, "");
    assert_eq!(err, include_str!("../text/help.txt"));
    assert_eq!(code, 2);
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn help_vai_para_o_stdout_com_exit_0() {
    for flag in ["--help", "-h", "help"] {
        let (out, err, code, home) = run_offline(&[flag], &[]);
        assert_eq!(out, include_str!("../text/help.txt"));
        assert_eq!((err.as_str(), code), ("", 0));
        let _ = std::fs::remove_dir_all(home);
    }
}

// ---- msg --------------------------------------------------------------------

#[test]
fn msg_manda_a_linha_exata_e_imprime_o_destino() {
    let r = Setup::new(&["msg", "--to", "Reviewer", "diff", "ready"])
        .reply(r#"{"ok":true,"data":{"from":"t7","to":"t9"}}"#)
        .run();
    assert_eq!(
        r.line(),
        "{\"args\":{\"text\":\"diff ready\",\"to\":\"Reviewer\"},\"cmd\":\"msg\",\"tabId\":\"t7\",\"type\":\"cmd\"}\n"
    );
    assert_eq!(
        (r.stdout.as_str(), r.stderr.as_str(), r.code),
        ("t9\n", "", 0)
    );
}

#[test]
fn msg_json_ecoa_o_data_compacto() {
    let r = Setup::new(&["msg", "--json", "--to", "a", "oi"])
        .reply(r#"{"ok":true,"data":{"from":"t7","to":"t9"}}"#)
        .run();
    assert_eq!(r.stdout, "{\"from\":\"t7\",\"to\":\"t9\"}\n");
    assert_eq!(r.code, 0);
}

#[test]
fn msg_data_sem_to_imprime_linha_vazia() {
    let r = Setup::new(&["msg", "--to", "a", "oi"])
        .reply(r#"{"ok":true}"#)
        .run();
    assert_eq!((r.stdout.as_str(), r.code), ("\n", 0));
}

#[test]
fn msg_sem_tab_id_e_erro_de_uso_sem_conectar() {
    let (out, err, code, home) = run_offline(&["msg", "--to", "a", "oi"], &[]);
    assert_eq!(out, "");
    assert_eq!(
        err,
        "gsh msg: GSH_TAB_ID is unset — run it inside a GSH terminal (the sender is the current tab)\n"
    );
    assert_eq!(code, 2);
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn msg_cai_no_gsh_pane_id_legado() {
    let r = Setup::new(&["msg", "--to", "a", "oi"])
        .env(&[("GSH_TAB_ID", None), ("GSH_PANE_ID", Some("t4"))])
        .reply(r#"{"ok":true,"data":{"to":"a"}}"#)
        .run();
    assert!(r.line().contains(r#""tabId":"t4""#), "{}", r.line());
}

// ---- deliver ----------------------------------------------------------------

#[test]
fn deliver_manda_caminho_absoluto_e_a_nota() {
    let r = Setup::new(&["deliver", "--to", "Bianca/fin", "r.pdf", "weekly", "report"])
        .reply(r#"{"ok":true,"data":{"to":"Bianca/fin"}}"#)
        .run();
    let expected = format!(
        "{{\"args\":{{\"path\":\"{}/r.pdf\",\"text\":\"weekly report\",\"to\":\"Bianca/fin\"}},\"cmd\":\"deliver\",\"tabId\":\"t7\",\"type\":\"cmd\"}}\n",
        path_str(&r.home)
    );
    assert_eq!(r.line(), expected);
    assert_eq!((r.stdout.as_str(), r.code), ("Bianca/fin\n", 0));
}

#[test]
fn deliver_sem_nota_manda_text_vazio_e_absoluto_fica_intacto() {
    let r = Setup::new(&["deliver", "--to", "B/x", "/abs/a.md"])
        .reply(r#"{"ok":true,"data":{"to":"B/x"}}"#)
        .run();
    assert_eq!(
        r.line(),
        "{\"args\":{\"path\":\"/abs/a.md\",\"text\":\"\",\"to\":\"B/x\"},\"cmd\":\"deliver\",\"tabId\":\"t7\",\"type\":\"cmd\"}\n"
    );
}

// ---- new-tab ----------------------------------------------------------------

#[test]
fn new_tab_manda_cwd_title_e_split_e_imprime_o_id() {
    let r = Setup::new(&[
        "new-tab", "--cwd", "/work", "--title", "Worker", "--split", "H",
    ])
    .reply(r#"{"ok":true,"data":{"tabId":"t12"}}"#)
    .run();
    assert_eq!(
        r.line(),
        "{\"args\":{\"cwd\":\"/work\",\"split\":\"right\",\"title\":\"Worker\"},\"cmd\":\"new-tab\",\"tabId\":\"t7\",\"type\":\"cmd\"}\n"
    );
    assert_eq!((r.stdout.as_str(), r.code), ("t12\n", 0));
}

#[test]
fn new_tab_sem_flags_usa_o_cwd_do_processo_e_nao_manda_title_nem_split() {
    let r = Setup::new(&["new-tab", "--json"])
        .reply(r#"{"ok":true,"data":{"tabId":"t12","kind":"terminal"}}"#)
        .run();
    let expected = format!(
        "{{\"args\":{{\"cwd\":\"{}\"}},\"cmd\":\"new-tab\",\"tabId\":\"t7\",\"type\":\"cmd\"}}\n",
        path_str(&r.home)
    );
    assert_eq!(r.line(), expected);
    assert_eq!(r.stdout, "{\"kind\":\"terminal\",\"tabId\":\"t12\"}\n");
}

#[test]
fn new_tab_tab_id_explicito_vence_o_ambiente() {
    let r = Setup::new(&["new-tab", "--cwd=/w", "--tab-id=t99", "--split", "down"])
        .reply(r#"{"ok":true,"data":{"tabId":"t1"}}"#)
        .run();
    assert_eq!(
        r.line(),
        "{\"args\":{\"cwd\":\"/w\",\"split\":\"down\"},\"cmd\":\"new-tab\",\"tabId\":\"t99\",\"type\":\"cmd\"}\n"
    );
}

#[test]
fn new_tab_split_invalido_e_erro_de_uso_sem_conectar() {
    let (out, err, code, home) = run_offline(&["new-tab", "--split", "Diag"], &[]);
    assert_eq!(out, "");
    assert_eq!(
        err,
        "gsh new-tab: invalid --split \"Diag\" (use h|right or v|down)\n"
    );
    assert_eq!(code, 2);
    let _ = std::fs::remove_dir_all(home);
}

// ---- close-tab --------------------------------------------------------------

#[test]
fn close_tab_sem_alvo_nao_manda_target() {
    let r = Setup::new(&["close-tab"])
        .reply(r#"{"ok":true,"data":{"tabId":"t7"}}"#)
        .run();
    assert_eq!(
        r.line(),
        "{\"args\":{},\"cmd\":\"close-tab\",\"tabId\":\"t7\",\"type\":\"cmd\"}\n"
    );
    assert_eq!((r.stdout.as_str(), r.code), ("t7\n", 0));
}

#[test]
fn close_tab_com_label_manda_target() {
    let r = Setup::new(&["close-tab", "Extension"])
        .reply(r#"{"ok":true,"data":{"tabId":"t3"}}"#)
        .run();
    assert_eq!(
        r.line(),
        "{\"args\":{\"target\":\"Extension\"},\"cmd\":\"close-tab\",\"tabId\":\"t7\",\"type\":\"cmd\"}\n"
    );
    assert_eq!(r.stdout, "t3\n");
}

#[test]
fn close_tab_focused_manda_o_sentinela_e_json_ecoa_o_data() {
    let r = Setup::new(&["close-tab", "--focused", "--json"])
        .reply(r#"{"ok":true,"data":{"tabId":"t3"}}"#)
        .run();
    assert_eq!(
        r.line(),
        "{\"args\":{},\"cmd\":\"close-tab\",\"tabId\":\"@focused\",\"type\":\"cmd\"}\n"
    );
    assert_eq!(r.stdout, "{\"tabId\":\"t3\"}\n");
}

// ---- list-tabs --------------------------------------------------------------

#[test]
fn list_tabs_usa_o_comando_de_wire_legado_e_formata_as_colunas() {
    let resp = r#"{"ok":true,"data":[
        {"id":"t1","kind":"terminal","title":"zsh","label":"Worker","workspacePath":"/x/remote_pi","working":true,"harness":"claudeCode"},
        {"id":"t12","kind":"viewer","title":"a.md","workspaceId":"w1"},
        7
    ]}"#;
    let r = Setup::new(&["list-tabs"]).reply(resp).run();
    assert_eq!(
        r.line(),
        "{\"cmd\":\"list-panes\",\"tabId\":\"t7\",\"type\":\"cmd\"}\n"
    );
    assert_eq!(
        r.stdout,
        "● t1     terminal  remote_pi      ⚲ Worker  [claudeCode]\n  t12    viewer    w1             a.md\n"
    );
    assert_eq!(r.code, 0);
}

#[test]
fn list_panes_e_alias_e_focused_vai_no_tab_id() {
    let r = Setup::new(&["list-panes", "--focused"])
        .reply(r#"{"ok":true,"data":[]}"#)
        .run();
    assert_eq!(
        r.line(),
        "{\"cmd\":\"list-panes\",\"tabId\":\"@focused\",\"type\":\"cmd\"}\n"
    );
    assert_eq!(r.stdout, "(none)\n");
}

#[test]
fn list_tabs_json_e_pretty_com_dois_espacos() {
    let r = Setup::new(&["list-tabs", "--json"])
        .reply(r#"{"ok":true,"data":[{"id":"t1"}]}"#)
        .run();
    assert_eq!(r.stdout, "[\n  {\n    \"id\": \"t1\"\n  }\n]\n");
    let r = Setup::new(&["list-tabs", "--json"])
        .reply(r#"{"ok":true}"#)
        .run();
    assert_eq!(r.stdout, "[]\n");
}

#[test]
fn list_tabs_so_com_entradas_que_nao_sao_objeto_nao_imprime_nada() {
    let r = Setup::new(&["list-tabs"])
        .reply(r#"{"ok":true,"data":[1,"x"]}"#)
        .run();
    assert_eq!((r.stdout.as_str(), r.code), ("", 0));
}

// ---- erros e transporte -----------------------------------------------------

#[test]
fn erro_do_app_vira_exit_1_no_stderr() {
    let r = Setup::new(&["list-tabs"])
        .reply(r#"{"ok":false,"error":"boom"}"#)
        .run();
    assert_eq!(
        (r.stdout.as_str(), r.stderr.as_str(), r.code),
        ("", "gsh: boom\n", 1)
    );
}

#[test]
fn resposta_ausente_ou_malformada_e_erro_de_aplicacao() {
    // O app aceita a conexão e fecha sem responder.
    let r = Setup::new(&["close-tab"]).no_reply().run();
    assert_eq!(
        (r.stderr.as_str(), r.code),
        ("gsh: no response from app\n", 1)
    );
    let r = Setup::new(&["close-tab"]).reply("[1]").run();
    assert_eq!(
        (r.stderr.as_str(), r.code),
        ("gsh: malformed response\n", 1)
    );
    let r = Setup::new(&["close-tab"]).reply(r#"{"ok":false}"#).run();
    assert_eq!((r.stderr.as_str(), r.code), ("gsh: failed\n", 1));
}

#[test]
fn aviso_do_app_vai_para_o_stderr_e_nunca_para_o_stdout() {
    let r = Setup::new(&["close-tab", "--json"])
        .reply(r#"{"ok":true,"data":{"tabId":"t3"},"warning":"other machine"}"#)
        .run();
    assert_eq!(r.stdout, "{\"tabId\":\"t3\"}\n");
    assert_eq!(r.stderr, "gsh: other machine\n");
}

#[test]
fn token_do_ambiente_entra_como_tok() {
    let r = Setup::new(&["close-tab"])
        .env(&[("GSH_STATUS_TOKEN", Some("s3cret"))])
        .reply(r#"{"ok":true,"data":{"tabId":"t7"}}"#)
        .run();
    assert_eq!(
        r.line(),
        "{\"args\":{},\"cmd\":\"close-tab\",\"tabId\":\"t7\",\"tok\":\"s3cret\",\"type\":\"cmd\"}\n"
    );
}

#[test]
fn acha_o_socket_bem_conhecido_sem_env_nenhuma() {
    let home = unique_dir("gsh-discovery");
    std::fs::create_dir_all(home.join(".gsh")).unwrap();
    let listener = UnixListener::bind(home.join(".gsh/status.sock")).unwrap();
    let app = spawn_app(listener, vec![Some(r#"{"ok":true,"data":[]}"#.into())]);
    let out = Command::new(BIN)
        .args(["list-tabs", "--focused"])
        .env("HOME", &home)
        .env_remove("GSH_STATUS_SOCK")
        .env_remove("GSH_STATUS_PORT")
        .env_remove("GSH_TAB_ID")
        .env_remove("GSH_PANE_ID")
        .output()
        .unwrap();
    let lines = app.join().unwrap();
    let _ = std::fs::remove_dir_all(&home);
    assert_eq!(
        lines[0],
        "{\"cmd\":\"list-panes\",\"tabId\":\"@focused\",\"type\":\"cmd\"}\n"
    );
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn sem_app_nenhum_falha_com_exit_3() {
    let (out, err, code, home) = run_offline(&["list-tabs"], &[]);
    assert_eq!(out, "");
    assert_eq!(
        err,
        "gsh: no GSH app to talk to (GSH_STATUS_SOCK is unset and no socket found in ~/.gsh). Is the app running?\n"
    );
    assert_eq!(code, 3);
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn socket_orfao_recusa_conexao_com_exit_3() {
    // O arquivo existe mas ninguém escuta: quem decide é o connect.
    let home = unique_dir("gsh-orphan");
    let sock = home.join("dead.sock");
    drop(UnixListener::bind(&sock).unwrap());
    let out = Command::new(BIN)
        .arg("list-tabs")
        .env("HOME", &home)
        .env("GSH_STATUS_SOCK", &sock)
        .env_remove("GSH_STATUS_PORT")
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&home);
    assert_eq!(out.status.code(), Some(3));
    assert!(
        String::from_utf8_lossy(&out.stderr).starts_with("gsh: could not connect to app: "),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn verbo_ainda_nao_portado_e_erro_de_uso() {
    let (out, err, code, home) = run_offline(&["version"], &[]);
    assert_eq!(
        (out.as_str(), err.as_str(), code),
        ("", "gsh: unknown verb: version\n", 2)
    );
    let _ = std::fs::remove_dir_all(home);
}

// ---- hook -------------------------------------------------------------------

fn hook_env<'a>() -> Vec<(&'a str, Option<&'a str>)> {
    vec![("GSH_TAB_ID", None), ("GSH_PANE_ID", Some("t3"))]
}

#[test]
fn hook_manda_a_linha_de_status_exata_sem_type_e_grava_a_fazenda() {
    let env = hook_env();
    let r = Setup::new(&["hook"])
        .env(&env)
        .stdin(r#"{"hook_event_name":"Stop","session_id":"s1","transcript_path":"/t/s1.jsonl"}"#)
        .no_reply()
        .run();
    assert_eq!(
        r.line(),
        "{\"ev\":\"Stop\",\"hn\":\"claude\",\"paneId\":\"t3\",\"sid\":\"s1\",\"st\":\"idle\",\"tx\":\"/t/s1.jsonl\"}\n"
    );
    assert_eq!((r.stdout.as_str(), r.stderr.as_str(), r.code), ("", "", 0));
    let farm: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(r.home.join(".gsh/farm/s1.json")).unwrap())
            .unwrap();
    assert_eq!(farm["st"], "idle");
    assert_eq!(farm["ev"], "Stop");
    assert_eq!(farm["hn"], "claude");
    assert!(farm["ts"].as_u64().unwrap() > 0);
    // Nada de .tmp sobrando.
    let left: Vec<_> = std::fs::read_dir(r.home.join(".gsh/farm"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left, ["s1.json"]);
}

#[test]
fn hook_codex_leva_turn_id_arquivos_tocados_e_token() {
    let env = vec![
        ("GSH_TAB_ID", None),
        ("GSH_PANE_ID", Some("t3")),
        ("GSH_STATUS_TOKEN", Some("tk")),
    ];
    let r = Setup::new(&["hook", "--harness", "codex"])
        .env(&env)
        .stdin(
            r#"{"hook_event_name":"PostToolUse","session_id":"s2","transcript_path":"","turn_id":"u9","tool_name":"Edit","cwd":"/repo","tool_input":{"file_path":"src/a.rs"}}"#,
        )
        .no_reply()
        .run();
    assert_eq!(
        r.line(),
        "{\"ev\":\"PostToolUse\",\"files\":[\"/repo/src/a.rs\"],\"hn\":\"codex\",\"paneId\":\"t3\",\"sid\":\"s2\",\"st\":\"working\",\"tid\":\"u9\",\"tok\":\"tk\",\"tx\":\"\"}\n"
    );
    assert_eq!((r.stdout.as_str(), r.code), ("", 0));
}

#[test]
fn hook_session_end_apaga_a_fazenda_e_manda_idle() {
    let env = hook_env();
    let r = Setup::new(&["hook"])
        .env(&env)
        .files(&[(".gsh/farm/s3.json", "{}")])
        .stdin(r#"{"hook_event_name":"SessionEnd","session_id":"s3"}"#)
        .no_reply()
        .run();
    assert!(r.line().contains(r#""st":"idle""#), "{}", r.line());
    assert!(!r.home.join(".gsh/farm/s3.json").exists());
}

#[test]
fn hook_pretooluse_com_host_protegido_bloqueia_com_exit_2() {
    // Nenhuma conexão esperada: o bloqueio vem antes de qualquer socket.
    let env = hook_env();
    let r = Setup::new(&["hook"])
        .env(&env)
        .files(&[(".gsh/protected-hosts.json", r#"[" Prod "]"#)])
        .stdin(
            r#"{"hook_event_name":"PreToolUse","session_id":"s4","tool_name":"Bash","tool_input":{"command":"ssh prod"}}"#,
        )
        .run();
    assert!(r.lines.is_empty());
    assert_eq!(r.stdout, "");
    assert_eq!(
        r.stderr,
        "GSH Desktop bloqueou este comando: ele cita o host protegido \"prod\". Peça ao usuário para rodar à mão ou remover o host em Configurações > Claude Code > Segurança.\n"
    );
    assert_eq!(r.code, 2);
}

#[test]
fn hook_permission_request_do_claude_imprime_a_saida_montada_pelo_app() {
    let env = hook_env();
    let r = Setup::new(&["hook"])
        .env(&env)
        .stdin(
            r#"{"hook_event_name":"PermissionRequest","session_id":"s5","tool_name":"Bash","tool_input":{"command":"ls"}}"#,
        )
        .no_reply()
        .reply(r#"{"ok":true,"data":{"output":{"hookSpecificOutput":{"decision":"allow"}}}}"#)
        .run();
    assert_eq!(r.lines.len(), 2, "{:?}", r.lines);
    assert!(r.lines[0].contains(r#""st":"waiting""#), "{}", r.lines[0]);
    assert_eq!(
        r.lines[1],
        "{\"args\":{\"permission_suggestions\":null,\"tool_input\":{\"command\":\"ls\"},\"tool_name\":\"Bash\"},\"cmd\":\"permission-ask\",\"tabId\":\"t3\",\"type\":\"cmd\"}\n"
    );
    assert_eq!(
        r.stdout,
        "{\"hookSpecificOutput\":{\"decision\":\"allow\"}}\n"
    );
    assert_eq!((r.stderr.as_str(), r.code), ("", 0));
}

#[test]
fn hook_permission_request_sem_output_ou_de_outro_harness_nao_imprime_nada() {
    let env = hook_env();
    let ev = r#"{"hook_event_name":"PermissionRequest","session_id":"s6","tool_name":"Bash"}"#;
    let r = Setup::new(&["hook"])
        .env(&env)
        .stdin(ev)
        .no_reply()
        .reply(r#"{"ok":true,"data":{"output":null}}"#)
        .run();
    assert_eq!((r.stdout.as_str(), r.code), ("", 0));
    // Codex: só o status, sem permission-ask.
    let r = Setup::new(&["hook", "--harness=codex"])
        .env(&env)
        .stdin(ev)
        .no_reply()
        .run();
    assert_eq!(r.lines.len(), 1);
    assert_eq!((r.stdout.as_str(), r.code), ("", 0));
}

#[test]
fn hook_fora_do_gsh_e_noop_silencioso_mas_ainda_grava_a_fazenda() {
    let (out, err, code, home) =
        run_offline_stdin(r#"{"hook_event_name":"Stop","session_id":"s7"}"#);
    assert_eq!((out.as_str(), err.as_str(), code), ("", "", 0));
    assert!(home.join(".gsh/farm/s7.json").exists());
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn hook_com_stdin_vazio_ou_invalido_sai_0_em_silencio() {
    for input in ["", "  \n", "nao e json", "[1]"] {
        let (out, err, code, home) = run_offline_stdin(input);
        assert_eq!((out.as_str(), err.as_str(), code), ("", "", 0), "{input:?}");
        assert!(!home.join(".gsh").exists(), "{input:?}");
        let _ = std::fs::remove_dir_all(home);
    }
}

#[test]
fn hook_evento_sem_status_nao_conecta() {
    let env = hook_env();
    let r = Setup::new(&["hook"])
        .env(&env)
        .stdin(r#"{"hook_event_name":"SubagentStart","session_id":"s8"}"#)
        .run();
    assert!(r.lines.is_empty());
    assert_eq!((r.stdout.as_str(), r.stderr.as_str(), r.code), ("", "", 0));
}

/// `gsh hook` sem env de GSH, com stdin.
fn run_offline_stdin(input: &str) -> (String, String, i32, PathBuf) {
    let home = unique_dir("gsh-hook-offline");
    let mut child = Command::new(BIN)
        .arg("hook")
        .env("HOME", &home)
        .env_remove("GSH_STATUS_SOCK")
        .env_remove("GSH_STATUS_PORT")
        .env_remove("GSH_STATUS_TOKEN")
        .env_remove("GSH_PANE_ID")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
        home,
    )
}
