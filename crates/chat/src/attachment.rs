//! Anexos da mensagem (`chat_attachment.dart` + `buildClaudeUserContent` do Flutter).

use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Value};

use crate::error::ChatError;

/// Teto da API para imagem.
pub const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;
/// Teto da API para PDF.
pub const MAX_PDF_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentKind {
    /// Vai como bloco `image` (o modelo vê a imagem).
    Image,
    /// Vai como bloco `document` (PDF).
    Pdf,
    /// Vai como menção `@"caminho"` no texto: o Claude Code lê o arquivo.
    File,
}

/// Um anexo: arquivo do disco (`path`) ou imagem colada (`bytes`, sem caminho).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    pub kind: AttachmentKind,
    pub name: String,
    pub path: Option<PathBuf>,
    pub bytes: Option<Vec<u8>>,
    pub media_type: Option<&'static str>,
}

fn image_type(ext: &str) -> Option<&'static str> {
    match ext {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

impl Attachment {
    /// Arquivo do disco; o tipo sai da extensão.
    pub fn file(path: &Path) -> Self {
        let text = path.to_string_lossy();
        let name = text
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or_default()
            .to_string();
        let ext = name
            .rsplit_once('.')
            .map(|(_, e)| e.to_lowercase())
            .unwrap_or_default();
        let (kind, media_type) = if let Some(img) = image_type(&ext) {
            (AttachmentKind::Image, Some(img))
        } else if ext == "pdf" {
            (AttachmentKind::Pdf, Some("application/pdf"))
        } else {
            (AttachmentKind::File, None)
        };
        Self {
            kind,
            name,
            path: Some(path.to_path_buf()),
            bytes: None,
            media_type,
        }
    }

    /// Imagem colada da área de transferência (PNG).
    pub fn pasted_image(png: Vec<u8>, name: String) -> Self {
        Self {
            kind: AttachmentKind::Image,
            name,
            path: None,
            bytes: Some(png),
            media_type: Some("image/png"),
        }
    }
}

/// Conteúdo da mensagem do usuário. Sem anexo, só o texto. Com anexo: imagem vira bloco
/// `image` (base64), PDF vira `document`, e o resto vira menção `@"caminho"` no texto, que o
/// próprio Claude Code expande lendo o arquivo.
pub fn build_user_content(text: &str, attachments: &[Attachment]) -> Result<Value, ChatError> {
    if attachments.is_empty() {
        return Ok(Value::String(text.to_string()));
    }
    let mentions: Vec<String> = attachments
        .iter()
        .filter(|a| a.kind == AttachmentKind::File)
        .filter_map(|a| a.path.as_ref())
        .map(|p| format!("@\"{}\"", p.display()))
        .collect();
    let first = if mentions.is_empty() {
        text.to_string()
    } else {
        format!("{text}\n\n{}", mentions.join(" "))
    };
    let mut blocks = vec![json!({"type": "text", "text": first})];
    for a in attachments {
        if a.kind == AttachmentKind::File {
            continue;
        }
        let bytes = match (&a.bytes, &a.path) {
            (Some(b), _) => b.clone(),
            (None, Some(p)) => std::fs::read(p).map_err(|e| ChatError::AttachmentRead {
                path: p.display().to_string(),
                cause: e.to_string(),
            })?,
            (None, None) => Vec::new(),
        };
        let max = if a.kind == AttachmentKind::Image {
            MAX_IMAGE_BYTES
        } else {
            MAX_PDF_BYTES
        };
        if bytes.len() > max {
            return Err(ChatError::AttachmentTooLarge {
                name: a.name.clone(),
            });
        }
        blocks.push(json!({
            "type": if a.kind == AttachmentKind::Image { "image" } else { "document" },
            "source": {
                "type": "base64",
                "media_type": a.media_type,
                "data": STANDARD.encode(&bytes),
            },
        }));
    }
    Ok(Value::Array(blocks))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Vetores de `test/data/claude_stream_engine_test.dart` (grupo "anexos").
    #[test]
    fn imagem_e_pdf_viram_blocos_outros_arquivos_viram_mencao() {
        let dir = std::env::temp_dir().join(format!("gsh-chat-att-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let pdf = dir.join("a b.pdf");
        std::fs::write(&pdf, [1u8, 2, 3]).expect("pdf");
        let content = build_user_content(
            "veja",
            &[
                Attachment::pasted_image(vec![9, 9], "x.png".to_string()),
                Attachment::file(&pdf),
                Attachment::file(Path::new("/repo/notas da reuniao.txt")),
            ],
        )
        .expect("content");
        let blocks = content.as_array().expect("array");
        assert_eq!(
            blocks[0],
            json!({"type": "text", "text": "veja\n\n@\"/repo/notas da reuniao.txt\""})
        );
        assert_eq!(blocks[1]["type"], "image");
        assert_eq!(blocks[1]["source"]["media_type"], "image/png");
        assert_eq!(blocks[1]["source"]["data"], "CQk=");
        assert_eq!(blocks[2]["type"], "document");
        assert_eq!(blocks[2]["source"]["data"], "AQID");
        assert_eq!(
            build_user_content("só texto", &[]).expect("text"),
            Value::String("só texto".to_string())
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tipo_sai_da_extensao() {
        assert_eq!(
            Attachment::file(Path::new("/a/F.JPG")).media_type,
            Some("image/jpeg")
        );
        assert_eq!(
            Attachment::file(Path::new("/a/b.pdf")).kind,
            AttachmentKind::Pdf
        );
        assert_eq!(
            Attachment::file(Path::new("/a/Makefile")).kind,
            AttachmentKind::File
        );
    }

    #[test]
    fn imagem_acima_do_teto_e_recusada() {
        let big = Attachment::pasted_image(vec![0; MAX_IMAGE_BYTES + 1], "grande.png".to_string());
        assert_eq!(
            build_user_content("x", &[big]),
            Err(ChatError::AttachmentTooLarge {
                name: "grande.png".to_string()
            })
        );
    }
}
