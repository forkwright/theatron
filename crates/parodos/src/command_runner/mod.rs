pub(crate) fn output(program: &str, args: &[&str]) -> std::io::Result<std::process::Output> {
    std::process::Command::new(program)
        .args(args.iter().copied())
        .output()
}
