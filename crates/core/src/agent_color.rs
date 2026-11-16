//! Cor estável de cada agente (aba), a mesma na árvore de arquivos e no grafo de agentes
//! (`agent_colors.dart` do Flutter).
//!
//! Paleta fixa de tons médios (Tailwind 500), fora do tema de propósito: a cor identifica a
//! ABA e não muda com o tema ativo. A cor escolhida à mão no grafo (que ainda não existe aqui)
//! vence o hash; quando o grafo chegar, ele consulta a escolha e cai em [`agent_color`].

/// Cor RGB de 8 bits por canal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentRgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

const fn rgb(r: u8, g: u8, b: u8) -> AgentRgb {
    AgentRgb { r, g, b }
}

/// Mesma ordem e mesmos valores de `agentPalette`.
pub const AGENT_PALETTE: [AgentRgb; 8] = [
    rgb(0x3B, 0x82, 0xF6), // blue
    rgb(0x10, 0xB9, 0x81), // emerald
    rgb(0xF5, 0x9E, 0x0B), // amber
    rgb(0xF4, 0x3F, 0x5E), // rose
    rgb(0x8B, 0x5C, 0xF6), // violet
    rgb(0x06, 0xB6, 0xD4), // cyan
    rgb(0xF9, 0x73, 0x16), // orange
    rgb(0xEC, 0x48, 0x99), // pink
];

const FNV_OFFSET: u32 = 0x811c_9dc5;
const FNV_PRIME: u32 = 0x0100_0193;

/// Índice na paleta: FNV-1a de 32 bits sobre as unidades UTF-16 do id (como `codeUnits` do
/// Dart), módulo o tamanho da paleta.
pub fn agent_color_index(tab_id: &str) -> usize {
    let hash = tab_id.encode_utf16().fold(FNV_OFFSET, |h, unit| {
        (h ^ u32::from(unit)).wrapping_mul(FNV_PRIME)
    });
    usize::try_from(hash).unwrap_or(0) % AGENT_PALETTE.len()
}

/// Cor do agente da aba `tab_id`: mesmo id, mesma cor, em qualquer execução.
pub fn agent_color(tab_id: &str) -> AgentRgb {
    AGENT_PALETTE
        .get(agent_color_index(tab_id))
        .copied()
        .unwrap_or(rgb(0x3B, 0x82, 0xF6))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vetores gerados pelo `agentColorFor` do Flutter (`dart run`), incluindo vazio, acento e
    /// par substituto (emoji), que o Dart hasheia como duas unidades UTF-16.
    #[test]
    fn indice_igual_ao_dart() {
        let vetores = [
            ("t1", 6),
            ("t2", 3),
            ("t3", 0),
            ("t4", 5),
            ("t5", 2),
            ("t6", 7),
            ("t7", 4),
            ("t8", 1),
            ("t9", 6),
            ("t10", 2),
            ("t11", 5),
            ("", 5),
            ("claude-1", 5),
            ("ação", 3),
            ("😀", 0),
        ];
        for (id, esperado) in vetores {
            assert_eq!(agent_color_index(id), esperado, "id {id:?}");
        }
    }

    #[test]
    fn cor_e_estavel_e_vem_da_paleta() {
        assert_eq!(agent_color("t1"), rgb(0xF9, 0x73, 0x16));
        assert_eq!(agent_color("t1"), agent_color("t1"));
        assert_eq!(agent_color("t3"), rgb(0x3B, 0x82, 0xF6));
    }
}
