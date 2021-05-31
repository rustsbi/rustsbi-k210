#![feature(naked_functions, asm)]
#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[cfg_attr(not(test), panic_handler)]
#[allow(unused)]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[naked]
#[link_section = ".text.entry"] 
#[export_name = "_start"]
unsafe extern "C" fn entry() -> ! {
    asm!("1: j 1b", options(noreturn))
}
