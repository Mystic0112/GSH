use serde_json::{json, Value};
use std::{net::TcpStream, time::Duration};
use thiserror::Error;
use tungstenite::{client::IntoClientRequest, http::header::AUTHORIZATION, Message, WebSocket};

#[derive(Debug, Error)]
pub enum BrowserError {
    #[error("{0}")]
    Message(String),
    #[error("CDP error: {0}")]
    Cdp(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("WebSocket error: {0}")]
    Ws(#[source] Box<tungstenite::Error>),
}
impl From<tungstenite::Error> for BrowserError {
    fn from(error: tungstenite::Error) -> Self {
        Self::Ws(Box::new(error))
    }
}
pub struct Cdp {
    ws: WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
    id: u64,
}
impl Cdp {
    pub fn connect(url: &str, token: &str) -> Result<Self, BrowserError> {
        let mut req = url.into_client_request()?;
        req.headers_mut().insert(
            AUTHORIZATION,
            format!("Bearer {token}")
                .parse()
                .map_err(|_| BrowserError::Message("invalid CDP authorization header".into()))?,
        );
        let (ws, _) = tungstenite::connect(req)?;
        if let tungstenite::stream::MaybeTlsStream::Plain(s) = ws.get_ref() {
            s.set_read_timeout(Some(Duration::from_secs(30)))?;
        }
        Ok(Self { ws, id: 0 })
    }
    pub fn call(
        &mut self,
        method: &str,
        params: Value,
        session: Option<&str>,
    ) -> Result<Value, BrowserError> {
        self.id += 1;
        let id = self.id;
        let mut msg = json!({"id":id,"method":method,"params":params});
        if let Some(s) = session {
            msg["sessionId"] = json!(s);
        }
        self.ws.send(Message::Text(msg.to_string().into()))?;
        loop {
            let m = self.ws.read()?;
            if let Message::Text(t) = m {
                let v: Value = serde_json::from_str(&t)?;
                if v.get("id").and_then(Value::as_u64) == Some(id) {
                    if let Some(e) = v.get("error") {
                        return Err(BrowserError::Cdp(e.to_string()));
                    }
                    return Ok(v.get("result").cloned().unwrap_or(Value::Null));
                }
            }
        }
    }
}
