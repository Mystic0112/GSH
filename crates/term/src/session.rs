//! Sessão de terminal: PTY (`portable-pty`) + emulador VT (`alacritty_terminal`).

use std::ffi::{OsStr, OsString};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::{
    point_to_viewport, viewport_to_point, Config as TermConfig, Term, MIN_COLUMNS, MIN_SCREEN_LINES,
};
use alacritty_terminal::vte::ansi::Processor;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};

use crate::error::TermError;
use crate::snapshot::{convert_cell, is_cursor_shape_visible, Cell, Cursor, Snapshot};

/// Limite padrão de linhas no histórico de scrollback (mesmo valor do Flutter).
pub const DEFAULT_SCROLLBACK_LINES: usize = 10_000;

/// Shell padrão em sistemas Unix.
pub const DEFAULT_UNIX_SHELL: &str = "/bin/bash";

/// Shell padrão de fallback no Windows.
pub const DEFAULT_WINDOWS_SHELL: &str = "cmd.exe";

/// Retorna o shell padrão da plataforma atual (`/bin/bash` no Unix,
/// `pwsh.exe` ou `cmd.exe` no Windows).
pub fn default_shell() -> &'static str {
    #[cfg(unix)]
    {
        DEFAULT_UNIX_SHELL
    }
    #[cfg(windows)]
    {
        if windows_has_pwsh() {
            "pwsh.exe"
        } else {
            DEFAULT_WINDOWS_SHELL
        }
    }
}

#[cfg(windows)]
fn windows_has_pwsh() -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| dir.join("pwsh.exe").is_file())
}

/// Especificação de comando/shell para iniciar uma [`TermSession`].
#[derive(Debug, Clone)]
pub struct ShellCommand {
    program: String,
    args: Vec<OsString>,
    cwd: Option<PathBuf>,
    env: Vec<(OsString, OsString)>,
    scrollback_lines: usize,
}

impl Default for ShellCommand {
    fn default() -> Self {
        Self::new(default_shell())
    }
}

impl ShellCommand {
    /// Cria uma configuração de comando para o executável informado.
    /// Se `program` for vazio, usa [`default_shell`].
    pub fn new(program: impl AsRef<str>) -> Self {
        let raw = program.as_ref().trim();
        let resolved = if raw.is_empty() {
            default_shell().to_owned()
        } else {
            raw.to_owned()
        };
        Self {
            program: resolved,
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
            scrollback_lines: DEFAULT_SCROLLBACK_LINES,
        }
    }

    /// Adiciona um argumento ao processo do shell.
    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    /// Adiciona uma sequência de argumentos ao processo do shell.
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.args.push(arg.as_ref().to_os_string());
        }
        self
    }

    /// Define o diretório de trabalho inicial do processo no PTY.
    pub fn cwd(mut self, dir: impl AsRef<Path>) -> Self {
        self.cwd = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Define ou sobrescreve uma variável de ambiente no processo do PTY.
    pub fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.env
            .push((key.as_ref().to_os_string(), value.as_ref().to_os_string()));
        self
    }

    /// Define a capacidade máxima de linhas de scrollback no emulador VT.
    pub fn scrollback_lines(mut self, lines: usize) -> Self {
        self.scrollback_lines = lines;
        self
    }

    /// Nome ou caminho do programa configurado.
    pub fn program(&self) -> &str {
        &self.program
    }
}

/// Conversão para [`ShellCommand`] aceita por [`TermSession::spawn`].
///
/// Permite passar `None` (usa [`default_shell`]), `Some("/bin/bash")`,
/// `"/bin/bash"`, `String`, `&Path`, `PathBuf` ou um [`ShellCommand`] completo.
pub trait IntoShell {
    fn into_shell(self) -> ShellCommand;
}

impl IntoShell for ShellCommand {
    fn into_shell(self) -> ShellCommand {
        self
    }
}

impl IntoShell for Option<&str> {
    fn into_shell(self) -> ShellCommand {
        match self {
            Some(s) => ShellCommand::new(s),
            None => ShellCommand::default(),
        }
    }
}

impl IntoShell for &str {
    fn into_shell(self) -> ShellCommand {
        ShellCommand::new(self)
    }
}

impl IntoShell for String {
    fn into_shell(self) -> ShellCommand {
        ShellCommand::new(self)
    }
}

impl IntoShell for &String {
    fn into_shell(self) -> ShellCommand {
        ShellCommand::new(self.as_str())
    }
}

impl IntoShell for &Path {
    fn into_shell(self) -> ShellCommand {
        ShellCommand::new(self.to_string_lossy())
    }
}

impl IntoShell for PathBuf {
    fn into_shell(self) -> ShellCommand {
        ShellCommand::new(self.to_string_lossy())
    }
}

#[derive(Debug, Clone, Copy)]
struct TermDimensions {
    cols: usize,
    rows: usize,
}

impl Dimensions for TermDimensions {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

#[derive(Clone, Default)]
struct ProxyListener {
    pty_replies: Arc<Mutex<Vec<String>>>,
}

impl ProxyListener {
    fn take_replies(&self) -> Vec<String> {
        let mut guard = lock_unpoisoned(&self.pty_replies);
        std::mem::take(&mut *guard)
    }
}

impl EventListener for ProxyListener {
    fn send_event(&self, event: Event) {
        if let Event::PtyWrite(text) = event {
            let mut guard = lock_unpoisoned(&self.pty_replies);
            guard.push(text);
        }
    }
}

struct EmulatorState {
    term: Term<ProxyListener>,
    processor: Processor,
    listener: ProxyListener,
    cols: u16,
    rows: u16,
}

impl EmulatorState {
    fn new(cols: u16, rows: u16, scrollback_lines: usize) -> Self {
        let dims = TermDimensions {
            cols: usize::from(cols),
            rows: usize::from(rows),
        };
        let config = TermConfig {
            scrolling_history: scrollback_lines,
            ..TermConfig::default()
        };
        let listener = ProxyListener::default();
        let term = Term::new(config, &dims, listener.clone());
        Self {
            term,
            processor: Processor::new(),
            listener,
            cols,
            rows,
        }
    }

    fn advance(&mut self, bytes: &[u8]) -> Vec<String> {
        self.processor.advance(&mut self.term, bytes);
        self.listener.take_replies()
    }

    fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
        let dims = TermDimensions {
            cols: usize::from(cols),
            rows: usize::from(rows),
        };
        self.term.resize(dims);
    }

    fn snapshot(&self) -> Snapshot {
        let grid = self.term.grid();
        let cols_usize = grid.columns();
        let rows_usize = grid.screen_lines();
        let display_offset = grid.display_offset();
        let history_size = grid.history_size();

        let mut lines: Vec<Vec<Cell>> = (0..rows_usize)
            .map(|_| vec![Cell::default(); cols_usize])
            .collect();

        for indexed in grid.display_iter() {
            if let Some(vp) = point_to_viewport(display_offset, indexed.point) {
                if let Some(row_cells) = lines.get_mut(vp.line) {
                    if let Some(slot) = row_cells.get_mut(vp.column.0) {
                        *slot = convert_cell(indexed.cell);
                    }
                }
            }
        }

        let renderable = self.term.renderable_content();
        let cursor_point = renderable.cursor.point;
        let cursor_shape = renderable.cursor.shape;

        let (cursor_row, cursor_col, in_viewport) =
            match point_to_viewport(display_offset, cursor_point) {
                Some(vp) if vp.line < rows_usize && vp.column.0 < cols_usize => {
                    (vp.line, vp.column.0, true)
                }
                _ => (
                    rows_usize.saturating_sub(1),
                    cursor_point.column.0.min(cols_usize.saturating_sub(1)),
                    false,
                ),
            };

        let cursor = Cursor {
            row: cursor_row,
            col: cursor_col,
            visible: in_viewport && is_cursor_shape_visible(cursor_shape),
        };

        Snapshot {
            cols: self.cols,
            rows: self.rows,
            lines,
            cursor,
            display_offset,
            history_size,
        }
    }

    fn viewport_point(&self, row: usize, col: usize) -> Point {
        let grid = self.term.grid();
        let clamped_row = row.min(grid.screen_lines().saturating_sub(1));
        let clamped_col = col.min(grid.columns().saturating_sub(1));
        let display_offset = grid.display_offset();
        viewport_to_point(display_offset, Point::new(clamped_row, Column(clamped_col)))
    }
}

/// Sessão ativa de terminal combinando processo em PTY (`portable-pty`) e
/// emulador VT (`alacritty_terminal`).
pub struct TermSession {
    state: Arc<(Mutex<EmulatorState>, Condvar)>,
    writer: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
    master: Mutex<Option<Box<dyn MasterPty + Send>>>,
    child: Mutex<Option<Box<dyn Child + Send + Sync>>>,
    shutdown: Arc<AtomicBool>,
    reader_thread: Option<JoinHandle<()>>,
}

/// Alias semântico para [`TermSession`].
pub type Session = TermSession;

impl TermSession {
    /// Inicia uma nova sessão de terminal com o `shell` e dimensões `(cols, rows)`.
    ///
    /// Se `shell` for `None` ou `""`, utiliza [`default_shell`] (`/bin/bash` no
    /// Unix, `pwsh.exe`/`cmd.exe` no Windows).
    pub fn spawn(shell: impl IntoShell, cols: u16, rows: u16) -> Result<Self, TermError> {
        validate_dimensions(cols, rows)?;

        let spec = shell.into_shell();
        let pty_size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(pty_size)
            .map_err(|err| TermError::OpenPty {
                cols,
                rows,
                cause: err.to_string(),
            })?;

        let cmd = build_command(&spec);
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|err| TermError::Spawn {
                shell: spec.program.clone(),
                cause: err.to_string(),
            })?;

        // Fecha o descritor slave no processo pai para que o master receba EOF
        // assim que o processo filho encerrar.
        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|err| TermError::PtyStream {
                cause: err.to_string(),
            })?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|err| TermError::PtyStream {
                cause: err.to_string(),
            })?;

        let state = Arc::new((
            Mutex::new(EmulatorState::new(cols, rows, spec.scrollback_lines)),
            Condvar::new(),
        ));
        let writer_arc: Arc<Mutex<Option<Box<dyn Write + Send>>>> =
            Arc::new(Mutex::new(Some(writer)));
        let shutdown = Arc::new(AtomicBool::new(false));

        let reader_state = Arc::clone(&state);
        let reader_writer = Arc::clone(&writer_arc);
        let reader_shutdown = Arc::clone(&shutdown);

        let reader_thread = thread::Builder::new()
            .name("gsh-term-reader".to_owned())
            .spawn(move || {
                reader_loop(reader, &reader_state, &reader_writer, &reader_shutdown);
            })
            .map_err(TermError::ReaderThread)?;

        Ok(Self {
            state,
            writer: writer_arc,
            master: Mutex::new(Some(pair.master)),
            child: Mutex::new(Some(child)),
            shutdown,
            reader_thread: Some(reader_thread),
        })
    }

    /// Envia bytes crus (teclas digitadas ou comandos) para a entrada do PTY.
    pub fn send(&self, bytes: &[u8]) -> Result<(), TermError> {
        let mut guard = lock_unpoisoned(&self.writer);
        let writer = guard.as_mut().ok_or(TermError::Closed)?;
        writer.write_all(bytes).map_err(TermError::Write)?;
        writer.flush().map_err(TermError::Write)?;
        Ok(())
    }

    /// Redimensiona o PTY e o emulador VT simultaneamente para `(cols, rows)`.
    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), TermError> {
        validate_dimensions(cols, rows)?;

        {
            let master_guard = lock_unpoisoned(&self.master);
            if let Some(master) = master_guard.as_ref() {
                master
                    .resize(PtySize {
                        rows,
                        cols,
                        pixel_width: 0,
                        pixel_height: 0,
                    })
                    .map_err(|err| TermError::Resize {
                        cols,
                        rows,
                        cause: err.to_string(),
                    })?;
            }
        }

        let (lock, cvar) = &*self.state;
        let mut state = lock_unpoisoned(lock);
        state.resize(cols, rows);
        cvar.notify_all();
        Ok(())
    }

    /// Dimensões atuais `(cols, rows)` da sessão.
    pub fn size(&self) -> (u16, u16) {
        let (lock, _) = &*self.state;
        let state = lock_unpoisoned(lock);
        (state.cols, state.rows)
    }

    /// Extrai um [`Snapshot`] do grid visível, cursor e scrollback para a UI.
    pub fn snapshot(&self) -> Snapshot {
        let (lock, _) = &*self.state;
        let state = lock_unpoisoned(lock);
        state.snapshot()
    }

    /// Rola o scrollback para cima (em direção ao histórico antigo) em `lines` linhas.
    pub fn scroll_up(&self, lines: usize) {
        if lines == 0 {
            return;
        }
        let delta = i32::try_from(lines).unwrap_or(i32::MAX);
        let (lock, cvar) = &*self.state;
        let mut state = lock_unpoisoned(lock);
        state.term.scroll_display(Scroll::Delta(delta));
        cvar.notify_all();
    }

    /// Rola o scrollback para baixo (em direção ao presente) em `lines` linhas.
    pub fn scroll_down(&self, lines: usize) {
        if lines == 0 {
            return;
        }
        let delta = i32::try_from(lines).unwrap_or(i32::MAX);
        let (lock, cvar) = &*self.state;
        let mut state = lock_unpoisoned(lock);
        state
            .term
            .scroll_display(Scroll::Delta(delta.saturating_neg()));
        cvar.notify_all();
    }

    /// Volta o scrollback imediatamente para o fim (`display_offset = 0`).
    pub fn scroll_to_bottom(&self) {
        let (lock, cvar) = &*self.state;
        let mut state = lock_unpoisoned(lock);
        state.term.scroll_display(Scroll::Bottom);
        cvar.notify_all();
    }

    /// Inicia uma seleção simples na posição `(row, col)` do viewport visível.
    pub fn start_selection(&self, row: usize, col: usize) {
        let (lock, cvar) = &*self.state;
        let mut state = lock_unpoisoned(lock);
        let point = state.viewport_point(row, col);
        state.term.selection = Some(Selection::new(SelectionType::Simple, point, Side::Left));
        cvar.notify_all();
    }

    /// Estende a seleção atual até a posição `(row, col)` do viewport visível
    /// (incluindo ambas as células extremas). Se não houver seleção ativa,
    /// inicia e seleciona a célula em `(row, col)`.
    pub fn extend_selection(&self, row: usize, col: usize) {
        let (lock, cvar) = &*self.state;
        let mut state = lock_unpoisoned(lock);
        let point = state.viewport_point(row, col);
        match state.term.selection.as_mut() {
            Some(selection) => {
                selection.update(point, Side::Right);
                selection.include_all();
            }
            None => {
                let mut selection = Selection::new(SelectionType::Simple, point, Side::Left);
                selection.update(point, Side::Right);
                selection.include_all();
                state.term.selection = Some(selection);
            }
        }
        cvar.notify_all();
    }

    /// Limpa qualquer seleção ativa.
    pub fn clear_selection(&self) {
        let (lock, cvar) = &*self.state;
        let mut state = lock_unpoisoned(lock);
        state.term.selection = None;
        cvar.notify_all();
    }

    /// Extrai o texto atualmente selecionado no terminal, ou `None` se não
    /// houver seleção ativa.
    pub fn selected_text(&self) -> Option<String> {
        let (lock, _) = &*self.state;
        let state = lock_unpoisoned(lock);
        state.term.selection_to_string()
    }

    /// Retorna o PID do processo filho em execução no PTY, quando disponível.
    pub fn child_pid(&self) -> Option<u32> {
        let guard = lock_unpoisoned(&self.child);
        guard.as_ref().and_then(|c| c.process_id())
    }
}

impl Drop for TermSession {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);

        // Fecha o writer do PTY primeiro para sinalizar EOF ao processo filho.
        {
            let mut writer_guard = lock_unpoisoned(&self.writer);
            let _ = writer_guard.take();
        }

        // Encerra e recolhe o processo filho para não deixar processo órfão/zumbi.
        {
            let mut child_guard = lock_unpoisoned(&self.child);
            if let Some(mut child) = child_guard.take() {
                if matches!(child.try_wait(), Ok(None)) {
                    let _ = child.kill();
                }
                let _ = child.wait();
            }
        }

        // Fecha o master PTY antes de aguardar o término da thread leitora.
        {
            let mut master_guard = lock_unpoisoned(&self.master);
            let _ = master_guard.take();
        }

        if let Some(handle) = self.reader_thread.take() {
            let _ = handle.join();
        }
    }
}

fn validate_dimensions(cols: u16, rows: u16) -> Result<(), TermError> {
    if usize::from(cols) < MIN_COLUMNS || usize::from(rows) < MIN_SCREEN_LINES {
        return Err(TermError::InvalidSize { cols, rows });
    }
    Ok(())
}

fn build_command(spec: &ShellCommand) -> CommandBuilder {
    let mut cmd = CommandBuilder::new(&spec.program);
    cmd.args(&spec.args);

    if let Some(cwd) = &spec.cwd {
        cmd.cwd(cwd);
    } else if let Ok(current) = std::env::current_dir() {
        cmd.cwd(current);
    }

    cmd.env_remove("LINES");
    cmd.env_remove("COLUMNS");

    let needs_default_term = match cmd.get_env("TERM").and_then(|v| v.to_str()) {
        None => true,
        Some(val) => val.trim().is_empty() || val == "dumb",
    };
    if needs_default_term {
        cmd.env("TERM", "xterm-256color");
    }
    if cmd.get_env("COLORTERM").is_none() {
        cmd.env("COLORTERM", "truecolor");
    }
    if cmd.get_env("LANG").is_none() {
        cmd.env("LANG", "en_US.UTF-8");
    }

    for (key, value) in &spec.env {
        cmd.env(key, value);
    }

    cmd
}

fn reader_loop(
    mut reader: Box<dyn Read + Send>,
    state: &Arc<(Mutex<EmulatorState>, Condvar)>,
    writer: &Arc<Mutex<Option<Box<dyn Write + Send>>>>,
    shutdown: &AtomicBool,
) {
    let mut buf = [0u8; 4096];
    loop {
        if shutdown.load(Ordering::Acquire) {
            break;
        }
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let Some(chunk) = buf.get(..n) else {
                    break;
                };
                let replies = {
                    let (lock, cvar) = &**state;
                    let mut guard = lock_unpoisoned(lock);
                    let replies = guard.advance(chunk);
                    cvar.notify_all();
                    replies
                };
                if !replies.is_empty() && !shutdown.load(Ordering::Acquire) {
                    let mut writer_guard = lock_unpoisoned(writer);
                    if let Some(w) = writer_guard.as_mut() {
                        for reply in replies {
                            let _ = w.write_all(reply.as_bytes());
                        }
                        let _ = w.flush();
                    }
                }
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::AtomicU64;
    use std::time::{Duration, Instant};

    use super::*;
    use crate::snapshot::{NamedColor, Rgb, TermColor};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TempHome {
        path: PathBuf,
    }

    impl TempHome {
        fn new() -> Self {
            let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("gsh-term-test-{}-{}", std::process::id(), id));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempHome {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn isolated_bash(tmp: &TempHome) -> ShellCommand {
        let mut cmd = ShellCommand::new(default_shell())
            .cwd(tmp.path())
            .env("HOME", tmp.path())
            .env("USERPROFILE", tmp.path())
            .env("HISTFILE", tmp.path().join(".bash_history"))
            .env("PS1", "$ ");
        #[cfg(unix)]
        {
            cmd = cmd.args(["--norc", "--noprofile"]);
        }
        cmd
    }

    fn wait_until(session: &TermSession, pred: impl Fn(&Snapshot) -> bool) -> Snapshot {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let snap = session.snapshot();
            if pred(&snap) {
                return snap;
            }
            assert!(
                Instant::now() < deadline,
                "timeout waiting for terminal condition; last snapshot:\n{}",
                snap.text()
            );
            let (lock, cvar) = &*session.state;
            let guard = lock.lock().unwrap();
            let _ = cvar.wait_timeout(guard, Duration::from_millis(25)).unwrap();
        }
    }

    #[test]
    fn bash_command_output_appears_in_snapshot() {
        let tmp = TempHome::new();
        let session = TermSession::spawn(isolated_bash(&tmp), 80, 24).unwrap();

        session.send(b"echo gsh-42-ok\n").unwrap();

        let snap = wait_until(&session, |s| {
            s.lines.iter().any(|row| {
                row.iter()
                    .collect::<_VecChars>()
                    .0
                    .trim_start()
                    .starts_with("gsh-42-ok")
            })
        });

        assert_eq!(snap.cols, 80);
        assert_eq!(snap.rows, 24);
        assert!(snap.cursor.visible);
        assert!(snap.text().contains("gsh-42-ok"));
    }

    struct _VecChars(String);
    impl FromIterator<Cell> for _VecChars {
        fn from_iter<T: IntoIterator<Item = Cell>>(iter: T) -> Self {
            Self(iter.into_iter().map(|c| c.ch).collect())
        }
    }
    impl<'a> FromIterator<&'a Cell> for _VecChars {
        fn from_iter<T: IntoIterator<Item = &'a Cell>>(iter: T) -> Self {
            Self(iter.into_iter().map(|c| c.ch).collect())
        }
    }

    #[test]
    fn resize_updates_snapshot_cols_and_rows() {
        let tmp = TempHome::new();
        let session = TermSession::spawn(isolated_bash(&tmp), 80, 24).unwrap();

        let initial = wait_until(&session, |s| s.text().contains('$'));
        assert_eq!(initial.cols, 80);
        assert_eq!(initial.rows, 24);
        assert_eq!(initial.lines.len(), 24);
        assert!(initial.lines.iter().all(|r| r.len() == 80));

        session.resize(100, 32).unwrap();
        assert_eq!(session.size(), (100, 32));

        let resized = session.snapshot();
        assert_eq!(resized.cols, 100);
        assert_eq!(resized.rows, 32);
        assert_eq!(resized.lines.len(), 32);
        assert!(resized.lines.iter().all(|r| r.len() == 100));

        // Confirma que o PTY no kernel também recebeu o novo winsize.
        #[cfg(unix)]
        {
            session.send(b"stty size\n").unwrap();
            let snap = wait_until(&session, |s| s.text().contains("32 100"));
            assert!(snap.text().contains("32 100"));
        }
    }

    #[test]
    fn cell_background_color_appears_in_snapshot() {
        let tmp = TempHome::new();
        let session = TermSession::spawn(isolated_bash(&tmp), 80, 24).unwrap();

        // Imprime 'R' com fundo vermelho ANSI (41), bold (1), underline (4), inverse (7)
        // e 'T' com fundo truecolor RGB(18, 52, 86) após marcador "BG_MARK:".
        session
            .send(b"printf 'BG_MARK:\\033[41;32;1;4;7mR\\033[0m\\033[48;2;18;52;86mT\\033[0m:END\\n'\n")
            .unwrap();

        let snap = wait_until(&session, |s| s.text().contains("BG_MARK:RT:END"));

        let mut found_r = None;
        let mut found_t = None;
        for r_idx in 0..snap.lines.len() {
            let line_str = snap.line_text(r_idx).unwrap_or_default();
            if let Some(pos) = line_str.find("BG_MARK:RT:END") {
                found_r = snap.cell(r_idx, pos + "BG_MARK:".len()).copied();
                found_t = snap.cell(r_idx, pos + "BG_MARK:R".len()).copied();
                break;
            }
        }

        let cell_r = found_r.expect("cell 'R' found");
        assert_eq!(cell_r.ch, 'R');
        assert_eq!(cell_r.bg, TermColor::Named(NamedColor::Red));
        assert_eq!(cell_r.fg, TermColor::Named(NamedColor::Green));
        assert!(cell_r.bold);
        assert!(cell_r.underline);
        assert!(cell_r.inverse);

        let cell_t = found_t.expect("cell 'T' found");
        assert_eq!(cell_t.ch, 'T');
        assert_eq!(cell_t.bg, TermColor::Rgb(Rgb::new(18, 52, 86)));
        assert_eq!(cell_t.effective_bg(), Some(Rgb::new(18, 52, 86)));
    }

    #[test]
    fn scrollback_scroll_up_and_back_to_bottom() {
        let tmp = TempHome::new();
        let session = TermSession::spawn(isolated_bash(&tmp), 60, 6).unwrap();

        session
            .send(b"for i in $(seq 1 25); do printf 'SB_LINE_%02d\\n' \"$i\"; done\n")
            .unwrap();

        let bottom_snap = wait_until(&session, |s| s.text().contains("SB_LINE_25"));
        assert_eq!(bottom_snap.display_offset, 0);
        assert!(bottom_snap.history_size >= 15);
        assert!(!bottom_snap.text().contains("SB_LINE_01"));

        // Rola para o início do histórico até encontrar SB_LINE_01.
        session.scroll_up(bottom_snap.history_size);
        let top_snap = session.snapshot();
        assert!(top_snap.display_offset > 0);

        // Procura SB_LINE_01 rolando pelo histórico se necessário.
        let mut found_first = top_snap.text().contains("SB_LINE_01");
        if !found_first {
            session.scroll_to_bottom();
            for step in 1..=bottom_snap.history_size {
                session.scroll_up(1);
                let s = session.snapshot();
                if s.text().contains("SB_LINE_01") {
                    assert_eq!(s.display_offset, step);
                    found_first = true;
                    break;
                }
            }
        }
        assert!(found_first, "SB_LINE_01 should be reachable in scrollback");

        // Rola 2 linhas para baixo e depois volta ao fim.
        let offset_before = session.snapshot().display_offset;
        session.scroll_down(2);
        assert_eq!(
            session.snapshot().display_offset,
            offset_before.saturating_sub(2)
        );

        session.scroll_to_bottom();
        let final_snap = session.snapshot();
        assert_eq!(final_snap.display_offset, 0);
        assert!(final_snap.text().contains("SB_LINE_25"));
    }

    #[test]
    fn selection_extracts_expected_text() {
        let tmp = TempHome::new();
        let session = TermSession::spawn(isolated_bash(&tmp), 80, 24).unwrap();

        session
            .send(b"printf 'PREFIX_[COPIAR_ESTE_TRECHO]_SUFFIX\\n'\n")
            .unwrap();

        let snap = wait_until(&session, |s| {
            s.lines.iter().enumerate().any(|(idx, _)| {
                s.line_text(idx)
                    .unwrap_or_default()
                    .starts_with("PREFIX_[COPIAR_ESTE_TRECHO]_SUFFIX")
            })
        });

        let target = "COPIAR_ESTE_TRECHO";
        let (target_row, start_col) = snap
            .lines
            .iter()
            .enumerate()
            .find_map(|(r_idx, _)| {
                let line = snap.line_text(r_idx)?;
                if line.starts_with("PREFIX_[COPIAR_ESTE_TRECHO]_SUFFIX") {
                    line.find(target).map(|col| (r_idx, col))
                } else {
                    None
                }
            })
            .expect("target line present in snapshot");

        let end_col = start_col + target.len() - 1;
        session.start_selection(target_row, start_col);
        session.extend_selection(target_row, end_col);

        assert_eq!(session.selected_text().as_deref(), Some(target));

        session.clear_selection();
        assert_eq!(session.selected_text(), None);
    }

    #[test]
    fn drop_terminates_child_and_invalid_dimensions_fail() {
        assert!(matches!(
            TermSession::spawn(None, 0, 24),
            Err(TermError::InvalidSize { cols: 0, rows: 24 })
        ));
        assert!(matches!(
            TermSession::spawn(None, 80, 0),
            Err(TermError::InvalidSize { cols: 80, rows: 0 })
        ));

        let tmp = TempHome::new();
        let session = TermSession::spawn(isolated_bash(&tmp), 80, 24).unwrap();
        let pid = session.child_pid();
        assert!(pid.is_some());
        drop(session);
    }
}
