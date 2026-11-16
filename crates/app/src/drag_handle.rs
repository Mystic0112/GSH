//! Alça de arrasto: publica o quanto o cursor andou enquanto o botão esquerdo segue apertado.
//!
//! É o `GestureDetector(onHorizontalDragStart/Update)` e `onVerticalDragUpdate` do Flutter, usado
//! no divisor editor/resultado, no divisor de coluna da grade e na alça do campo da connection string.

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::{mouse, Element, Event, Length, Point, Rectangle, Size};

/// Eixo da alça (decide o cursor e qual componente do movimento é publicado).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

type Delta<'a, Message> = Box<dyn Fn(f32) -> Message + 'a>;

pub struct DragHandle<'a, Message> {
    content: Element<'a, Message>,
    axis: Axis,
    on_start: Option<Message>,
    on_drag: Delta<'a, Message>,
    on_reset: Option<Message>,
}

impl<'a, Message> DragHandle<'a, Message> {
    pub fn new(
        content: impl Into<Element<'a, Message>>,
        axis: Axis,
        on_drag: impl Fn(f32) -> Message + 'a,
    ) -> Self {
        Self {
            content: content.into(),
            axis,
            on_start: None,
            on_drag: Box::new(on_drag),
            on_reset: None,
        }
    }

    /// Publicado ao apertar o botão (início do arrasto).
    pub fn on_start(mut self, message: Message) -> Self {
        self.on_start = Some(message);
        self
    }

    /// Publicado no duplo clique.
    pub fn on_reset(mut self, message: Message) -> Self {
        self.on_reset = Some(message);
        self
    }
}

#[derive(Default)]
struct State {
    /// Posição do cursor no último evento, enquanto arrasta.
    dragging_from: Option<Point>,
    last_click: Option<std::time::Instant>,
}

/// Dois cliques em menos disto contam como duplo clique.
const DOUBLE_CLICK: std::time::Duration = std::time::Duration::from_millis(400);

impl<Message: Clone> Widget<Message, iced::Theme, iced::Renderer> for DragHandle<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let child = tree.children.first_mut();
        match child {
            Some(child) => {
                let node = self.content.as_widget_mut().layout(child, renderer, limits);
                layout::Node::with_children(node.size(), vec![node])
            }
            None => layout::Node::new(Size::ZERO),
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        if let (Some(child), Some(layout)) = (tree.children.first_mut(), layout.children().next()) {
            self.content
                .as_widget_mut()
                .operate(child, layout, renderer, operation);
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(layout.bounds()) =>
            {
                let now = std::time::Instant::now();
                let double = state
                    .last_click
                    .is_some_and(|before| now.duration_since(before) < DOUBLE_CLICK);
                state.last_click = Some(now);
                if double {
                    state.last_click = None;
                    if let Some(message) = &self.on_reset {
                        shell.publish(message.clone());
                    }
                }
                state.dragging_from = cursor.position();
                if let Some(message) = &self.on_start {
                    shell.publish(message.clone());
                }
                shell.capture_event();
                return;
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some(from) = state.dragging_from {
                    let delta = match self.axis {
                        Axis::Horizontal => position.x - from.x,
                        Axis::Vertical => position.y - from.y,
                    };
                    state.dragging_from = Some(*position);
                    if delta != 0.0 {
                        shell.publish((self.on_drag)(delta));
                    }
                    shell.capture_event();
                    return;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.dragging_from.is_some() =>
            {
                state.dragging_from = None;
                shell.capture_event();
                return;
            }
            _ => {}
        }
        if let (Some(child), Some(layout)) = (tree.children.first_mut(), layout.children().next()) {
            self.content.as_widget_mut().update(
                child, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let dragging = tree.state.downcast_ref::<State>().dragging_from.is_some();
        if dragging || cursor.is_over(layout.bounds()) {
            match self.axis {
                Axis::Horizontal => mouse::Interaction::ResizingHorizontally,
                Axis::Vertical => mouse::Interaction::ResizingVertically,
            }
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if let (Some(child), Some(layout)) = (tree.children.first(), layout.children().next()) {
            self.content
                .as_widget()
                .draw(child, renderer, theme, style, layout, cursor, viewport);
        }
    }
}

impl<'a, Message: Clone + 'a> From<DragHandle<'a, Message>> for Element<'a, Message> {
    fn from(handle: DragHandle<'a, Message>) -> Self {
        Element::new(handle)
    }
}
