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
/// Rolagem que deixa a linha `index` (base 0) 80px abaixo do topo, sem passar do início
/// (`(idx * lineHeight - 80).clamp(0, max)` do Flutter; o máximo é do próprio scrollable).
pub fn reveal_offset(index: usize, line_height: f32) -> f32 {
    (index as f32 * line_height - crate::theme::EDITOR_REVEAL_MARGIN).max(0.0)
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
    #[test]
    fn reveal_keeps_the_line_80px_from_the_top_and_never_goes_negative() {
        assert_eq!(reveal_offset(0, 20.0), 0.0);
        assert_eq!(reveal_offset(3, 20.0), 0.0);
        assert_eq!(reveal_offset(10, 20.0), 120.0);
    }
}
