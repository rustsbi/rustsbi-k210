use crate::{sbi, println};
use riscv::{asm, register::satp::{self, Mode}};

#[repr(align(4096))]
struct PageTable {
    #[allow(unused)] // Will be used by RISC-V hardware
    entries: [usize; 512],
}

static mut TEST_PAGE_TABLE: PageTable = {
    let mut entries = [0; 512];
    entries[2] = (0x80000 << 10) | 0xcf;   // 0x8000_0000 -> 0x8000_0000，0xcf 表示 VRWXAD 均为 1
    // entries[3] = ...... << 10 | ....;   // 0x1_0000_0000 -> 子页表，0x1 表示V为1，RWX为0
    PageTable { entries }
};

static mut TEST_PAGE_TABLE_2: PageTable = {
    let mut entries = [0; 512];
    entries[1] = (0x80200 << 10) | 0xcf;   // 0x1_0020_0000 -> 0x8020_0000，0xcf 表示 VRWXAD 均为 1
    // entries[2] = ...... << 10 | ....;   // 0x1_0040_0000 -> 子页表，0x1 表示V为1，RWX为0
    PageTable { entries }
};

static TEST_PAGE_TABLE_3: PageTable = {
    let mut entries = [0; 512];
    entries[1] = (0x80200 << 10) | 0xcf;   // 0x1_0040_1000 -> 0x8020_0000，0xcf 表示 VRWXAD 均为 1
    PageTable { entries }
};

pub fn test_catch_page_fault() {
    println!(">> Test-kernel: Testing catch page fault");
    let ppn2 = unsafe { &TEST_PAGE_TABLE_2 } as *const _ as usize;
    unsafe { 
        TEST_PAGE_TABLE.entries[3] = (ppn2 << 10) | 0x1;
    }
    let ppn3 = &TEST_PAGE_TABLE_3 as *const _ as usize;
    unsafe { 
        TEST_PAGE_TABLE_2.entries[2] = (ppn3 << 10) | 0x1;
    }
    let pa = unsafe { &TEST_PAGE_TABLE } as *const _ as usize;
    let ppn = pa >> 12;
    unsafe { satp::set(Mode::Sv39, 0, ppn) };
    unsafe { asm::sfence_vma_all() }; 
    // let first_asm = unsafe { *(0x8020_0000 as *const usize) };
    // let first_asm_shadow = unsafe { *(0x1_0020_0000 as *const usize) };
    // todo
}
