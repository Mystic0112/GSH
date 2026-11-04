//! Parse de argumentos, manual como na CLI original (não é `clap`): a saída de
//! erro e os códigos de saída têm que ficar idênticos, e quem lê é agente e
//! script. O parse não lê ambiente nem disco; resolver `GSH_TAB_ID`, cwd e
//! caminhos é da execução ([`crate::commands`]).

use gsh_core::wire::FOCUSED;

use crate::error::CliError;

pub const HELP: &str = include_str!("../text/help.txt");

pub const MSG_HELP: &str = "gsh msg --to <label|tab-id> [--json] <text...>
  Messages another agent: types `[from: <your tab> · <your harness>] <text>`
  into the target tab and presses Enter. The sender is this tab
  ($GSH_TAB_ID), so it only works inside a GSH terminal.
  --to   stable tab label (exact, case-insensitive) or tab-id; an ambiguous
         label is an error (see `gsh list-tabs`). A teammate's agent on the
         office network is `<Name>/<agent>` (see `gsh peers`): it knocks on
         their door and their agent decides what to do with it.
  Prints the target tab id (--json: {\"from\": …, \"to\": …}).";

pub const DELIVER_HELP: &str = "gsh deliver --to <Name/agent> [--json] <file> [note...]
  Delivers a file (report, .md, .pdf...) to a teammate's agent on the office
  network: it lands in `.gsh/inbox/<your name>/` inside that agent's folder
  and the agent is told it arrived (with the optional note). Needs the team
  farm on with the same team name and password on both sides (see
  `gsh peers` for the addresses). Max 25 MB.";

pub const NEW_TAB_HELP: &str = "gsh new-tab [--cwd <dir>] [--title <name>] [--split h|v]
  --cwd    working directory (default: current directory)
  --title  stable tab label (read-tab/send can target it)
  --split  h|right = side by side, v|down = stacked; omit = new tab
           in the same pane
  Prints the new tab id (e.g. t12).";

pub const CLOSE_TAB_HELP: &str = "gsh close-tab [<label|tab-id>] [--json]
  Closes a tab (the counterpart of `new-tab`). Without a target, closes the
  CURRENT tab — which ends the shell you are typing in.
  A target may be a stable tab label or a tab-id (see `gsh list-tabs`).
  Same behaviour as the tab's \"x\" in the UI: closing the last tab of a split
  removes the split; closing the last tab of a workspace leaves an empty tab.
  Prints the id of the closed tab.";

/// Qual aba o comando mira. A resolução final precisa do ambiente
/// (`GSH_TAB_ID`), por isso fica pendente até a execução.
#[derive(Debug, Default, PartialEq)]
pub struct Target {
    pub focused: bool,
    pub tab_id: Option<String>,
}

impl Target {
    /// Na ordem: `--focused` (sentinela que o app resolve), `--tab-id`
    /// explícito, e por fim a aba que emitiu o comando (`own`).
    pub fn resolve(&self, own: Option<String>) -> Option<String> {
        if self.focused {
            return Some(FOCUSED.to_string());
        }
        self.tab_id.clone().or(own)
    }
}

/// Flags comuns aos comandos simples, incluindo as de verbos que ainda não
/// foram portados: o parser tem que consumir exatamente o que o original
/// consome (uma flag desconhecida vira posicional, ou seja, texto).
#[derive(Debug, Default, PartialEq)]
#[allow(dead_code)] // ponytail: lines/offset/from_start/enter/force são lidos por verbos ainda não portados
pub struct Flags {
    pub positionals: Vec<String>,
    pub tab_id: Option<String>,
    pub json: bool,
    pub force: bool,
    pub lines: Option<u64>,
    pub offset: Option<u64>,
    pub from_start: bool,
    pub enter: bool,
    pub focused: bool,
}

fn int_value(flag: &str, raw: &str) -> Result<u64, CliError> {
    raw.parse::<u64>()
        .map_err(|_| CliError::usage(format!("gsh: {flag} requires a non-negative integer")))
}

/// Valor separado de uma flag (`--flag valor`), avançando o índice.
fn next_value<'a>(args: &'a [String], i: &mut usize, flag: &str) -> Result<&'a str, CliError> {
    *i += 1;
    args.get(*i)
        .map(String::as_str)
        .ok_or_else(|| CliError::usage(format!("gsh: {flag} requires a value")))
}

impl Flags {
    pub fn parse(args: &[String]) -> Result<Flags, CliError> {
        let mut f = Flags::default();
        let mut i = 0usize;
        while let Some(a) = args.get(i).map(String::as_str) {
            if a == "--lines" || a == "-n" {
                f.lines = Some(int_value("--lines", next_value(args, &mut i, "--lines")?)?);
            } else if let Some(v) = a.strip_prefix("--lines=") {
                f.lines = Some(int_value("--lines", v)?);
            } else if a == "--offset" {
                f.offset = Some(int_value(
                    "--offset",
                    next_value(args, &mut i, "--offset")?,
                )?);
            } else if let Some(v) = a.strip_prefix("--offset=") {
                f.offset = Some(int_value("--offset", v)?);
            } else if a == "--from-start" {
                f.from_start = true;
            } else if a == "--tab-id" || a == "-t" {
                f.tab_id = Some(next_value(args, &mut i, "--tab-id")?.to_string());
            } else if let Some(v) = a.strip_prefix("--tab-id=") {
                f.tab_id = Some(v.to_string());
            } else if a == "--enter" {
                f.enter = true;
            } else if a == "--focused" {
                f.focused = true;
            } else if a == "--json" {
                f.json = true;
            } else if a == "--force" || a == "-f" {
                f.force = true;
            } else if a == "--" {
                f.positionals
                    .extend(args.get(i + 1..).unwrap_or_default().iter().cloned());
                break;
            } else {
                f.positionals.push(a.to_string());
            }
            i += 1;
        }
        Ok(f)
    }

    pub fn target(&self) -> Target {
        Target {
            focused: self.focused,
            tab_id: self.tab_id.clone(),
        }
    }
}

/// Lê `--flag valor` ou `--flag=valor` na posição atual, avançando o índice ao
/// consumir o valor separado. Devolve `(flag, valor)` quando casa; `--flag` no
/// fim dos argumentos casa com valor `None`.
fn take<'a>(
    args: &[String],
    i: &mut usize,
    flags: &[&'a str],
) -> Option<(&'a str, Option<String>)> {
    let a = args.get(*i)?.as_str();
    for flag in flags {
        if a == *flag {
            *i += 1;
            return Some((flag, args.get(*i).cloned()));
        }
        if let Some(v) = a.strip_prefix(*flag).and_then(|r| r.strip_prefix('=')) {
            return Some((flag, Some(v.to_string())));
        }
    }
    None
}

/// `--to` sai dos argumentos; o resto vai pro [`Flags::parse`] (texto + `--json`).
fn split_to(args: &[String]) -> (Option<String>, Vec<String>) {
    let mut to = None;
    let mut rest = Vec::new();
    let mut i = 0usize;
    while let Some(a) = args.get(i) {
        if a == "--" {
            rest.extend(args.get(i..).unwrap_or_default().iter().cloned());
            break;
        }
        match take(args, &mut i, &["--to"]) {
            Some((_, v)) => to = v,
            None => rest.push(a.clone()),
        }
        i += 1;
    }
    (to, rest)
}

#[derive(Debug, PartialEq)]
pub enum Split {
    Right,
    Down,
}

impl Split {
    pub fn wire(&self) -> &'static str {
        match self {
            Split::Right => "right",
            Split::Down => "down",
        }
    }
}

/// O que a linha de comando pede. Montado só com os argumentos.
#[derive(Debug, PartialEq)]
pub enum Command {
    /// Texto de ajuda para o stdout (exit 0). Termina em uma quebra de linha só.
    Help(&'static str),
    Version,
    Hook {
        harness: String,
    },
    Msg {
        to: String,
        text: String,
        json: bool,
    },
    Deliver {
        to: String,
        file: String,
        note: String,
        json: bool,
    },
    NewTab {
        cwd: Option<String>,
        title: Option<String>,
        split: Option<Split>,
        target: Target,
        json: bool,
    },
    CloseTab {
        label: Option<String>,
        target: Target,
        json: bool,
    },
    ListTabs {
        target: Target,
        json: bool,
    },
}

pub fn parse(argv: &[String]) -> Result<Command, CliError> {
    let Some(first) = argv.first().map(String::as_str) else {
        // Sem argumentos: ajuda no stderr, exit 2 (como o original).
        return Err(CliError::usage(HELP.trim_end_matches('\n')));
    };
    let args = argv.get(1..).unwrap_or_default();
    match first {
        "--help" | "-h" | "help" => Ok(Command::Help(HELP)),
        "--version" | "-v" => Ok(Command::Version),
        "hook" => Ok(Command::Hook {
            harness: harness_from(args),
        }),
        "msg" => parse_msg(args),
        "deliver" => parse_deliver(args),
        "new-tab" => parse_new_tab(args),
        "close-tab" => parse_close_tab(args),
        // `list-panes` é o verbo legado; o comando de wire é o mesmo.
        "list-tabs" | "list-panes" => {
            let f = Flags::parse(args)?;
            Ok(Command::ListTabs {
                target: f.target(),
                json: f.json,
            })
        }
        other => Err(CliError::usage(format!("gsh: unknown verb: {other}"))),
    }
}

fn wants_help(args: &[String]) -> bool {
    args.iter().any(|a| a == "--help" || a == "-h")
}

/// `--harness <nome>` / `--harness=<nome>`. Default `claude`: entries de hook
/// instalados por versões anteriores não passam a flag, e são todos do Claude.
fn harness_from(args: &[String]) -> String {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let value = if a == "--harness" {
            it.next().map(String::as_str)
        } else {
            a.strip_prefix("--harness=")
        };
        if let Some(v) = value.map(str::trim).filter(|v| !v.is_empty()) {
            return v.to_string();
        }
    }
    "claude".to_string()
}

fn parse_msg(args: &[String]) -> Result<Command, CliError> {
    if wants_help(args) {
        return Ok(Command::Help(MSG_HELP));
    }
    let (to, rest) = split_to(args);
    let parsed = Flags::parse(&rest)?;
    let to = to
        .filter(|t| !t.is_empty())
        .ok_or_else(|| CliError::usage(format!("gsh msg: missing --to\n{MSG_HELP}")))?;
    let text = parsed.positionals.join(" ");
    if text.is_empty() {
        return Err(CliError::usage("gsh msg: missing text"));
    }
    Ok(Command::Msg {
        to,
        text,
        json: parsed.json,
    })
}

fn parse_deliver(args: &[String]) -> Result<Command, CliError> {
    if wants_help(args) {
        return Ok(Command::Help(DELIVER_HELP));
    }
    let (to, rest) = split_to(args);
    let parsed = Flags::parse(&rest)?;
    let to = to
        .filter(|t| !t.is_empty())
        .ok_or_else(|| CliError::usage(format!("gsh deliver: missing --to\n{DELIVER_HELP}")))?;
    let Some((file, note)) = parsed.positionals.split_first() else {
        return Err(CliError::usage("gsh deliver: missing file"));
    };
    Ok(Command::Deliver {
        to,
        file: file.clone(),
        note: note.join(" "),
        json: parsed.json,
    })
}

fn parse_new_tab(args: &[String]) -> Result<Command, CliError> {
    let (mut cwd, mut title, mut split, mut tab_id) = (None, None, None, None);
    let mut json = false;
    let mut i = 0usize;
    while let Some(a) = args.get(i) {
        if a == "--help" || a == "-h" {
            return Ok(Command::Help(NEW_TAB_HELP));
        }
        if a == "--json" {
            json = true;
            i += 1;
            continue;
        }
        if let Some((flag, value)) =
            take(args, &mut i, &["--cwd", "--title", "--split", "--tab-id"])
        {
            match flag {
                "--cwd" => cwd = value,
                "--title" => title = value,
                "--split" => split = value,
                _ => tab_id = value,
            }
        }
        i += 1;
    }

    let split = match split {
        None => None,
        Some(raw) => match raw.to_lowercase().as_str() {
            "h" | "horizontal" | "right" => Some(Split::Right),
            "v" | "vertical" | "down" => Some(Split::Down),
            _ => {
                return Err(CliError::usage(format!(
                    "gsh new-tab: invalid --split \"{raw}\" (use h|right or v|down)"
                )))
            }
        },
    };
    Ok(Command::NewTab {
        cwd,
        title,
        split,
        target: Target {
            focused: false,
            tab_id,
        },
        json,
    })
}

fn parse_close_tab(args: &[String]) -> Result<Command, CliError> {
    // Ordem do original: o parse das flags vem antes do `--help`.
    let parsed = Flags::parse(args)?;
    if wants_help(args) {
        return Ok(Command::Help(CLOSE_TAB_HELP));
    }
    Ok(Command::CloseTab {
        label: parsed
            .positionals
            .first()
            .cloned()
            .filter(|t| !t.is_empty()),
        target: parsed.target(),
        json: parsed.json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    fn usage_msg(r: Result<Command, CliError>) -> String {
        match r {
            Err(e @ CliError::Usage(_)) => {
                assert_eq!(e.exit_code(), 2);
                e.to_string()
            }
            other => panic!("esperava erro de uso, veio {other:?}"),
        }
    }

    #[test]
    fn posicionais_e_tab_id() {
        let f = Flags::parse(&v(&["--tab-id", "t3", "olá", "mundo"])).unwrap();
        assert_eq!(f.tab_id.as_deref(), Some("t3"));
        assert_eq!(f.positionals, vec!["olá", "mundo"]);
    }

    #[test]
    fn forma_com_igual_e_aliases() {
        let f = Flags::parse(&v(&["--tab-id=t9", "--lines=25", "--offset=5"])).unwrap();
        assert_eq!(f.tab_id.as_deref(), Some("t9"));
        assert_eq!(f.lines, Some(25));
        assert_eq!(f.offset, Some(5));
        let f = Flags::parse(&v(&["-t", "t1", "-n", "10", "-f"])).unwrap();
        assert_eq!(f.tab_id.as_deref(), Some("t1"));
        assert_eq!(f.lines, Some(10));
        assert!(f.force);
    }

    #[test]
    fn separador_manda_o_resto_pra_posicionais() {
        let f = Flags::parse(&v(&["--json", "--", "--nao-e-flag", "-x"])).unwrap();
        assert!(f.json);
        assert_eq!(f.positionals, vec!["--nao-e-flag", "-x"]);
    }

    #[test]
    fn flag_sem_valor_ou_com_valor_ruim_e_erro_de_uso() {
        for (args, msg) in [
            (&["--lines"][..], "gsh: --lines requires a value"),
            (&["-n"], "gsh: --lines requires a value"),
            (&["--offset"], "gsh: --offset requires a value"),
            (&["--tab-id"], "gsh: --tab-id requires a value"),
            (&["-t"], "gsh: --tab-id requires a value"),
            (
                &["--lines", "x"],
                "gsh: --lines requires a non-negative integer",
            ),
            (
                &["--lines=-1"],
                "gsh: --lines requires a non-negative integer",
            ),
            (
                &["--offset=a"],
                "gsh: --offset requires a non-negative integer",
            ),
        ] {
            let err = Flags::parse(&v(args)).unwrap_err();
            assert_eq!(err.to_string(), msg, "{args:?}");
            assert_eq!(err.exit_code(), 2);
        }
    }

    #[test]
    fn alvo_focused_vence_tab_id_que_vence_o_ambiente() {
        let f = Flags::parse(&v(&["--focused", "--tab-id", "t1", "oi"])).unwrap();
        assert_eq!(
            f.target().resolve(Some("t7".into())).as_deref(),
            Some(FOCUSED)
        );
        assert_eq!(f.positionals, vec!["oi"], "a flag não vira texto");
        let f = Flags::parse(&v(&["--tab-id", "t1"])).unwrap();
        assert_eq!(f.target().resolve(Some("t7".into())).as_deref(), Some("t1"));
        let f = Flags::parse(&[]).unwrap();
        assert_eq!(f.target().resolve(Some("t7".into())).as_deref(), Some("t7"));
        assert_eq!(f.target().resolve(None), None);
    }

    #[test]
    fn sem_argumentos_e_ajuda_com_exit_2() {
        let err = parse(&[]).unwrap_err();
        assert_eq!(err.exit_code(), 2);
        assert_eq!(format!("{err}\n"), HELP);
    }

    #[test]
    fn ajuda_e_versao() {
        for a in ["--help", "-h", "help"] {
            assert_eq!(parse(&v(&[a])).unwrap(), Command::Help(HELP));
        }
        for a in ["--version", "-v"] {
            assert_eq!(parse(&v(&[a])).unwrap(), Command::Version);
        }
        // O original só conhece `--version`/`-v`; `-V` e `version` não são verbos.
        for a in ["-V", "version"] {
            assert_ne!(parse(&v(&[a])).ok(), Some(Command::Version), "{a}");
        }
    }

    #[test]
    fn verbo_desconhecido_e_erro_de_uso() {
        assert_eq!(usage_msg(parse(&v(&["xyz"]))), "gsh: unknown verb: xyz");
    }

    #[test]
    fn msg_junta_o_texto_e_aceita_to_nas_duas_formas() {
        let want = Command::Msg {
            to: "Reviewer".into(),
            text: "diff ready".into(),
            json: false,
        };
        assert_eq!(
            parse(&v(&["msg", "--to", "Reviewer", "diff", "ready"])).unwrap(),
            want
        );
        assert_eq!(
            parse(&v(&["msg", "diff", "--to=Reviewer", "ready"])).unwrap(),
            want
        );
        let Command::Msg { json, .. } = parse(&v(&["msg", "--json", "--to", "a", "b"])).unwrap()
        else {
            panic!("esperava Msg");
        };
        assert!(json);
    }

    #[test]
    fn msg_separador_protege_texto_que_parece_flag() {
        let Command::Msg { text, to, .. } =
            parse(&v(&["msg", "--to", "a", "--", "--to", "x"])).unwrap()
        else {
            panic!("esperava Msg");
        };
        assert_eq!(to, "a");
        assert_eq!(text, "--to x");
    }

    #[test]
    fn msg_erros_de_uso() {
        assert_eq!(
            usage_msg(parse(&v(&["msg", "oi"]))),
            format!("gsh msg: missing --to\n{MSG_HELP}")
        );
        assert_eq!(
            usage_msg(parse(&v(&["msg", "--to"]))),
            format!("gsh msg: missing --to\n{MSG_HELP}")
        );
        assert_eq!(
            usage_msg(parse(&v(&["msg", "--to", "a"]))),
            "gsh msg: missing text"
        );
        // Erro de flag vem antes de --to faltando.
        assert_eq!(
            usage_msg(parse(&v(&["msg", "--lines"]))),
            "gsh: --lines requires a value"
        );
    }

    #[test]
    fn deliver_separa_arquivo_e_nota() {
        assert_eq!(
            parse(&v(&[
                "deliver", "--to", "B/fin", "r.pdf", "weekly", "report"
            ]))
            .unwrap(),
            Command::Deliver {
                to: "B/fin".into(),
                file: "r.pdf".into(),
                note: "weekly report".into(),
                json: false
            }
        );
        assert_eq!(
            usage_msg(parse(&v(&["deliver", "--to", "B/fin"]))),
            "gsh deliver: missing file"
        );
        assert_eq!(
            usage_msg(parse(&v(&["deliver", "r.pdf"]))),
            format!("gsh deliver: missing --to\n{DELIVER_HELP}")
        );
    }

    #[test]
    fn ajuda_de_verbo_vence_em_qualquer_posicao() {
        assert_eq!(
            parse(&v(&["msg", "--to", "a", "-h"])).unwrap(),
            Command::Help(MSG_HELP)
        );
        assert_eq!(
            parse(&v(&["deliver", "--help"])).unwrap(),
            Command::Help(DELIVER_HELP)
        );
        assert_eq!(
            parse(&v(&["new-tab", "--title", "x", "--help"])).unwrap(),
            Command::Help(NEW_TAB_HELP)
        );
        assert_eq!(
            parse(&v(&["close-tab", "-h"])).unwrap(),
            Command::Help(CLOSE_TAB_HELP)
        );
    }

    #[test]
    fn new_tab_le_flags_nas_duas_formas_e_ignora_o_resto() {
        let cmd = parse(&v(&[
            "new-tab", "--cwd=/p", "--title", "Worker", "--split", "H", "--tab-id", "t2", "--json",
            "lixo",
        ]))
        .unwrap();
        assert_eq!(
            cmd,
            Command::NewTab {
                cwd: Some("/p".into()),
                title: Some("Worker".into()),
                split: Some(Split::Right),
                target: Target {
                    focused: false,
                    tab_id: Some("t2".into())
                },
                json: true,
            }
        );
    }

    #[test]
    fn new_tab_split_aceita_sinonimos_e_recusa_o_resto() {
        for (raw, want) in [
            ("h", "right"),
            ("horizontal", "right"),
            ("RIGHT", "right"),
            ("v", "down"),
            ("Vertical", "down"),
            ("down", "down"),
        ] {
            let Command::NewTab { split: Some(s), .. } =
                parse(&v(&["new-tab", "--split", raw])).unwrap()
            else {
                panic!("esperava NewTab com split");
            };
            assert_eq!(s.wire(), want, "{raw}");
        }
        // A mensagem cita o valor como foi digitado.
        assert_eq!(
            usage_msg(parse(&v(&["new-tab", "--split", "Diag"]))),
            "gsh new-tab: invalid --split \"Diag\" (use h|right or v|down)"
        );
        // Sem valor: não há split (comportamento do original).
        let Command::NewTab { split, .. } = parse(&v(&["new-tab", "--split"])).unwrap() else {
            panic!("esperava NewTab");
        };
        assert_eq!(split, None);
    }

    #[test]
    fn close_tab_alvo_posicional_opcional() {
        assert_eq!(
            parse(&v(&["close-tab"])).unwrap(),
            Command::CloseTab {
                label: None,
                target: Target::default(),
                json: false
            }
        );
        let Command::CloseTab { label, json, .. } =
            parse(&v(&["close-tab", "--json", "Extension"])).unwrap()
        else {
            panic!("esperava CloseTab");
        };
        assert_eq!(label.as_deref(), Some("Extension"));
        assert!(json);
    }

    #[test]
    fn list_tabs_e_o_alias_legado() {
        let want = Command::ListTabs {
            target: Target {
                focused: false,
                tab_id: Some("t1".into()),
            },
            json: true,
        };
        assert_eq!(
            parse(&v(&["list-tabs", "--json", "-t", "t1"])).unwrap(),
            want
        );
        assert_eq!(
            parse(&v(&["list-panes", "--json", "-t", "t1"])).unwrap(),
            want
        );
    }

    #[test]
    fn hook_harness_default_e_claude() {
        assert_eq!(harness_from(&[]), "claude");
        assert_eq!(harness_from(&v(&["--harness"])), "claude");
        assert_eq!(harness_from(&v(&["--harness", "  "])), "claude");
        assert_eq!(harness_from(&v(&["--harness", "codex"])), "codex");
        assert_eq!(harness_from(&v(&["--harness=codex"])), "codex");
    }
}
