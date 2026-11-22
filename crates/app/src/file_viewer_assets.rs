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

pub fn strip_frontmatter(markdown: &str) -> &str {
    let Some(after_open) = markdown
        .strip_prefix("---\n")
        .or_else(|| markdown.strip_prefix("---\r\n"))
    else {
        return markdown;
    };
    if let Some(end) = after_open
        .find("\r\n---\r\n")
        .or_else(|| after_open.find("\r\n...\r\n"))
    {
        return after_open.get(end + 7..).unwrap_or(markdown);
    }
    let Some(end) = after_open
        .find("\n---\n")
        .or_else(|| after_open.find("\n...\n"))
    else {
        return markdown;
    };
    after_open.get(end + 5..).unwrap_or(markdown)
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
    fn removes_closed_yaml_frontmatter_and_keeps_plain_markdown() {
        assert_eq!(
            strip_frontmatter("---\ntitle: Notes\n---\n# Body"),
            "# Body"
        );
        assert_eq!(strip_frontmatter("# Plain"), "# Plain");
        assert_eq!(
            strip_frontmatter("---\r\ntitle: Notes\r\n...\r\n# Body"),
            "# Body"
        );
        assert_eq!(
            strip_frontmatter("---\ntitle: unfinished\n# Body"),
            "---\ntitle: unfinished\n# Body"
        );
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
