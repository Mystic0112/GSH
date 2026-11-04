//! Placeholder da CLI `gsh`. A porta real vem no card "[F0] Portar o crate cli/".
//! Saída em inglês: quem lê é agente e script.

use std::process::ExitCode;

fn run(args: &[String]) -> Result<String, String> {
    match args.first().map(String::as_str) {
        None => Err("missing verb; usage: gsh <verb> [args] (try `gsh --version`)".to_string()),
        Some("--version" | "-V" | "version") => Ok(format!("gsh {}", gsh_core::VERSION)),
        Some(other) => Err(format!("unknown verb: {other}")),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(out) => {
            println!("{out}");
            ExitCode::SUCCESS
        }
        Err(msg) => {
            eprintln!("gsh: {msg}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_verb_is_an_error() {
        assert!(run(&[]).is_err());
    }

    #[test]
    fn version_verb_prints_version() {
        assert_eq!(
            run(&["--version".to_string()]),
            Ok(format!("gsh {}", gsh_core::VERSION))
        );
    }
}
