use crate::{println, sbi};

pub fn test_base_extension() {
    println!(">> Test-kernel: Testing base extension");
    let base_version = sbi::probe_extension(sbi::EXTENSION_BASE);
    if base_version == 0 {
        println!("!! Test-kernel: no base extension probed; SBI call returned value '0'");
        println!(
            "!! Test-kernel: This SBI implementation may only have legacy extension implemented"
        );
        println!("!! Test-kernel: SBI test FAILED due to no base extension found");
        sbi::shutdown()
    }
    println!("<< Test-kernel: Base extension version: {:x}", base_version);
    println!(
        "<< Test-kernel: SBI specification version: {:x}",
        sbi::get_spec_version()
    );
    println!(
        "<< Test-kernel: SBI implementation Id: {:x}",
        sbi::get_sbi_impl_id()
    );
    println!(
        "<< Test-kernel: SBI implementation version: {:x}",
        sbi::get_sbi_impl_version()
    );
    println!(
        "<< Test-kernel: Device mvendorid: {:x}",
        sbi::get_mvendorid()
    );
    println!("<< Test-kernel: Device marchid: {:x}", sbi::get_marchid());
    println!("<< Test-kernel: Device mimpid: {:x}", sbi::get_mimpid());
}
