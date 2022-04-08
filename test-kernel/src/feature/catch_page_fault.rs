use crate::{println, sbi};
use core::arch::{asm, riscv64::sfence_vma_all};
use riscv::register::{
    satp::{self, Mode},
    scause::{self, Exception, Trap},
    sepc,
    stvec::{self, TrapMode},
};

#[repr(align(4096))]
struct PageTable {
    #[allow(unused)] // Will be used by RISC-V hardware
    entries: [usize; 512],
}

static mut TEST_PAGE_TABLE_0: PageTable = PageTable { entries: [0; 512] };
static mut TEST_PAGE_TABLE_1: PageTable = PageTable { entries: [0; 512] };
static mut TEST_PAGE_TABLE_2: PageTable = PageTable { entries: [0; 512] };

pub fn test_catch_page_fault() {
    println!(">> Test-kernel: Testing catch page fault");
    init_trap_vector();
    let ppn = init_page_table();
    unsafe { satp::set(Mode::Sv39, 0, ppn) };
    unsafe { sfence_vma_all() };
    unsafe {
        println!(">> Test-kernel: Wrong sign extension");
        assert!(is_read_page_fault(0xfeff_ff80_0000_0000 as *const usize));
        assert!(is_read_page_fault(0x0100_0000_0000_0000 as *const usize));
    }
    unsafe {
        println!(">> Test-kernel: Read from invalid entry");
        assert!(is_read_page_fault(0x1_0000_0000 as *const usize));
        assert!(is_read_page_fault(0x0_c040_0000 as *const usize));
        assert!(is_read_page_fault(0x0_c020_2000 as *const usize));
    };
    // unsafe {
    println!(">> Test-kernel: Unaligned huge page");
    //     assert!(is_read_page_fault(0x1_4000_0000 as *const usize));
    //     assert!(is_read_page_fault(0x0_c060_0000 as *const usize));
    // };
    // unsafe {
    println!(">> Test-kernel: Non existing page");
    //     assert!(is_read_page_fault(0x1_8000_0000 as *const usize));
    //     assert!(is_read_page_fault(0x0_c080_0000 as *const usize));
    //     assert!(is_read_page_fault(0x0_c020_3000 as *const usize));
    // };
    unsafe {
        println!(">> Test-kernel: Level zero page cannot have leaves");
        assert!(is_read_page_fault(0x0_c020_1000 as *const usize));
    };
}

fn init_page_table() -> usize {
    let ppn1 = (unsafe { &TEST_PAGE_TABLE_1 } as *const _ as usize) >> 12;
    let ppn2 = (unsafe { &TEST_PAGE_TABLE_2 } as *const _ as usize) >> 12;
    unsafe {
        TEST_PAGE_TABLE_0.entries[2] = (0x80000 << 10) | 0xf; // RWX, V
        TEST_PAGE_TABLE_0.entries[3] = (ppn1 << 10) | 0x1; // 叶子, V
        TEST_PAGE_TABLE_0.entries[4] = 0; // 无效
        TEST_PAGE_TABLE_0.entries[5] = (0x80200 << 10) | 0xf; // RWX, V
        TEST_PAGE_TABLE_0.entries[6] = (0x7ffff << 10) | 0xf; // RWX, V
        TEST_PAGE_TABLE_0.entries[7] = (0x80000 << 10) | 0x7; // RW, V
    }
    unsafe {
        TEST_PAGE_TABLE_1.entries[1] = (ppn2 << 10) | 0x1; // 叶子, V
        TEST_PAGE_TABLE_1.entries[2] = 0; // 无效
        TEST_PAGE_TABLE_1.entries[3] = (0x80201 << 10) | 0xf; // RWX, V
        TEST_PAGE_TABLE_1.entries[4] = (0x7ffff << 10) | 0xf; // RWX, V
        TEST_PAGE_TABLE_1.entries[5] = (0x80200 << 10) | 0x3; // R, V
    }
    unsafe {
        TEST_PAGE_TABLE_2.entries[1] = (0x80200 << 10) | 0x1; // 叶子, V
        TEST_PAGE_TABLE_2.entries[2] = 0; // 无效
        TEST_PAGE_TABLE_2.entries[3] = (0x7ffff << 10) | 0xf; // RWX, V
        TEST_PAGE_TABLE_2.entries[4] = (0x80200 << 10) | 0x9; // X, V
    }
    let pa = unsafe { &TEST_PAGE_TABLE_0 } as *const _ as usize;
    let ppn = pa >> 12;
    ppn
}

#[derive(Debug)]
struct NotPageFault;

unsafe fn is_read_page_fault(ptr: *const usize) -> bool {
    let saved_stvec_address = init_trap_vector();
    let _ans: usize;
    asm!("li    tp, 0");
    asm!("ld    {}, 0({})", out(reg) _ans, in(reg) ptr);
    let is_page_fault: usize;
    asm!("mv    {}, tp", out(reg) is_page_fault);
    let ans = is_page_fault == 1;
    recover_trap_vector(saved_stvec_address);
    return ans;
}

fn init_trap_vector() -> usize {
    let mut addr = delegate_test_trap as usize;
    if addr & 0x2 != 0 {
        addr = addr.wrapping_add(0x2); // 必须对齐到4个字节
    }
    let saved_stvec_address = stvec::read().address();
    unsafe { stvec::write(addr, TrapMode::Direct) };
    saved_stvec_address
}

fn recover_trap_vector(saved_stvec_address: usize) {
    unsafe { stvec::write(saved_stvec_address, TrapMode::Direct) }
}

extern "C" fn rust_test_trap_handler() {
    let cause = scause::read().cause();
    if cause != Trap::Exception(Exception::LoadPageFault) {
        println!(
            "!! Test-kernel: Wrong cause associated to page fault, sepc: {:#x}, stval: {:#x}",
            riscv::register::sepc::read(),
            riscv::register::stval::read()
        );
        sbi::shutdown()
    }
    unsafe { asm!("li   tp, 1") }; // tp = 1 说明是缺页异常
    let bad_ins_addr = sepc::read();
    let ins_16 = unsafe { core::ptr::read_volatile(bad_ins_addr as *const u16) };
    let bytes = if ins_16 & 0b11 != 0b11 { 2 } else { 4 };
    sepc::write(sepc::read().wrapping_add(bytes)); // skip current instruction
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
