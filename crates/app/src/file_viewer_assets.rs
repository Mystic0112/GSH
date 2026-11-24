use std::path::Path;

pub fn is_media(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some(
            "mp3"
                | "wav"
                | "flac"
                | "ogg"
                | "m4a"
                | "aac"
                | "mp4"
                | "mkv"
                | "webm"
                | "mov"
                | "avi"
                | "m4v"
        )
    )
}

pub fn is_image(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("png" | "jpg" | "jpeg")
    )
}

pub fn is_svg(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
}

pub fn is_markdown(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownFrontmatter {
    pub fields: Vec<(String, String)>,
    pub body: String,
}

/// Extracts the leading YAML block and leaves ordinary Markdown untouched.
pub fn split_frontmatter(markdown: &str) -> Option<MarkdownFrontmatter> {
    let normalized = markdown.replace("\r\n", "\n").replace('\r', "\n");
    let normalized = normalized.strip_prefix('\u{feff}').unwrap_or(&normalized);
    let lead = normalized.len() - normalized.trim_start_matches([' ', '\t', '\n']).len();
    let candidate = normalized.get(lead..)?;
    let (opening, tail) = candidate.split_once('\n')?;
    if opening.trim_end() != "---" {
        return None;
    }
    let mut yaml_lines = Vec::new();
    let mut body_start = None;
    let mut offset = opening.len() + 1;
    for line in tail.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        if matches!(content.trim_end(), "---" | "...") {
            body_start = Some(offset + line.len());
            break;
        }
        yaml_lines.push(content);
        offset = offset.saturating_add(line.len());
    }
    let body_start = body_start?;
    let yaml = yaml_lines.join("\n");
    let documents = yaml_rust::YamlLoader::load_from_str(&yaml).ok()?;
    let [document] = documents.as_slice() else {
        return None;
    };
    let mapping = document.as_hash()?;
    let fields = mapping
        .iter()
        .map(|(key, value)| (format_yaml_value(key), format_yaml_value(value)))
        .collect::<Vec<_>>();
    if fields.is_empty() {
        return None;
    }
    Some(MarkdownFrontmatter {
        fields,
        body: normalized
            .get(lead.saturating_add(body_start)..)?
            .to_owned(),
    })
}

fn format_yaml_value(value: &yaml_rust::Yaml) -> String {
    use yaml_rust::Yaml;

    match value {
        Yaml::String(value) => value.clone(),
        Yaml::Integer(value) => value.to_string(),
        Yaml::Real(value) => value.clone(),
        Yaml::Boolean(value) => value.to_string(),
        Yaml::Null | Yaml::BadValue => String::new(),
        Yaml::Array(values) => values
            .iter()
            .map(format_yaml_value)
            .collect::<Vec<_>>()
            .join(", "),
        Yaml::Hash(values) => values
            .iter()
            .map(|(key, value)| format!("{}: {}", format_yaml_value(key), format_yaml_value(value)))
            .collect::<Vec<_>>()
            .join(", "),
        Yaml::Alias(_) => String::new(),
    }
}

pub fn preview_image(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| error.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let thumbnail = reader
        .decode()
        .map_err(|error| error.to_string())?
        .thumbnail(1600, 1200);
    let mut output = std::io::Cursor::new(Vec::new());
    thumbnail
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    Ok(output.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_yaml_frontmatter_and_formats_nested_values_like_flutter() {
        let parsed = split_frontmatter(
            "\u{feff} \n---\r\ntitle: Notes\r\ntags:\r\n  - rust\r\n  - ui\r\nmetadata: { owner: helio, active: true }\r\n...\r\n# Body",
        )
        .unwrap();
        assert_eq!(parsed.body, "# Body");
        assert_eq!(
            parsed.fields.first(),
            Some(&("title".to_owned(), "Notes".to_owned()))
        );
        assert_eq!(
            parsed.fields.get(1),
            Some(&("tags".to_owned(), "rust, ui".to_owned()))
        );
        assert_eq!(
            parsed.fields.get(2),
            Some(&(
                "metadata".to_owned(),
                "owner: helio, active: true".to_owned()
            ))
        );
        assert!(split_frontmatter("# Plain").is_none());
        assert!(split_frontmatter("---\ntitle: unfinished\n# Body").is_none());
    }

    #[test]
    fn creates_a_bounded_png_preview_from_jpeg_or_png_bytes() {
        let mut input = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(2, 2)
            .write_to(&mut input, image::ImageFormat::Jpeg)
            .unwrap();
        let preview = preview_image(input.into_inner()).unwrap();
        assert_eq!(&preview[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn recognizes_linux_media_extensions_case_insensitively() {
        assert!(is_media(Path::new("clip.MP4")));
        assert!(!is_media(Path::new("notes.mp4.txt")));
    }
}
