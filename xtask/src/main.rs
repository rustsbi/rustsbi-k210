use clap::{clap_app, crate_authors, crate_description, crate_version};

mod detect;
mod test;

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
    ).get_matches();
    // Read: python xtask/ktool.py -a 0x80000000 -R -L 0x20000 ./target/xtask/flash_dump.bin
    if let Some(_matches) = matches.subcommand_matches("k210") {
        let port = match detect::read_serial_port_choose_file() {
            Ok(string) => {
                println!("xtask: using previously selected serial port {}.", string);
                string
            },
            Err(_e) => detect_save_port_or_exit()
        };
        println!("Run k210 on {}", port);
    }else if let Some(_matches) = matches.subcommand_matches("detect") {
        let ans = detect::detect_serial_ports();
        if let Some((port_name, info)) = ans {
            detect::dump_port(&port_name, &info);
            detect::save_to_file(&port_name);
        } else { 
            println!("xtask: no CH340 serial port found.");
        }
    } else {
        println!("Use `cargo qemu` to run, `cargo xtask --help` for help")
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
