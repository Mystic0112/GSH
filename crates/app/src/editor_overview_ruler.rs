//! Marcas compactas proporcionais ao documento, sobre a margem da scrollbar iced.

use iced::widget::canvas::{self, Action, Event, Path};
use iced::{mouse, Color, Point, Rectangle, Renderer, Size, Theme};

#[derive(Debug, Clone, Copy)]
pub struct Mark {
    pub line: usize,
    pub lane: usize,
    pub color: Color,
}

pub struct Ruler<Message> {
    pub line_count: usize,
    pub marks: Vec<Mark>,
    pub on_jump: fn(usize) -> Message,
}

pub fn y_for_line(line: usize, line_count: usize, height: f32) -> f32 {
    if line_count <= 1 {
        return 0.0;
    }
    (line.saturating_sub(1) as f32 / line_count.saturating_sub(1) as f32).clamp(0.0, 1.0) * height
}

impl<Message: 'static> canvas::Program<Message> for Ruler<Message> {
    type State = ();

    fn update(
        &self,
        _: &mut (),
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        if !matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        ) || self.line_count == 0
        {
            return None;
        }
        let point = cursor.position_in(bounds)?;
        let line = if self.line_count <= 1 {
            1
        } else {
            ((point.y / bounds.height.max(1.0)) * self.line_count.saturating_sub(1) as f32).round()
                as usize
                + 1
        };
        Some(Action::publish((self.on_jump)(line)))
    }

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        for mark in &self.marks {
            let y = y_for_line(mark.line, self.line_count, bounds.height)
                .min((bounds.height - 2.0).max(0.0));
            let x = mark.lane.min(2) as f32 * 2.5;
            frame.fill(
                &Path::rounded_rectangle(Point::new(x, y), Size::new(2.5, 3.0), 1.0.into()),
                mark.color,
            );
        }
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_document_lines_to_ruler_track() {
        assert_eq!(y_for_line(1, 101, 500.0), 0.0);
        assert_eq!(y_for_line(51, 101, 500.0), 250.0);
        assert_eq!(y_for_line(101, 101, 500.0), 500.0);
        assert_eq!(y_for_line(1, 1, 500.0), 0.0);
    }
}
