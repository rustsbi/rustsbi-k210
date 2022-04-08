use crate::{println, sbi};
use riscv::{
    asm,
    register::satp::{self, Mode},
};

#[repr(align(4096))]
struct PageTable {
    #[allow(unused)] // Will be used by RISC-V hardware
    entries: [usize; 512],
}

static TEST_PAGE_TABLE: PageTable = {
    let mut entries = [0; 512];
    entries[2] = (0x80000 << 10) | 0xcf; // 0x8000_0000 -> 0x8000_0000，0xcf 表示 VRWXAD 均为 1
    entries[508] = (0x00000 << 10) | 0xcf; // 0xffff_ffff_0000_0000 -> 0x0000_0000，0xcf 表示 VRWXAD 均为 1
    entries[510] = (0x80000 << 10) | 0xcf; // 0xffff_ffff_8000_0000 -> 0x8000_0000，0xcf 表示 VRWXAD 均为 1
    PageTable { entries }
};

static VARIABLE: usize = 0x6666233399998888;

pub fn test_sfence_vma() {
    println!(">> Test-kernel: Testing emulated virtual memory unit");
    let pa = &TEST_PAGE_TABLE as *const _ as usize;
    let ppn = pa >> 12;
    unsafe { satp::set(Mode::Sv39, 0, ppn) };
    unsafe { asm::sfence_vma_all() }; // SBI will emulate this instruction
    println!("<< Test-kernel: Code memory page test success");
    let ptr = &VARIABLE as *const _ as usize;
    let mapped_ptr = ptr + 0xffff_ffff_0000_0000;
    let mapped_variable = unsafe { *(mapped_ptr as *const usize) };
    if mapped_variable != VARIABLE {
        println!("!! Test-kernel: Multi mapping page test failed: variable value don't match");
        sbi::shutdown()
    }
    println!("<< Test-kernel: Multi mapping page test success");
}
