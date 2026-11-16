//! `ipv4InCidr` de lan_team_farm_channel.dart, mesma tabela de casos.

fn ipv4(s: &str) -> Option<i64> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let mut v = 0_i64;
    for p in parts {
        let n = p.parse::<i64>().ok().filter(|n| (0..=255).contains(n))?;
        v = (v << 8) | n;
    }
    Some(v)
}

/// `192.168.10.37` esta em `192.168.10.0/24`? CIDR invalido = nao.
pub fn ipv4_in_cidr(ip: &str, cidr: &str) -> bool {
    let parts: Vec<&str> = cidr.trim().split('/').collect();
    // como o Dart: so com exatamente 2 partes o prefixo vem do texto, senao e /32
    let bits = if let [_, b] = parts.as_slice() {
        b.parse::<i64>().ok()
    } else {
        Some(32)
    };
    let (Some(net), Some(addr), Some(bits)) = (parts.first().and_then(|n| ipv4(n)), ipv4(ip), bits)
    else {
        return false;
    };
    if !(0..=32).contains(&bits) {
        return false;
    }
    let mask = if bits == 0 {
        0
    } else {
        (0xFFFF_FFFF_i64 << (32 - bits)) & 0xFFFF_FFFF
    };
    (net & mask) == (addr & mask)
}
