#![no_std]
#![no_main]
#![feature(naked_functions)]
#![feature(generator_trait)]
#![feature(default_alloc_error_handler)]
#![feature(asm_sym, asm_const)]

mod execute;
mod feature;
mod hart_csr_utils;
mod peripheral;
mod runtime;

extern crate alloc;

use buddy_system_allocator::LockedHeap;
use core::arch::asm;
use core::panic::PanicInfo;

use rustsbi::println;

const PER_HART_STACK_SIZE: usize = 8 * 1024; // 8KiB
const SBI_STACK_SIZE: usize = 2 * PER_HART_STACK_SIZE;
#[link_section = ".bss.uninit"]
static mut SBI_STACK: [u8; SBI_STACK_SIZE] = [0; SBI_STACK_SIZE];

const SBI_HEAP_SIZE: usize = 8 * 1024; // 8KiB
#[link_section = ".bss.uninit"]
static mut HEAP_SPACE: [u8; SBI_HEAP_SIZE] = [0; SBI_HEAP_SIZE];
#[global_allocator]
static SBI_HEAP: LockedHeap<32> = LockedHeap::empty();

static DEVICE_TREE_BINARY: &[u8] = include_bytes!("../kendryte-k210.dtb");

#[cfg_attr(not(test), panic_handler)]
#[allow(unused)]
fn panic(info: &PanicInfo) -> ! {
    let hart_id = riscv::register::mhartid::read();
    // 输出的信息大概是“[rustsbi-panic] hart 0 panicked at ...”
    println!("[rustsbi-panic] hart {} {}", hart_id, info);
    println!("[rustsbi-panic] system shutdown scheduled due to RustSBI panic");
    use rustsbi::Reset;
    peripheral::Reset.system_reset(
        rustsbi::reset::RESET_TYPE_SHUTDOWN,
        rustsbi::reset::RESET_REASON_SYSTEM_FAILURE,
    );
    loop {}
}

extern "C" fn rust_main() -> ! {
    let hartid = riscv::register::mhartid::read();
    if hartid == 0 {
        init_bss();
    }
    pause_if_not_start_hart();
    runtime::init();
    if hartid == 0 {
        init_heap();
        peripheral::init_peripheral();
        println!("[rustsbi] RustSBI version {}", rustsbi::VERSION);
        println!("{}", rustsbi::LOGO);
        println!(
            "[rustsbi] Implementation: RustSBI-K210 Version {}",
            env!("CARGO_PKG_VERSION")
        );
    }
    delegate_interrupt_exception();
    if hartid == 0 {
        hart_csr_utils::print_hart_csrs();
        println!("[rustsbi] enter supervisor 0x80020000");
    }
    execute::execute_supervisor(0x80020000, hartid, DEVICE_TREE_BINARY.as_ptr() as usize)
}

fn pause_if_not_start_hart() {
    use k210_hal::clint::msip;
    use riscv::asm::wfi;
    use riscv::register::{mhartid, mie, mip};

    let hartid = mhartid::read();
    if hartid != 0 {
        unsafe {
            // Clear IPI
            msip::clear_ipi(hartid);
            // Start listening for software interrupts
            mie::set_msoft();

            loop {
                wfi();
                if mip::read().msoft() {
                    break;
                }
            }

            // Stop listening for software interrupts
            mie::clear_msoft();
            // Clear IPI
            msip::clear_ipi(hartid);
        }
    }
}

fn init_bss() {
    extern "C" {
        static mut ebss: u32;
        static mut sbss: u32;
        static mut edata: u32;
        static mut sdata: u32;
        static sidata: u32;
    }
    unsafe {
        r0::zero_bss(&mut sbss, &mut ebss);
        r0::init_data(&mut sdata, &mut edata, &sidata);
    }
}

fn init_heap() {
    unsafe {
        SBI_HEAP
            .lock()
            .init(HEAP_SPACE.as_ptr() as usize, SBI_HEAP_SIZE)
    }
}

// 委托终端；把S的中断全部委托给S层
fn delegate_interrupt_exception() {
    use riscv::register::{medeleg, mideleg, mie};
    unsafe {
        //mideleg::set_sext();
        mideleg::set_stimer();
        mideleg::set_ssoft();
        medeleg::set_instruction_misaligned();
        medeleg::set_breakpoint();
        medeleg::set_user_env_call();
        /* MMU Exception Delegation
        /* Page Faults are *Reserved* in 1.9.1 version */
        - medeleg::set_instruction_page_fault();
        - medeleg::set_load_page_fault();
        - medeleg::set_store_page_fault();
        /* Actually, in 1.9.1 they are merged into more general exceptions */
        + medeleg::set_instruction_fault();
        + medeleg::set_load_fault();
        + medeleg::set_store_fault(); */
        // medeleg::set_instruction_fault();
        // medeleg::set_load_fault();
        // medeleg::set_store_fault();
        // 默认不打开mie::set_mext
        // 不打开mie::set_mtimer
        mie::set_msoft();
    }
}

#[naked]
#[link_section = ".text.entry"]
#[export_name = "_start"]
unsafe extern "C" fn entry() -> ! {
    asm!(
    // 1. set sp
    // sp = bootstack + (hartid + 1) * HART_STACK_SIZE
    "
    la      sp, {stack}
    li      t0, {per_hart_stack_size}
    csrr    a0, mhartid
    addi    t1, a0, 1
1:  add     sp, sp, t0
    addi    t1, t1, -1
    bnez    t1, 1b
    ",
    // 2. jump to rust_main (absolute address)
    "j      {rust_main}", 
    per_hart_stack_size = const PER_HART_STACK_SIZE,
    stack = sym SBI_STACK,
    rust_main = sym rust_main,
    options(noreturn))
}
