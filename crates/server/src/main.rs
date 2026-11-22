#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
mod protocol;
#[cfg(target_os = "linux")]
mod pty;

fn main() -> std::process::ExitCode {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args == ["--version"] {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return std::process::ExitCode::SUCCESS;
    }
    if args == ["--help"] || args == ["-h"] {
        println!("Usage: gsh-server [--socket ABSOLUTE_PATH]");
        return std::process::ExitCode::SUCCESS;
    }
    match run(&args) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("gsh-server: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(target_os = "linux")]
fn run(args: &[std::ffi::OsString]) -> std::io::Result<()> {
    use std::{io, path::PathBuf};

    let path = match args {
        [] => PathBuf::from(
            std::env::var_os("HOME")
                .ok_or_else(|| io::Error::other("HOME is unset; pass --socket"))?,
        )
        .join(".gsh")
        .join("run")
        .join("gsh-server.sock"),
        [flag, path] if flag == "--socket" => PathBuf::from(path),
        _ => {
            return Err(io::Error::other(
                "usage: gsh-server [--socket ABSOLUTE_PATH]",
            ))
        }
    };
    linux::serve(&path)
}

#[cfg(not(target_os = "linux"))]
fn run(_args: &[std::ffi::OsString]) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "the terminal server is available on Linux",
    ))
}
