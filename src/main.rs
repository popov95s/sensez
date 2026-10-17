fn main() -> std::process::ExitCode {
    sensez::run_cli().unwrap_or_else(|err| {
        eprintln!("Error: {err:?}");
        std::process::ExitCode::from(2)
    })
}
