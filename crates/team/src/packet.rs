//! Pacote da fazenda do time (versao 1 de lan_team_farm_channel.dart): JSON + gzip, e com
//! senha o frame cifrado por [`crate::crypto`]. A decodificacao e estrita como a do Dart:
//! campo de tipo errado derruba o pacote inteiro; so loop/agente sem id valido e pulado.

use crate::crypto::{self, TeamKey};
use crate::error::TeamError;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde_json::{json, Map, Value};
use std::io::{Read, Write};
use std::net::IpAddr;

/// Teto do datagrama: o resto dos loops (os mais antigos) fica de fora.
pub const MAX_LOOPS: usize = 80;
/// Teto do corpo depois do gunzip (um datagrama UDP cabe em 64 KiB; gzip nao pode virar bomba).
const MAX_BODY: u64 = 2 * 1024 * 1024;
/// Quantos loops/agentes de um pacote recebido entram (o Dart manda no maximo 80 loops).
const MAX_RECEIVED: usize = 200;

macro_rules! activities {
    ($($v:ident => $n:literal),* $(,)?) => {
        /// O que um loop de agente esta fazendo (`FarmActivity`).
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Activity { $($v),* }
        impl Activity {
            pub const ALL: &[Activity] = &[$(Activity::$v),*];
            /// O `.name` do enum do Dart, que vai no fio.
            pub fn name(self) -> &'static str { match self { $(Activity::$v => $n),* } }
            pub fn from_name(s: &str) -> Option<Self> { match s { $($n => Some(Activity::$v),)* _ => None } }
        }
    };
}
activities! {
    Thinking => "thinking", Reading => "reading", Editing => "editing", Running => "running",
    Searching => "searching", Delegating => "delegating", Tooling => "tooling",
    Waiting => "waiting", Idle => "idle", Sleeping => "sleeping", Done => "done",
}

/// Um loop de agente (`FarmLoop`): so o que vai no fio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FarmLoop {
    pub id: String,
    pub parent_id: Option<String>,
    pub teammate: Option<String>,
    pub harness: String,
    pub project: String,
    pub title: String,
    pub agent_type: Option<String>,
    pub activity: Activity,
    pub tool: Option<String>,
    pub output_tokens: i64,
    pub context_tokens: i64,
    pub input_tokens: i64,
    pub cache_read_tokens: i64,
    pub context_window: i64,
    pub last_seen_ms: i64,
}

/// Uma aba de terminal com agente que aceita mensagem (`TeamAgent`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamAgent {
    pub tab_id: String,
    pub label: String,
    pub harness: String,
    pub project: String,
    pub session_id: Option<String>,
}

/// O que um GSH anuncia ao time (`TeamFarmPacket`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    pub machine_id: String,
    pub name: String,
    pub team: String,
    pub loops: Vec<FarmLoop>,
    pub agents: Vec<TeamAgent>,
    /// Porta TCP do correio dele, como veio (o Dart aceita qualquer inteiro).
    pub mail_port: Option<i64>,
    /// IP de onde o pacote veio (preenchido por quem recebe).
    pub address: Option<IpAddr>,
}

fn loop_to_json(l: &FarmLoop) -> Value {
    let mut o = Map::new();
    o.insert("id".into(), json!(l.id));
    if let Some(v) = &l.parent_id {
        o.insert("p".into(), json!(v));
    }
    if let Some(v) = &l.teammate {
        o.insert("tm".into(), json!(v));
    }
    o.insert("h".into(), json!(l.harness));
    o.insert("pr".into(), json!(l.project));
    o.insert("t".into(), json!(l.title));
    if let Some(v) = &l.agent_type {
        o.insert("ty".into(), json!(v));
    }
    o.insert("a".into(), json!(l.activity.name()));
    if let Some(v) = &l.tool {
        o.insert("tl".into(), json!(v));
    }
    o.insert("o".into(), json!(l.output_tokens));
    o.insert("c".into(), json!(l.context_tokens));
    o.insert("i".into(), json!(l.input_tokens));
    o.insert("r".into(), json!(l.cache_read_tokens));
    if l.context_window > 0 {
        o.insert("cw".into(), json!(l.context_window));
    }
    o.insert("ls".into(), json!(l.last_seen_ms));
    Value::Object(o)
}

fn agent_to_json(a: &TeamAgent) -> Value {
    let mut o = Map::new();
    o.insert("id".into(), json!(a.tab_id));
    o.insert("l".into(), json!(a.label));
    o.insert("h".into(), json!(a.harness));
    o.insert("p".into(), json!(a.project));
    if let Some(s) = &a.session_id {
        o.insert("s".into(), json!(s));
    }
    Value::Object(o)
}

/// O corpo JSON do pacote (sem gzip nem cifra).
pub fn packet_body(p: &Packet) -> Vec<u8> {
    let mut loops: Vec<&FarmLoop> = p.loops.iter().collect();
    loops.sort_by_key(|l| std::cmp::Reverse(l.last_seen_ms));
    let mut o = Map::new();
    o.insert("v".into(), json!(1));
    o.insert("id".into(), json!(p.machine_id));
    o.insert("n".into(), json!(p.name));
    o.insert("t".into(), json!(p.team));
    o.insert(
        "l".into(),
        Value::Array(
            loops
                .into_iter()
                .take(MAX_LOOPS)
                .map(loop_to_json)
                .collect(),
        ),
    );
    if !p.agents.is_empty() {
        o.insert(
            "ag".into(),
            Value::Array(p.agents.iter().map(agent_to_json).collect()),
        );
    }
    if let Some(mp) = p.mail_port {
        o.insert("mp".into(), json!(mp));
    }
    // serializar um Value nao falha; o fallback vazio so existe para nao ter panic
    serde_json::to_vec(&Value::Object(o)).unwrap_or_default()
}

/// Com `key`, o pacote vai cifrado com a senha do time: quem esta na rede sem a senha nao le
/// nomes, pastas nem titulos. Sem chave vai o corpo puro (gzip).
pub fn encode_packet(p: &Packet, key: Option<&TeamKey>) -> Result<Vec<u8>, TeamError> {
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(&packet_body(p))
        .map_err(TeamError::io("gzip"))?;
    let packed = gz.finish().map_err(TeamError::io("gzip"))?;
    match key {
        Some(key) => crypto::seal(key, &packed),
        None => Ok(packed),
    }
}

type Obj = Map<String, Value>;

/// Valor presente com tipo errado: o Dart estoura o cast e descarta o pacote todo.
struct WrongType;

fn opt_str(o: &Obj, k: &str) -> Result<Option<String>, WrongType> {
    match o.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(WrongType),
    }
}

/// `v is int ? v : 0`.
fn int_or_zero(o: &Obj, k: &str) -> i64 {
    o.get(k).and_then(Value::as_i64).unwrap_or(0)
}

fn loop_from_json(l: &Obj) -> Result<Option<FarmLoop>, WrongType> {
    let id = match l.get("id") {
        Some(Value::String(s)) => s.clone(),
        _ => return Ok(None),
    };
    // o Dart le `p` antes de olhar a atividade? Nao: id/atividade primeiro, depois os casts
    let Some(activity) = l
        .get("a")
        .and_then(Value::as_str)
        .and_then(Activity::from_name)
    else {
        return Ok(None);
    };
    Ok(Some(FarmLoop {
        id,
        parent_id: opt_str(l, "p")?,
        teammate: opt_str(l, "tm")?,
        harness: opt_str(l, "h")?.unwrap_or_else(|| "claude".into()),
        project: opt_str(l, "pr")?.unwrap_or_default(),
        title: opt_str(l, "t")?.unwrap_or_default(),
        agent_type: opt_str(l, "ty")?,
        activity,
        tool: opt_str(l, "tl")?,
        output_tokens: int_or_zero(l, "o"),
        context_tokens: int_or_zero(l, "c"),
        input_tokens: int_or_zero(l, "i"),
        cache_read_tokens: int_or_zero(l, "r"),
        context_window: int_or_zero(l, "cw"),
        last_seen_ms: int_or_zero(l, "ls"),
    }))
}

fn agent_from_json(a: &Obj) -> Result<Option<TeamAgent>, WrongType> {
    let id = match a.get("id") {
        Some(Value::String(s)) => s.clone(),
        _ => return Ok(None),
    };
    Ok(Some(TeamAgent {
        label: opt_str(a, "l")?.unwrap_or_else(|| id.clone()),
        harness: opt_str(a, "h")?.unwrap_or_default(),
        project: opt_str(a, "p")?.unwrap_or_default(),
        session_id: opt_str(a, "s")?,
        tab_id: id,
    }))
}

/// `(o[k] as List? ?? const []).whereType<Map>()`.
fn maps<'a>(o: &'a Obj, k: &str) -> Result<impl Iterator<Item = &'a Obj>, WrongType> {
    let list: &[Value] = match o.get(k) {
        None | Some(Value::Null) => &[],
        Some(Value::Array(a)) => a,
        Some(_) => return Err(WrongType),
    };
    Ok(list.iter().filter_map(Value::as_object).take(MAX_RECEIVED))
}

pub(crate) fn gunzip(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    GzDecoder::new(data)
        .take(MAX_BODY + 1)
        .read_to_end(&mut out)
        .ok()?;
    (out.len() as u64 <= MAX_BODY).then_some(out)
}

fn parse_body(body: &[u8], address: Option<IpAddr>) -> Result<Option<Packet>, WrongType> {
    let Ok(Value::Object(o)) = serde_json::from_slice::<Value>(body) else {
        return Ok(None);
    };
    // `o['v'] != 1`: no Dart 1.0 == 1
    if o.get("v").and_then(Value::as_f64) != Some(1.0) {
        return Ok(None);
    }
    let Some(Value::String(machine_id)) = o.get("id") else {
        return Err(WrongType);
    };
    let mail_port = match o.get("mp") {
        None | Some(Value::Null) => None,
        Some(v) => Some(v.as_i64().ok_or(WrongType)?),
    };
    let mut loops = Vec::new();
    for l in maps(&o, "l")? {
        loops.extend(loop_from_json(l)?);
    }
    let mut agents = Vec::new();
    for a in maps(&o, "ag")? {
        agents.extend(agent_from_json(a)?);
    }
    Ok(Some(Packet {
        machine_id: machine_id.clone(),
        name: opt_str(&o, "n")?.unwrap_or_default(),
        team: opt_str(&o, "t")?.unwrap_or_default(),
        loops,
        agents,
        mail_port,
        address,
    }))
}

/// `None` para lixo, versao desconhecida, pacote de outro programa ou cifra que nao abre.
/// Com `key`, so aceita pacote cifrado com ela; sem, so aceita pacote aberto (times com e
/// sem senha nao se misturam). Tamanho e formato sao conferidos antes de decifrar.
pub fn decode_packet(
    bytes: &[u8],
    key: Option<&TeamKey>,
    address: Option<IpAddr>,
) -> Option<Packet> {
    let packed = match key {
        Some(key) => crypto::open(key, bytes)?,
        None => bytes.to_vec(),
    };
    parse_body(&gunzip(&packed)?, address).ok().flatten()
}
