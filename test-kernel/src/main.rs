#![feature(naked_functions)]
#![feature(asm_sym, asm_const)]
#![feature(generator_trait)]
#![feature(default_alloc_error_handler)]
#![feature(stdsimd)]
#![no_std]
#![no_main]

use core::arch::asm;

mod console;
mod feature;
mod sbi;

const PER_HART_STACK_SIZE: usize = 64 * 1024; // 64KiB
const KERNEL_STACK_SIZE: usize = 2 * PER_HART_STACK_SIZE;
#[link_section = ".bss.uninit"]
static mut KERNEL_STACK: [u8; KERNEL_STACK_SIZE] = [0; KERNEL_STACK_SIZE];

const KERNEL_HEAP_SIZE: usize = 128 * 1024; // 128KiB
#[link_section = ".bss.uninit"]
static mut HEAP_SPACE: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];
#[global_allocator]
static KERNEL_HEAP: LockedHeap<32> = LockedHeap::empty();

use buddy_system_allocator::LockedHeap;

extern "C" fn rust_main(hartid: usize, opaque: usize) -> ! {
    if hartid == 0 {
        init_bss();
        init_heap();
    }
    println!(
        "<< Test-kernel: Hart id = {}, opaque = {:#x}",
        hartid, opaque
    );
    feature::test_base_extension();
    feature::test_delegate_trap();
    feature::test_sfence_vma();
    test_emulate_rdtime();
    feature::test_catch_page_fault();
    println!("<< Test-kernel: SBI test SUCCESS, shutdown");
    sbi::shutdown()
}

pub fn test_emulate_rdtime() {
    println!(">> Test-kernel: Testing SBI instruction emulation");
    let time = riscv::register::time::read64();
    println!("<< Test-kernel: Current time: {:x}", time);
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
        KERNEL_HEAP
            .lock()
            .init(HEAP_SPACE.as_ptr() as usize, KERNEL_HEAP_SIZE)
    }
}

use core::panic::PanicInfo;

#[cfg_attr(not(test), panic_handler)]
#[allow(unused)]
fn panic(info: &PanicInfo) -> ! {
    println!("!! Test-kernel: {}", info);
    println!("!! Test-kernel: SBI test FAILED due to panic");
    sbi::shutdown()
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
    addi    t1, a0, 1
1:  add     sp, sp, t0
    addi    t1, t1, -1
    bnez    t1, 1b
    ",
    // 2. jump to rust_main (absolute address)
    "j      {rust_main}", 
    per_hart_stack_size = const PER_HART_STACK_SIZE,
    stack = sym KERNEL_STACK,
    rust_main = sym rust_main,
    options(noreturn))
}
