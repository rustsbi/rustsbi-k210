mod detect;
mod test;

use clap::{clap_app, crate_authors, crate_description, crate_version};
use std::{
    env, fs,
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{self, Command},
};

#[derive(Debug)]
struct XtaskEnv {
    compile_mode: CompileMode,
}

#[derive(Debug)]
enum CompileMode {
    Debug,
    Release,
}

const DEFAULT_TARGET: &'static str = "riscv64imac-unknown-none-elf";

fn main() {
    let matches = clap_app!(xtask =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@subcommand make =>
            (about: "Build project")
            (@arg release: --release "Build artifacts in release mode, with optimizations")
        )
        (@subcommand k210 =>
            (about: "Run project on actual board")
            (@arg release: --release "Build artifacts in release mode, with optimizations")
        )
        (@subcommand detect =>
            (about: "Detect target serial port")
        )
        (@subcommand asm =>
            (about: "View asm code for project")
        )
        (@subcommand size =>
            (about: "View size for project")
        )
    )
    .get_matches();
    let mut xtask_env = XtaskEnv {
        compile_mode: CompileMode::Debug,
    };
    // Read: python xtask/ktool.py -p COM11 -a 0x80000000 -R -L 0x20000 ./target/xtask/flash_dump.bin
    if let Some(matches) = matches.subcommand_matches("k210") {
        let port = match detect::read_serial_port_choose_file() {
            Ok(string) => {
                println!("xtask: using previously selected serial port {}.", string);
                string
            }
            Err(_e) => detect_save_port_or_exit(),
        };
        let ktool_exists = fs::metadata(project_root().join("xtask").join("ktool.py")).is_ok();
        if !ktool_exists {
            eprintln!(
                "xtask: ktool.py file not found
    To install ktool.py, download from https://github.com/loboris/ktool,
    then copy ktool.py file into path xtask/ktool.py."
            );
            process::exit(1);
        }
        if matches.is_present("release") {
            xtask_env.compile_mode = CompileMode::Release;
        }
        println!("xtask: mode: {:?}", xtask_env.compile_mode);
        xtask_build_sbi(&xtask_env);
        xtask_binary_sbi(&xtask_env);
        xtask_build_test_kernel(&xtask_env);
        xtask_binary_test_kernel(&xtask_env);
        xtask_fuse_binary(&xtask_env);
        xtask_run_k210(&xtask_env, &port);
    } else if let Some(matches) = matches.subcommand_matches("make") {
        if matches.is_present("release") {
            xtask_env.compile_mode = CompileMode::Release;
        }
        println!("xtask: mode: {:?}", xtask_env.compile_mode);
        xtask_build_sbi(&xtask_env);
        xtask_binary_sbi(&xtask_env);
    } else if let Some(_matches) = matches.subcommand_matches("detect") {
        let ans = detect::detect_serial_ports();
        if let Some((port_name, info)) = ans {
            detect::dump_port(&port_name, &info);
            detect::save_to_file(&port_name);
        } else {
            println!("xtask: no CH340 serial port found.");
        }
    } else {
        println!("Use `cargo k210` to run, `cargo xtask --help` for help")
    }
}

fn detect_save_port_or_exit() -> String {
    if let Some((port_name, info)) = detect::detect_serial_ports() {
        println!("xtask: port detected");
        detect::dump_port(&port_name, &info);
        detect::save_to_file(&port_name);
        port_name
    } else {
        println!("xtask: no serial port found; program exit");
        std::process::exit(1);
    }
}

// @python ./ktool.py --port {{k210-serialport}} -b 1500000 --terminal {{fused-bin}}
fn xtask_run_k210(xtask_env: &XtaskEnv, port: &str) {
    let status = Command::new("python")
        .current_dir(project_root().join("xtask"))
        .arg("ktool.py")
        .args(&["--port", port])
        .args(&["--baudrate", "1500000"]) // todo: configurate baudrate
        .arg("--terminal")
        .arg(dist_dir(xtask_env).join("k210-fused.bin"))
        .status()
        .unwrap();
    if !status.success() {
        eprintln!(
            "xtask: run ktool.py failed with code {}",
            status.code().unwrap()
        );
        process::exit(status.code().unwrap())
    }
}

fn xtask_build_sbi(xtask_env: &XtaskEnv) {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut command = Command::new(cargo);
    command.current_dir(project_root().join("rustsbi-k210"));
    command.arg("build");
    match xtask_env.compile_mode {
        CompileMode::Debug => {}
        CompileMode::Release => {
            command.arg("--release");
        }
    }
    command.args(&["--package", "rustsbi-k210"]);
    command.args(&["--target", DEFAULT_TARGET]);
    let status = command.status().unwrap();
    if !status.success() {
        println!("cargo build failed");
        process::exit(1);
    }
}

fn xtask_binary_sbi(xtask_env: &XtaskEnv) {
    let objcopy = "rust-objcopy";
    let status = Command::new(objcopy)
        .current_dir(dist_dir(xtask_env))
        .arg("rustsbi-k210")
        .arg("--binary-architecture=riscv64")
        .arg("--strip-all")
        .args(&["-O", "binary", "rustsbi-k210.bin"])
        .status()
        .unwrap();

    if !status.success() {
        println!("objcopy binary failed");
        process::exit(1);
    }
}

fn xtask_build_test_kernel(xtask_env: &XtaskEnv) {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut command = Command::new(cargo);
    command.current_dir(project_root().join("test-kernel"));
    command.arg("build");
    match xtask_env.compile_mode {
        CompileMode::Debug => {}
        CompileMode::Release => {
            command.arg("--release");
        }
    }
    command.args(&["--package", "test-kernel"]);
    command.args(&["--target", DEFAULT_TARGET]);
    let status = command.status().unwrap();
    if !status.success() {
        println!("cargo build failed");
        process::exit(1);
    }
}

fn xtask_binary_test_kernel(xtask_env: &XtaskEnv) {
    let objcopy = "rust-objcopy";
    let status = Command::new(objcopy)
        .current_dir(dist_dir(xtask_env))
        .arg("test-kernel")
        .arg("--binary-architecture=riscv64")
        .arg("--strip-all")
        .args(&["-O", "binary", "test-kernel.bin"])
        .status()
        .unwrap();

    if !status.success() {
        println!("objcopy binary failed");
        process::exit(1);
    }
}

fn xtask_fuse_binary(xtask_env: &XtaskEnv) {
    let sbi_binary_path = dist_dir(xtask_env).join("rustsbi-k210.bin");
    let test_kernel_binary_path = dist_dir(xtask_env).join("test-kernel.bin");
    let output_path = dist_dir(xtask_env).join("k210-fused.bin");
    let offset = 0x20000;
    fs::copy(sbi_binary_path, &output_path).expect("copy sbi base");
    let mut output = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(output_path)
        .expect("open output file");
    let buf = fs::read(test_kernel_binary_path).expect("read kernel binary");
    output
        .seek(SeekFrom::Start(offset))
        .expect("seek to offset");
    output.write(&buf).expect("write output");
}

fn dist_dir(xtask_env: &XtaskEnv) -> PathBuf {
    let mut path_buf = project_root().join("target").join(DEFAULT_TARGET);
    path_buf = match xtask_env.compile_mode {
        CompileMode::Debug => path_buf.join("debug"),
        CompileMode::Release => path_buf.join("release"),
    };
    path_buf
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
