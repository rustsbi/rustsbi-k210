use crate::{println, sbi};
use core::arch::asm;
use riscv::register::{
    scause::{self, Exception, Trap},
    sepc,
    stvec::{self, TrapMode},
};

pub fn test_delegate_trap() {
    println!(">> Test-kernel: Trigger illegal exception");
    let stvec_before = stvec::read().address();
    init_trap_vector();
    unsafe { asm!("csrw mcycle, x0") }; // mcycle cannot be written, this is always a 4-byte illegal instruction
    unsafe { stvec::write(stvec_before, TrapMode::Direct) };
}

fn init_trap_vector() {
    let mut addr = delegate_test_trap as usize;
    if addr & 0x2 != 0 {
        addr = addr.wrapping_add(0x2); // 必须对齐到4个字节
    }
    unsafe { stvec::write(addr, TrapMode::Direct) };
}

extern "C" fn rust_test_trap_handler() {
    let cause = scause::read().cause();
    println!("<< Test-kernel: Value of scause: {:?}", cause);
    if cause != Trap::Exception(Exception::IllegalInstruction) {
        println!("!! Test-kernel: Wrong cause associated to illegal instruction");
        sbi::shutdown()
    }
    println!("<< Test-kernel: Illegal exception delegate success");
    sepc::write(sepc::read().wrapping_add(4)); // skip mcycle write illegal instruction
}

#[naked]
#[link_section = ".text"]
unsafe extern "C" fn delegate_test_trap() -> ! {
    asm!(
        ".align 4", // align to 4 bytes
        "addi   sp, sp, -8*16
        sd      ra, 8*0(sp)
        sd      t0, 8*1(sp)
        sd      t1, 8*2(sp)
        sd      t2, 8*3(sp)
        sd      t3, 8*4(sp)
        sd      t4, 8*5(sp)
        sd      t5, 8*6(sp)
        sd      t6, 8*7(sp)
        sd      a0, 8*8(sp)
        sd      a1, 8*9(sp)
        sd      a2, 8*10(sp)
        sd      a3, 8*11(sp)
        sd      a4, 8*12(sp)
        sd      a5, 8*13(sp)
        sd      a6, 8*14(sp)
        sd      a7, 8*15(sp)",
        "call   {rust_test_trap_handler}",
        "ld     ra, 8*0(sp)
        ld      t0, 8*1(sp)
        ld      t1, 8*2(sp)
        ld      t2, 8*3(sp)
        ld      t3, 8*4(sp)
        ld      t4, 8*5(sp)
        ld      t5, 8*6(sp)
        ld      t6, 8*7(sp)
        ld      a0, 8*8(sp)
        ld      a1, 8*9(sp)
        ld      a2, 8*10(sp)
        ld      a3, 8*11(sp)
        ld      a4, 8*12(sp)
        ld      a5, 8*13(sp)
        ld      a6, 8*14(sp)
        ld      a7, 8*15(sp)
        addi    sp, sp, 8*16",
        "sret",
        rust_test_trap_handler = sym rust_test_trap_handler,
        options(noreturn)
    )
}
