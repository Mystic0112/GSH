use super::*;

#[test]
fn editing_and_history_preserve_unicode_and_selection() {
    let mut b = Buffer::new("aé\n".into());
    assert!(b.select(2, 2).is_err());
    b.select(3, 3).unwrap();
    b.select(1, 3).unwrap();
    b.insert("");
    assert_eq!(b.text(), "a\n");
    b.undo();
    assert_eq!(b.text(), "aé\n");
    b.redo();
    assert_eq!(b.text(), "a\n");
    b.select(1, 2).unwrap();
    b.insert("");
    assert_eq!(b.text(), "a");
    b.insert("\n");
    b.insert("    ");
    b.insert("x");
    assert_eq!(b.text(), "a\n    x");
    b.select(2, 6).unwrap();
    b.insert("!");
    assert_eq!(b.text(), "a\n!x");
    assert!(b.is_dirty());
}

#[test]
fn save_is_atomic_and_tracks_saved_revision() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.txt");
    std::fs::write(&path, "old").unwrap();
    let mut b = Buffer::new("old".into());
    b.insert("new");
    b.save(&path).unwrap();
    assert_eq!(std::fs::read_to_string(path).unwrap(), "newold");
    assert!(!b.is_dirty());
    b.undo();
    assert!(b.is_dirty());
    b.redo();
    assert!(!b.is_dirty());
}

#[cfg(unix)]
#[test]
fn readonly_and_failed_saves_preserve_original() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("readonly");
    std::fs::write(&path, "original").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).unwrap();
    let mut buffer = Buffer::new("new".into());
    assert!(matches!(buffer.save(&path), Err(EditorError::ReadOnly)));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");
    assert!(buffer.save(&dir.path().join("missing/file")).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "original");
}

#[test]
fn typing_groups_and_history_is_bounded() {
    let mut buffer = Buffer::new(String::new());
    for value in ["a", "é", "b"] {
        assert!(buffer.insert(value));
    }
    buffer.undo();
    assert_eq!(buffer.text(), "");
    buffer.redo();
    assert_eq!(buffer.text(), "aéb");
    for _ in 0..600 {
        buffer.break_group();
        assert!(buffer.insert("x"));
    }
    for _ in 0..600 {
        buffer.undo();
    }
    assert_eq!(buffer.text().chars().count(), 103);
}
