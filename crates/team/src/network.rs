//! "Rede do trabalho": o GSH so anuncia e escuta se uma interface IPv4 desta maquina cai no
//! CIDR configurado (`_onAllowedNetwork` do Dart).

use crate::cidr::ipv4_in_cidr;
use std::net::Ipv4Addr;

/// Vazio = qualquer rede.
pub fn allowed_network(cidr: &str) -> bool {
    cidr.trim().is_empty()
        || local_ipv4s()
            .iter()
            .any(|ip| ipv4_in_cidr(&ip.to_string(), cidr))
}

#[cfg(unix)]
#[allow(unsafe_code)] // FFI do getifaddrs: unica borda unsafe do crate, isolada aqui
fn local_ipv4s() -> Vec<Ipv4Addr> {
    let mut head: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: `getifaddrs` so escreve em `head` o ponteiro da lista que ele mesmo aloca; nada
    // e lido antes de ele retornar 0.
    if unsafe { libc::getifaddrs(&mut head) } != 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut cur = head;
    while !cur.is_null() {
        // SAFETY: `cur` vem da lista devolvida por `getifaddrs`, valida ate o `freeifaddrs`.
        let ifa = unsafe { &*cur };
        let addr = ifa.ifa_addr;
        // SAFETY: `ifa_addr`, se nao nulo, aponta para um `sockaddr` valido; so e relido como
        // `sockaddr_in` depois de conferir `sa_family == AF_INET`.
        if !addr.is_null() && i32::from(unsafe { (*addr).sa_family }) == libc::AF_INET {
            // SAFETY: a familia e AF_INET, logo o objeto e um `sockaddr_in` completo.
            let sin = unsafe { &*addr.cast::<libc::sockaddr_in>() };
            out.push(Ipv4Addr::from(u32::from_be(sin.sin_addr.s_addr)));
        }
        cur = ifa.ifa_next;
    }
    // SAFETY: `head` veio de um `getifaddrs` bem-sucedido e e liberado uma unica vez.
    unsafe { libc::freeifaddrs(head) };
    out
}

/// Fora do Unix nao ha `getifaddrs` na stdlib: o endereco da interface de saida (o que o
/// multicast usaria) e o que se consegue sem dependencia nova.
#[cfg(not(unix))]
fn local_ipv4s() -> Vec<Ipv4Addr> {
    use std::net::{IpAddr, UdpSocket};
    let probe = || -> std::io::Result<IpAddr> {
        let s = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        s.connect((crate::channel::GROUP, crate::channel::PORT))?;
        Ok(s.local_addr()?.ip())
    };
    match probe() {
        Ok(IpAddr::V4(ip)) => vec![ip],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vazio_libera_qualquer_rede_e_cidr_invalido_bloqueia() {
        assert!(allowed_network(""));
        assert!(allowed_network("   "));
        assert!(!allowed_network("lixo"));
    }

    #[cfg(unix)]
    #[test]
    fn loopback_esta_entre_as_interfaces() {
        // o harness roda em maquina com `lo`; getifaddrs tem que enxerga-lo
        assert!(local_ipv4s().contains(&Ipv4Addr::LOCALHOST));
        assert!(allowed_network("127.0.0.0/8"));
    }
}
