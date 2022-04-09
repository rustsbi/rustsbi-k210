use crate::runtime::SupervisorContext;
use core::arch::asm;
use riscv::register::{mie, mip, mstatus};

static mut DEVINTRENTRY: usize = 0;

pub unsafe fn call_supervisor_interrupt(ctx: &mut SupervisorContext) {
    let mut mstatus: usize;
    asm!("csrr {}, mstatus", out(reg) mstatus);
    // set mstatus.mprv
    mstatus |= 1 << 17;
    // it may trap from U/S Mode
    // save mpp and set mstatus.mpp to S Mode
    let mpp = (mstatus >> 11) & 3;
    mstatus = mstatus & !(3 << 11);
    mstatus |= 1 << 11;
    // drop mstatus.mprv protection
    asm!("csrw mstatus, {}", in(reg) mstatus);
    // compiler helps us save/restore caller-saved registers
    devintr();
    // restore mstatus
    mstatus = mstatus & !(3 << 11);
    mstatus |= mpp << 11;
    mstatus -= 1 << 17;
    asm!("csrw mstatus, {}", in(reg) mstatus);
    ctx.mstatus = mstatus::read();
}

// We use implementation specific sbi_rustsbi_k210_sext function (extension
// id: 0x0A000004, function id: 0x210) to register S-level interrupt handler
// for K210 chip only. This chip uses 1.9.1 version of privileged spec,
// which did not declare any S-level external interrupts.
#[inline]
pub fn emulate_sbi_rustsbi_k210_sext(ctx: &mut SupervisorContext) -> bool {
    if ctx.a7 == 0x0A000004 && ctx.a6 == 0x210 {
        unsafe {
            DEVINTRENTRY = ctx.a0;
        }
        // enable mext
        unsafe {
            mie::set_mext();
        }
        // return values
        ctx.a0 = 0; // SbiRet::error = SBI_SUCCESS
        ctx.a1 = 0; // SbiRet::value = 0
        ctx.mepc = ctx.mepc.wrapping_add(4); // PC += 4
        return true;
    } else {
        return false;
    }
}

fn devintr() {
    #[cfg(target_arch = "riscv")]
    unsafe {
        // call devintr defined in application
        // we have to ask compiler save ra explicitly
        asm!("jalr 0({})", in(reg) DEVINTRENTRY, lateout("ra") _);
    }
}

// Due to legacy 1.9.1 version of privileged spec, if we are in S-level
// timer handler (delegated from M mode), and we call SBI's `set_timer`,
// a M-level external interrupt may be triggered. This may try to obtain
// data structures locked previously by S-level interrupt handler, which
// results in a deadlock.
// Ref: https://github.com/luojia65/rustsbi/pull/5
pub fn preprocess_supervisor_external(ctx: &mut SupervisorContext) {
    if ctx.a7 == 0x0 {
        unsafe {
            let mtip = mip::read().mtimer();
            if mtip {
                if DEVINTRENTRY != 0 {
                    mie::set_mext();
                }
            }
        }
    }
}

pub fn forward_supervisor_timer() {
    // Forward to S-level timer interrupt
    unsafe {
        mip::set_stimer(); // set S-timer interrupt flag
        mie::clear_mext(); // Ref: rustsbi Pull request #5
        mie::clear_mtimer(); // mask M-timer interrupt
    }
}

pub fn forward_supervisor_soft() {
    // Forward to S-level software interrupt
    unsafe {
        mip::set_ssoft(); // set S-soft interrupt flag
        mie::clear_msoft(); // mask M-soft interrupt
    }
}
