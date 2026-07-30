use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

const DEFAULT_TARGETS: &[&str] = &[
    "x86_64-pc-windows-msvc",
    "x86_64-unknown-linux-gnu",
    "x86_64-unknown-linux-musl",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("test") => test(),
        Some("ci") => ci(),
        Some("package") => package(args.collect()),
        Some("-h") | Some("--help") | None => {
            print_help();
            Ok(())
        }
        Some(other) => Err(format!("未知 xtask 命令：{other}")),
    }
}

fn test() -> Result<(), String> {
    run_command("cargo", &["fmt", "--check"])?;
    run_command(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run_command("cargo", &["test", "--workspace"])?;
    Ok(())
}

fn ci() -> Result<(), String> {
    test()
}

fn package(args: Vec<String>) -> Result<(), String> {
    let targets = parse_targets(args)?;
    fs::create_dir_all("dist").map_err(|error| error.to_string())?;

    for target in targets {
        run_command("cargo", &["build", "--release", "--target", &target])?;
        copy_artifact(&target)?;
    }

    Ok(())
}

fn parse_targets(args: Vec<String>) -> Result<Vec<String>, String> {
    if args.is_empty() || args.iter().any(|arg| arg == "--all-default-targets") {
        return Ok(DEFAULT_TARGETS
            .iter()
            .map(|target| (*target).to_owned())
            .collect());
    }

    let mut targets = Vec::new();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--target" => {
                let target = iter
                    .next()
                    .ok_or_else(|| "--target 需要指定 target triple".to_owned())?;
                targets.push(target);
            }
            other => return Err(format!("未知 package 參數：{other}")),
        }
    }

    if targets.is_empty() {
        Err("請指定 --target 或 --all-default-targets".into())
    } else {
        Ok(targets)
    }
}

fn copy_artifact(target: &str) -> Result<(), String> {
    let exe_name = if target.contains("windows") {
        "adoctl.exe"
    } else {
        "adoctl"
    };
    let source = Path::new("target")
        .join(target)
        .join("release")
        .join(exe_name);
    if !source.exists() {
        return Err(format!("找不到編譯產物：{}", source.display()));
    }

    let extension = if target.contains("windows") {
        ".exe"
    } else {
        ""
    };
    let destination = PathBuf::from("dist").join(format!("adoctl-{target}{extension}"));
    fs::copy(&source, &destination).map_err(|error| error.to_string())?;
    println!("已產生 {}", destination.display());
    Ok(())
}

fn run_command(program: &str, args: &[&str]) -> Result<(), String> {
    println!("執行：{} {}", program, args.join(" "));
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|error| format!("無法執行 {program}：{error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} 執行失敗", args.join(" ")))
    }
}

fn print_help() {
    println!(
        "adoctl xtask\n\n用法：\n  cargo xtask test\n  cargo xtask ci\n  cargo xtask package --target <target-triple>\n  cargo xtask package --all-default-targets"
    );
}
