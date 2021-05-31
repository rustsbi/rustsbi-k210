use crate::println;

pub fn test_emulate_rdtime() {
    println!(">> Test-kernel: Testing SBI instruction emulation");
    let time = riscv::register::time::read64();
    println!("<< Test-kernel: Current time: {:x}", time);
}
