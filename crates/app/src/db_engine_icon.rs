//! Logo (SVG de marca, Devicon) do motor de banco (`db_engine_icon.dart`): painel, menu do "+"
//! e a barra da aba de query. Cada motor tem um asset em `assets/db_icons/<id>.svg`.

use gsh_db::Engine;
use iced::widget::svg;
use iced::Element;

fn bytes(engine: Engine) -> &'static [u8] {
    match engine {
        Engine::Sqlite => include_bytes!("../assets/db_icons/sqlite.svg"),
        Engine::Postgres => include_bytes!("../assets/db_icons/postgres.svg"),
        Engine::Mysql => include_bytes!("../assets/db_icons/mysql.svg"),
        Engine::Mssql => include_bytes!("../assets/db_icons/mssql.svg"),
        Engine::Redis => include_bytes!("../assets/db_icons/redis.svg"),
        Engine::Mongo => include_bytes!("../assets/db_icons/mongo.svg"),
    }
}

/// Logo do motor num quadrado de `size` px.
pub fn view<'a, M: 'a>(engine: Engine, size: f32) -> Element<'a, M> {
    svg(svg::Handle::from_memory(bytes(engine)))
        .width(size)
        .height(size)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_engine_has_an_svg_asset() {
        for engine in Engine::ALL {
            let text = std::str::from_utf8(bytes(engine)).unwrap();
            assert!(text.contains("<svg"), "{engine:?}");
            let _: Element<'_, ()> = view(engine, 13.0);
        }
    }
}
