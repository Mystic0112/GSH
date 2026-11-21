use std::fs;
use std::io;
use std::path::Path;

#[cfg(test)]
pub(crate) fn unique_filename(dir: &Path, input: &str) -> io::Result<String> {
    let cleaned = sanitize_filename(input)?;
    unique_clean_filename(dir, &cleaned)
}

#[cfg(test)]
fn unique_clean_filename(dir: &Path, cleaned: &str) -> io::Result<String> {
    let path = Path::new(&cleaned);
    let stem = path.file_stem().and_then(|v| v.to_str()).unwrap_or("file");
    let extension = path.extension().and_then(|v| v.to_str());
    for suffix in 0u32..u32::MAX {
        let candidate = if suffix == 0 {
            cleaned.to_owned()
        } else if let Some(ext) = extension {
            format!("{stem} ({suffix}).{ext}")
        } else {
            format!("{stem} ({suffix})")
        };
        if !dir.join(&candidate).exists() {
            return Ok(candidate);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "no free filename",
    ))
}

pub(crate) fn persist_without_overwrite(part: &Path, dir: &Path, initial: &str) -> io::Result<()> {
    let cleaned = sanitize_filename(initial)?;
    let path = Path::new(&cleaned);
    let stem = path.file_stem().and_then(|v| v.to_str()).unwrap_or("file");
    let extension = path.extension().and_then(|v| v.to_str());
    for suffix in 0u32..u32::MAX {
        let candidate = if suffix == 0 {
            cleaned.clone()
        } else if let Some(ext) = extension {
            format!("{stem} ({suffix}).{ext}")
        } else {
            format!("{stem} ({suffix})")
        };
        match fs::hard_link(part, dir.join(candidate)) {
            Ok(()) => {
                fs::remove_file(part)?;
                return Ok(());
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "no free filename",
    ))
}

pub fn sanitize_filename(name: &str) -> io::Result<String> {
    let reserved = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9", "COM¹",
        "COM²", "COM³", "LPT¹", "LPT²", "LPT³",
    ];
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains("..")
        || name.contains('/')
        || name.contains('\\')
        || name.starts_with('/')
        || name.contains(':')
        || name
            .chars()
            .any(|ch| matches!(ch, '<' | '>' | '"' | '|' | '?' | '*'))
        || name.chars().any(char::is_control)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unsafe filename",
        ));
    }
    let trimmed = name.trim().trim_end_matches([' ', '.']);
    let stem = trimmed
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end_matches([' ', '.'])
        .to_ascii_uppercase();
    if trimmed.is_empty() || reserved.contains(&stem.as_str()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "reserved filename",
        ));
    }
    Ok(trimmed.to_string())
}
