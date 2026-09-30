use std::io::Write;

fn main() {
    let arguments = std::env::args_os().collect();
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let code = stencil_cli::run(arguments, &mut stdout, &mut stderr);
    // process::exit skips destructors, so buffered output is flushed here first.
    let _ = stdout.flush();
    let _ = stderr.flush();
    std::process::exit(code as i32);
}
