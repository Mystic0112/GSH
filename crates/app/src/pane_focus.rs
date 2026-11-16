//! Foco de pane por clique (`Listener(onPointerDown: vm.focus)` de `pane_view.dart:126`).
//!
//! Envolve o conteúdo do pane, repassa todos os eventos ao filho e, num clique
//! esquerdo (ou toque) dentro dos próprios limites, publica a mensagem de foco
//! **mesmo que o filho capture o clique** (terminal, editor, chat...), como o
//! `Listener` do Flutter, que observa sem impedir o filho de receber o evento.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::{mouse, touch, Element, Event, Length, Rectangle, Size, Vector};

pub struct FocusOnPress<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    /// `None` quando o pane já é o focado: nada a publicar.
    on_press: Option<Message>,
}

impl<'a, Message, Theme, Renderer> FocusOnPress<'a, Message, Theme, Renderer> {
    pub fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        on_press: Option<Message>,
    ) -> Self {
        Self {
            content: content.into(),
            on_press,
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for FocusOnPress<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
    Message: Clone,
{
    fn tag(&self) -> tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            tree, event, layout, cursor, renderer, clipboard, shell, viewport,
        );

        let pressed = matches!(
            event,
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                | Event::Touch(touch::Event::FingerPressed { .. })
        );
        if let (true, Some(message)) = (pressed, self.on_press.as_ref()) {
            if cursor.is_over(layout.bounds()) {
                shell.publish(message.clone());
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Message, Theme, Renderer> From<FocusOnPress<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a,
    Renderer: 'a + renderer::Renderer,
{
    fn from(widget: FocusOnPress<'a, Message, Theme, Renderer>) -> Self {
        Element::new(widget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::clipboard;
    use iced::widget::{mouse_area, Space};
    use iced::{Point, Theme};

    /// Entrega `event` com o cursor em `cursor_at` a um pane de 100x100 cujo filho
    /// captura o clique (como o canvas do terminal) e publica `1`.
    fn deliver(on_press: Option<u8>, event: Event, cursor_at: Point) -> Vec<u8> {
        let child: Element<'_, u8, Theme, ()> = mouse_area(Space::new()).on_press(1).into();
        let mut element: Element<'_, u8, Theme, ()> =
            Element::new(FocusOnPress::new(child, on_press));
        let mut tree = Tree::new(&element);
        let node = layout::Node::new(Size::new(100.0, 100.0));
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        element.as_widget_mut().update(
            &mut tree,
            &event,
            Layout::new(&node),
            mouse::Cursor::Available(cursor_at),
            &(),
            &mut clipboard::Null,
            &mut shell,
            &Rectangle::new(Point::ORIGIN, Size::new(100.0, 100.0)),
        );
        messages
    }

    fn press() -> Event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
    }

    #[test]
    fn publica_o_foco_mesmo_quando_o_filho_captura_o_clique() {
        let messages = deliver(Some(7), press(), Point::new(50.0, 50.0));
        assert_eq!(messages, vec![1, 7]);
    }

    #[test]
    fn nao_publica_quando_o_pane_ja_esta_focado() {
        assert_eq!(deliver(None, press(), Point::new(50.0, 50.0)), vec![1]);
    }

    #[test]
    fn nao_publica_com_o_cursor_fora_ou_sem_ser_clique_esquerdo() {
        assert!(deliver(Some(7), press(), Point::new(150.0, 50.0)).is_empty());
        let release = Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
        assert!(deliver(Some(7), release, Point::new(50.0, 50.0)).is_empty());
        let right = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right));
        assert!(deliver(Some(7), right, Point::new(50.0, 50.0)).is_empty());
    }
}
