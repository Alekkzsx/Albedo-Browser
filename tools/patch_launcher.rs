use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let exe = match env::current_exe() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("patch launcher: failed to locate executable: {err}");
            return ExitCode::from(1);
        }
    };

    let script = exe
        .parent()
        .map(|dir| dir.join("patch.py"))
        .unwrap_or_else(|| PathBuf::from("patch.py"));

    let status = Command::new("python")
        .arg(script)
        .args(env::args().skip(1))
        .status();

    match status {
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(err) => {
            eprintln!("patch launcher: failed to execute python: {err}");
            ExitCode::from(1)
        }
    }
}
