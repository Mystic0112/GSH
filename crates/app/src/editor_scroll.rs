//! Cálculo da rolagem necessária para acompanhar o cursor.
pub fn follow_delta(line_y: f32, line_height: f32, scroll_y: f32, viewport_height: f32) -> f32 {
    if line_y < scroll_y {
        line_y - scroll_y
    } else if line_y + line_height > scroll_y + viewport_height {
        line_y + line_height - scroll_y - viewport_height
    } else {
        0.0
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn follows_cursor_at_both_edges() {
        assert_eq!(follow_delta(5.0, 10.0, 10.0, 100.0), -5.0);
        assert_eq!(follow_delta(108.0, 10.0, 10.0, 100.0), 8.0);
        assert_eq!(follow_delta(20.0, 10.0, 10.0, 100.0), 0.0);
    }
}
