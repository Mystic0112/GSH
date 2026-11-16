//! Popup ancorado: desenha `menu` numa camada acima de tudo, logo abaixo do `base` (e acima dele
//! se não couber), como o `showPopover` do shadcn. Clicar fora ou Esc publica `on_dismiss`.
//!
//! O menu é só um `Element`; quem monta os itens é `app_menu`.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::keyboard::key::Named;
use iced::{keyboard, mouse, Element, Event, Length, Point, Rectangle, Size, Vector};

/// Respiro entre o gatilho e o popup (`Offset(0, 4)` do `showAppMenu`).
const GAP: f32 = 4.0;

/// Margem até a borda da janela quando o popup precisa deslizar para caber.
const EDGE_MARGIN: f32 = 8.0;

pub struct Anchored<'a, Message> {
    /// `[base]` ou `[base, menu]`.
    children: Vec<Element<'a, Message>>,
    on_dismiss: Message,
}

impl<'a, Message: Clone> Anchored<'a, Message> {
    pub fn new(
        base: impl Into<Element<'a, Message>>,
        menu: Option<Element<'a, Message>>,
        on_dismiss: Message,
    ) -> Self {
        let mut children = vec![base.into()];
        children.extend(menu);
        Self {
            children,
            on_dismiss,
        }
    }
}

impl<Message: Clone> Widget<Message, iced::Theme, iced::Renderer> for Anchored<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn state(&self) -> tree::State {
        tree::State::None
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        self.children
            .first()
            .map_or(Size::new(Length::Shrink, Length::Shrink), |base| {
                base.as_widget().size()
            })
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        match (self.children.first_mut(), tree.children.first_mut()) {
            (Some(base), Some(state)) => base.as_widget_mut().layout(state, renderer, limits),
            _ => layout::Node::new(Size::ZERO),
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        if let (Some(base), Some(state)) = (self.children.first_mut(), tree.children.first_mut()) {
            base.as_widget_mut()
                .operate(state, layout, renderer, operation);
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
        if let (Some(base), Some(state)) = (self.children.first_mut(), tree.children.first_mut()) {
            base.as_widget_mut().update(
                state, event, layout, cursor, renderer, clipboard, shell, viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        match (self.children.first(), tree.children.first()) {
            (Some(base), Some(state)) => base
                .as_widget()
                .mouse_interaction(state, layout, cursor, viewport, renderer),
            _ => mouse::Interaction::None,
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
        if let (Some(base), Some(state)) = (self.children.first(), tree.children.first()) {
            base.as_widget()
                .draw(state, renderer, theme, style, layout, cursor, viewport);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        let mut children = self.children.iter_mut();
        let mut states = tree.children.iter_mut();
        let base = children.next()?;
        let base_state = states.next()?;
        let inner =
            base.as_widget_mut()
                .overlay(base_state, layout, renderer, viewport, translation);
        let menu = match (children.next(), states.next()) {
            (Some(content), Some(state)) => Some(overlay::Element::new(Box::new(Popup {
                anchor: layout.bounds() + translation,
                content,
                tree: state,
                on_dismiss: self.on_dismiss.clone(),
            }))),
            _ => None,
        };
        let layers: Vec<_> = inner.into_iter().chain(menu).collect();
        (!layers.is_empty()).then(|| overlay::Group::with_children(layers).overlay())
    }
}

struct Popup<'a, 'b, Message> {
    anchor: Rectangle,
    content: &'b mut Element<'a, Message>,
    tree: &'b mut Tree,
    on_dismiss: Message,
}

impl<Message: Clone> overlay::Overlay<Message, iced::Theme, iced::Renderer>
    for Popup<'_, '_, Message>
{
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> layout::Node {
        let limits = layout::Limits::new(Size::ZERO, bounds);
        let node = self
            .content
            .as_widget_mut()
            .layout(self.tree, renderer, &limits);
        let size = node.size();
        let x = self
            .anchor
            .x
            .min(bounds.width - size.width - EDGE_MARGIN)
            .max(0.0);
        let below = self.anchor.y + self.anchor.height + GAP;
        // sem espaço embaixo, abre acima do gatilho (o shadcn inverte sozinho)
        let y = if below + size.height > bounds.height {
            (self.anchor.y - GAP - size.height).max(0.0)
        } else {
            below
        };
        node.move_to(Point::new(x, y))
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.content.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &layout.bounds(),
        );
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let escape = matches!(
            event,
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(Named::Escape),
                ..
            })
        );
        let outside_click = matches!(event, Event::Mouse(mouse::Event::ButtonPressed(_)))
            && !cursor.is_over(layout.bounds());
        if escape || outside_click {
            shell.publish(self.on_dismiss.clone());
            shell.capture_event();
            return;
        }
        self.content.as_widget_mut().update(
            self.tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        );
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            self.tree,
            layout,
            cursor,
            &layout.bounds(),
            renderer,
        )
    }
}

impl<'a, Message: Clone + 'a> From<Anchored<'a, Message>> for Element<'a, Message> {
    fn from(anchored: Anchored<'a, Message>) -> Self {
        Element::new(anchored)
    }
}
