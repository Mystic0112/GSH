//! Saved feedback is scoped to one repository and the exact diff being reviewed.
use gsh_git::FileDiff;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

pub type Notes = BTreeMap<(usize, usize), String>;

#[derive(Serialize, Deserialize)]
struct SavedNotes {
    revision: String,
    notes: Vec<((usize, usize), String)>,
}

#[derive(Debug)]
pub struct NoteFile {
    path: PathBuf,
    revision: String,
}

impl NoteFile {
    pub fn new(directory: &Path, repo: &Path, diff: &FileDiff) -> io::Result<Self> {
        let hunks = diff.hunks.iter().map(|hunk| {
            let lines = hunk
                .lines
                .iter()
                .map(|line| (line.kind as u8, line.old_line, line.new_line, &line.text));
            (&hunk.header, lines.collect::<Vec<_>>())
        });
        let revision = serde_json::to_string(&(
            repo,
            &diff.path,
            diff.staged,
            diff.kind as u8,
            hunks.collect::<Vec<_>>(),
        ))?;
        // The filename uses FNV-1a for a stable lookup. Reads also verify the full revision.
        let key = revision.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        });
        Ok(Self {
            path: directory.join(format!("{key:016x}.json")),
            revision,
        })
    }

    pub fn load(&self) -> io::Result<Notes> {
        let data = match fs::read(&self.path) {
            Ok(data) => data,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Notes::new()),
            Err(error) => return Err(error),
        };
        let saved: SavedNotes = serde_json::from_slice(&data)?;
        if saved.revision != self.revision {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "diff incompatível com as notas salvas",
            ));
        }
        Ok(saved.notes.into_iter().collect())
    }

    pub fn save(&self, previous: &Notes, notes: &Notes) -> io::Result<()> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| io::Error::other("pasta de notas inválida"))?;
        fs::create_dir_all(parent)?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(parent.join(".lock"))?;
        lock.lock()?;
        if &self.load()? != previous {
            return Err(io::Error::other(
                "as notas mudaram em outra janela. Reabra o diff antes de salvar",
            ));
        }
        if previous == notes {
            return Ok(());
        }
        if notes.is_empty() {
            return fs::remove_file(&self.path);
        }
        let saved = SavedNotes {
            revision: self.revision.clone(),
            notes: notes.clone().into_iter().collect(),
        };
        #[cfg(target_os = "linux")]
        {
            let mut file = tempfile::NamedTempFile::new_in(parent)?;
            serde_json::to_writer(&mut file, &saved)?;
            file.as_file().sync_all()?;
            file.persist(&self.path)?;
        }
        #[cfg(not(target_os = "linux"))]
        fs::write(&self.path, serde_json::to_vec(&saved)?)?;
        Ok(())
    }
}
