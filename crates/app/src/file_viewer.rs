//! Viewer read-only de arquivo de texto (`file_viewer.dart` e
//! `file_reader_impl.dart` do app Flutter).
//!
//! Carrega arquivos UTF-8 até [`MAX_VIEWER_BYTES`] (2 MiB) com numeração de
//! linha à esquerda em `JetBrainsMono NF` (`TEXT2`) e corpo em `PANEL`.
//! Acima do limite, binários (bytes nulos ou UTF-8 inválido) e falhas de I/O
//! viram estados tipados sem carregar nem causar `panic`.

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use iced::widget::{column, container, row, scrollable, text, Space};
use iced::{Background, Color, Element, Fill, Length, Theme};

use crate::labels;
use crate::theme;

/// Limite máximo de tamanho para abertura no viewer de texto (2 MiB, igual a
/// `_maxTextBytes` em `file_reader_impl.dart`).
pub const MAX_VIEWER_BYTES: u64 = 2 * 1024 * 1024;
const LARGE_PREVIEW_BYTES: u64 = 64 * 1024;

/// Motivo tipado pelo qual um arquivo foi classificado como binário.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryKind {
    /// Contém ao menos um byte nulo (`0x00`).
    NullBytes,
    /// Sequência de bytes não é UTF-8 válido.
    InvalidUtf8,
}

impl BinaryKind {
    pub fn ui_message(self) -> &'static str {
        match self {
            Self::NullBytes => labels::VIEWER_BINARY_NULL_BYTES,
            Self::InvalidUtf8 => labels::VIEWER_BINARY_INVALID_UTF8,
        }
    }
}

/// Erro tipado ao ler um arquivo para o viewer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewerReadError {
    NotFound,
    PermissionDenied,
    NotRegularFile,
    Io(String),
}

impl ViewerReadError {
    pub fn from_io(err: &io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => Self::NotFound,
            io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            _ => Self::Io(err.to_string()),
        }
    }

    pub fn ui_message(&self) -> String {
        match self {
            Self::NotFound => labels::VIEWER_ERR_NOT_FOUND.to_string(),
            Self::PermissionDenied => labels::VIEWER_ERR_PERMISSION.to_string(),
            Self::NotRegularFile => labels::VIEWER_ERR_NOT_FILE.to_string(),
            Self::Io(detail) => format!("{}{detail}", labels::VIEWER_ERR_IO_PREFIX),
        }
    }
}

/// Documento de texto UTF-8 preparado para exibição com gutter de linhas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDocument {
    path: PathBuf,
    lines: Vec<String>,
    gutter_text: String,
    body_text: String,
    source: String,
}

impl TextDocument {
    /// Constrói o documento numerando as linhas a partir de 1.
    pub fn from_text(path: PathBuf, content: &str) -> Self {
        Self::from_owned_text(path, content.to_owned())
    }
    pub fn from_owned_text(path: PathBuf, content: String) -> Self {
        let mut lines: Vec<String> = content.lines().map(ToOwned::to_owned).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        let digits = lines.len().to_string().len().max(2);
        let gutter_text = (1..=lines.len())
            .map(|n| format!("{n:>digits$}"))
            .collect::<Vec<_>>()
            .join("\n");
        let body_text = lines.join("\n");
        Self {
            path,
            lines,
            gutter_text,
            body_text,
            source: content,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn gutter_text(&self) -> &str {
        &self.gutter_text
    }

    pub fn body_text(&self) -> &str {
        &self.body_text
    }

    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn into_source(self) -> String {
        self.source
    }
}

/// Estado tipado do viewer de arquivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileViewerState {
    Loaded(TextDocument),
    TooLarge {
        size_bytes: u64,
        max_bytes: u64,
    },
    LargePreview {
        size_bytes: u64,
        max_bytes: u64,
        document: TextDocument,
    },
    Binary(BinaryKind),
    Error(ViewerReadError),
}

impl FileViewerState {
    /// Carrega `path` aplicando o limite padrão [`MAX_VIEWER_BYTES`].
    pub fn load(path: &Path) -> Self {
        Self::load_with_limit(path, MAX_VIEWER_BYTES)
    }

    /// Carrega `path` verificando metadados antes de ler os bytes.
    pub fn load_with_limit(path: &Path, max_bytes: u64) -> Self {
        let meta = match fs::metadata(path) {
            Ok(m) => m,
            Err(e) => return Self::Error(ViewerReadError::from_io(&e)),
        };
        if !meta.is_file() {
            return Self::Error(ViewerReadError::NotRegularFile);
        }
        if meta.len() > max_bytes {
            return Self::large_preview(path, meta.len(), max_bytes);
        }

        let file = match fs::File::open(path) {
            Ok(f) => f,
            Err(e) => return Self::Error(ViewerReadError::from_io(&e)),
        };
        let mut bytes = Vec::new();
        if let Err(e) = file
            .take(max_bytes.saturating_add(1))
            .read_to_end(&mut bytes)
        {
            return Self::Error(ViewerReadError::from_io(&e));
        }
        let read_len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        if read_len > max_bytes {
            return Self::large_preview(path, read_len, max_bytes);
        }

        if bytes.contains(&0) {
            return Self::Binary(BinaryKind::NullBytes);
        }
        match String::from_utf8(bytes) {
            Ok(content) => Self::Loaded(TextDocument::from_owned_text(path.to_path_buf(), content)),
            Err(_) => Self::Binary(BinaryKind::InvalidUtf8),
        }
    }

    fn large_preview(path: &Path, size_bytes: u64, max_bytes: u64) -> Self {
        let fallback = Self::TooLarge {
            size_bytes,
            max_bytes,
        };
        let Ok(file) = fs::File::open(path) else {
            return fallback;
        };
        let mut bytes = Vec::new();
        if file
            .take(max_bytes.min(LARGE_PREVIEW_BYTES))
            .read_to_end(&mut bytes)
            .is_err()
        {
            return fallback;
        }
        if bytes.contains(&0) {
            return Self::Binary(BinaryKind::NullBytes);
        }
        let content = match std::str::from_utf8(&bytes) {
            Ok(content) => content,
            Err(error) if error.error_len().is_none() => {
                match std::str::from_utf8(bytes.get(..error.valid_up_to()).unwrap_or_default()) {
                    Ok(content) => content,
                    Err(_) => return fallback,
                }
            }
            Err(_) => return Self::Binary(BinaryKind::InvalidUtf8),
        };
        Self::LargePreview {
            size_bytes,
            max_bytes,
            document: TextDocument::from_text(path.to_path_buf(), content),
        }
    }

    /// Mensagem de aviso/erro quando o estado não é [`Self::Loaded`].
    pub fn ui_notice(&self) -> Option<String> {
        match self {
            Self::Loaded(_) => None,
            Self::TooLarge {
                size_bytes,
                max_bytes,
            }
            | Self::LargePreview {
                size_bytes,
                max_bytes,
                ..
            } => Some(labels::format_viewer_too_large(*size_bytes, *max_bytes)),
            Self::Binary(kind) => Some(kind.ui_message().to_string()),
            Self::Error(err) => Some(err.ui_message()),
        }
    }
}

/// Renderiza o viewer de arquivo na superfície `PANEL`.
pub fn view<'a, M: 'a>(state: &'a FileViewerState) -> Element<'a, M> {
    match state {
        FileViewerState::Loaded(doc) => view_document(doc),
        FileViewerState::LargePreview {
            size_bytes,
            max_bytes,
            document,
        } => column![
            text(labels::format_viewer_large_preview(*size_bytes, *max_bytes)).color(theme::WARN),
            view_document(document),
        ]
        .into(),
        FileViewerState::TooLarge {
            size_bytes,
            max_bytes,
        } => view_status_notice(
            labels::format_viewer_too_large(*size_bytes, *max_bytes),
            theme::WARN,
        ),
        FileViewerState::Binary(kind) => {
            view_status_notice(kind.ui_message().to_string(), theme::TEXT2)
        }
        FileViewerState::Error(err) => view_status_notice(err.ui_message(), theme::ERROR),
    }
}

fn view_document<'a, M: 'a>(doc: &'a TextDocument) -> Element<'a, M> {
    let gutter = container(
        text(&doc.gutter_text)
            .font(theme::FONT_MONO)
            .size(theme::SIZE_MONO)
            .line_height(theme::LINE_HEIGHT_MONO)
            .color(theme::TEXT2)
            .wrapping(text::Wrapping::None),
    )
    .padding([0, 14]);

    let separator = container(Space::new().width(1.0).height(Fill))
        .width(Length::Fixed(1.0))
        .height(Fill)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::BORDER)),
            ..Default::default()
        });

    let code = container(
        text(&doc.body_text)
            .font(theme::FONT_MONO)
            .size(theme::SIZE_MONO)
            .line_height(theme::LINE_HEIGHT_MONO)
            .color(theme::TEXT)
            .wrapping(text::Wrapping::None),
    )
    .padding([0, 14]);

    let scrolled = scrollable(row![gutter, separator, code].padding([12, 0]))
        .direction(scrollable::Direction::Both {
            vertical: scrollable::Scrollbar::default(),
            horizontal: scrollable::Scrollbar::default(),
        })
        .width(Fill)
        .height(Fill);

    container(scrolled)
        .width(Fill)
        .height(Fill)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::PANEL)),
            ..Default::default()
        })
        .into()
}

fn view_status_notice<'a, M: 'a>(message: String, color: Color) -> Element<'a, M> {
    container(
        text(message)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_BODY)
            .color(color),
    )
    .padding([20, 24])
    .width(Fill)
    .height(Fill)
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::PANEL)),
        ..Default::default()
    })
    .into()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static SEQ: AtomicU64 = AtomicU64::new(0);

    struct TempFile {
        dir: PathBuf,
    }

    impl TempFile {
        fn new() -> Self {
            let id = SEQ.fetch_add(1, Ordering::Relaxed);
            let dir =
                std::env::temp_dir().join(format!("gsh-viewer-test-{}-{}", std::process::id(), id));
            fs::create_dir_all(&dir).expect("create temp viewer dir");
            Self { dir }
        }

        fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let p = self.dir.join(name);
            fs::write(&p, bytes).expect("write test file");
            p
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn carrega_texto_utf8_e_gera_numeros_de_linha_alinhados() {
        let tmp = TempFile::new();
        let path = tmp.write("hello.rs", b"fn main() {\n    println!(\"oi\");\n}\n");

        let state = FileViewerState::load(&path);
        let FileViewerState::Loaded(doc) = state else {
            panic!("esperava FileViewerState::Loaded");
        };
        assert_eq!(doc.line_count(), 3);
        assert_eq!(doc.lines(), &["fn main() {", "    println!(\"oi\");", "}"]);
        assert_eq!(doc.gutter_text(), " 1\n 2\n 3");
    }

    #[test]
    fn arquivo_acima_do_limite_retorna_aviso_tipado_sem_carregar() {
        let tmp = TempFile::new();
        let path = tmp.write("big.log", &[b'a'; 128]);

        let state = FileViewerState::load_with_limit(&path, 64);
        assert!(matches!(
            state,
            FileViewerState::LargePreview {
                size_bytes: 128,
                max_bytes: 64,
                ..
            }
        ));
        assert!(state
            .ui_notice()
            .unwrap()
            .starts_with(labels::VIEWER_TOO_LARGE_PREFIX));
    }

    #[test]
    fn rejeita_arquivo_binario_com_bytes_nulos_ou_utf8_invalido() {
        let tmp = TempFile::new();
        let null_file = tmp.write("bin.dat", b"abc\0def");
        assert_eq!(
            FileViewerState::load(&null_file),
            FileViewerState::Binary(BinaryKind::NullBytes)
        );

        let bad_utf8 = tmp.write("latin.bin", &[0xFF, 0xFE, 0xFD]);
        assert_eq!(
            FileViewerState::load(&bad_utf8),
            FileViewerState::Binary(BinaryKind::InvalidUtf8)
        );
    }

    #[test]
    fn erro_de_leitura_retorna_estado_tipado_sem_panic() {
        let missing = std::env::temp_dir().join("gsh-viewer-arquivo-inexistente-999999.txt");
        let state = FileViewerState::load(&missing);
        assert_eq!(state, FileViewerState::Error(ViewerReadError::NotFound));
        assert_eq!(
            state.ui_notice().as_deref(),
            Some(labels::VIEWER_ERR_NOT_FOUND)
        );
    }
}
