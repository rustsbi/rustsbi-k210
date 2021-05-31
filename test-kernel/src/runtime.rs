use riscv::register::{
    sstatus::{self, Sstatus, SPP},
    scause::{self, Trap, Exception},
    stvec::{self, TrapMode}, stval,
};
use core::{
    pin::Pin,
    ops::{Generator, GeneratorState},
};

pub fn init() {
    let mut addr = crate::executor::from_user_save as usize;
    if addr & 0x2 != 0 {
        addr += 0x2; // 必须对齐到4个字节
    }
    unsafe { stvec::write(addr, TrapMode::Direct) };
}

#[repr(C)]
pub struct Runtime {
    context: UserContext, 
    user_stack: usize, // 这里应该给所有权。暂时只给一页的栈（太抠了)
}

impl Runtime {
    pub fn new_user(first_app_sepc: usize, user_stack: usize) -> Self {
        let context: UserContext = unsafe { core::mem::MaybeUninit::zeroed().assume_init() };
        let mut ans = Runtime { context, user_stack };
        ans.prepare_next_app(first_app_sepc);
        ans
    }

    fn reset(&mut self) {
        self.context.sp = self.user_stack;
        unsafe { sstatus::set_spp(SPP::User) };
        self.context.sstatus = sstatus::read();
        self.context.kernel_stack = 0x233333666666; // 将会被resume函数覆盖
    }

    // 在处理异常的时候，使用context_mut得到运行时当前用户的上下文，可以改变上下文的内容
    pub fn context_mut(&mut self) -> &mut UserContext {
        &mut self.context
    }

    pub fn prepare_next_app(&mut self, new_sepc: usize) {
        self.reset();
        self.context.sepc = new_sepc;
    }
}

impl Generator for Runtime {
    type Yield = KernelTrap;
    type Return = ();
    fn resume(mut self: Pin<&mut Self>, _arg: ()) -> GeneratorState<Self::Yield, Self::Return> {
        unsafe { do_resume(&mut self.context as *mut _) };
        let stval = stval::read();
        let trap = match scause::read().cause() {
            Trap::Exception(Exception::UserEnvCall) => KernelTrap::Syscall(),
            Trap::Exception(Exception::LoadFault) => KernelTrap::LoadAccessFault(stval),
            Trap::Exception(Exception::StoreFault) => KernelTrap::StoreAccessFault(stval),
            Trap::Exception(Exception::IllegalInstruction) => KernelTrap::IllegalInstruction(stval),
            e => panic!("unhandled exception: {:?}! stval: {:#x?}, ctx: {:#x?}", e, stval, self.context)
        };
        GeneratorState::Yielded(trap)
    }
}

#[repr(C)]
pub enum KernelTrap {
    Syscall(),
    LoadAccessFault(usize),
    StoreAccessFault(usize),
    IllegalInstruction(usize),
}

#[derive(Debug)]
#[repr(C)]
pub struct UserContext {
    pub ra: usize, // 0
    pub sp: usize,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub s0: usize,
    pub s1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize, // 30
    pub sstatus: Sstatus, // 31
    pub sepc: usize, // 32
    pub kernel_stack: usize, // 33
}

#[naked]
#[link_section = ".text"]
unsafe extern "C" fn do_resume(_user_context: *mut UserContext) {
    asm!("j     {from_kernel_save}", from_kernel_save = sym from_kernel_save, options(noreturn))
}

#[naked]
#[link_section = ".text"]
unsafe extern "C" fn from_kernel_save(_user_context: *mut UserContext) -> ! {
    asm!( // sp:内核栈顶
        "addi   sp, sp, -15*8", // sp:内核栈顶
        // 进入函数之前，已经保存了调用者寄存器，应当保存被调用者寄存器
        "sd     ra, 0*8(sp)
        sd      gp, 1*8(sp)
        sd      tp, 2*8(sp)
        sd      s0, 3*8(sp)
        sd      s1, 4*8(sp)
        sd      s2, 5*8(sp)
        sd      s3, 6*8(sp)
        sd      s4, 7*8(sp)
        sd      s5, 8*8(sp)
        sd      s6, 9*8(sp)
        sd      s7, 10*8(sp)
        sd      s8, 11*8(sp)
        sd      s9, 12*8(sp)
        sd      s10, 13*8(sp)
        sd      s11, 14*8(sp)", 
        // a0:用户上下文
        "j      {to_user_restore}",
        to_user_restore = sym to_user_restore,
        options(noreturn)
    )
}

#[naked]
#[link_section = ".text"]
pub unsafe extern "C" fn to_user_restore(_user_context: *mut UserContext) -> ! {
    asm!( // a0:用户上下文
        "sd     sp, 33*8(a0)", // 内核栈顶放进用户上下文
        "csrw   sscratch, a0", // 新sscratch:用户上下文
        // sscratch:用户上下文
        "mv     sp, a0", // 新sp:用户上下文
        "ld     t0, 31*8(sp)
        ld      t1, 32*8(sp)
        csrw    sstatus, t0
        csrw    sepc, t1",
        "ld     ra, 0*8(sp)
        ld      gp, 2*8(sp)
        ld      tp, 3*8(sp)
        ld      t0, 4*8(sp)
        ld      t1, 5*8(sp)
        ld      t2, 6*8(sp)
        ld      s0, 7*8(sp)
        ld      s1, 8*8(sp)
        ld      a0, 9*8(sp)
        ld      a1, 10*8(sp)
        ld      a2, 11*8(sp)
        ld      a3, 12*8(sp)
        ld      a4, 13*8(sp)
        ld      a5, 14*8(sp)
        ld      a6, 15*8(sp)
        ld      a7, 16*8(sp)
        ld      s2, 17*8(sp)
        ld      s3, 18*8(sp)
        ld      s4, 19*8(sp)
        ld      s5, 20*8(sp)
        ld      s6, 21*8(sp)
        ld      s7, 22*8(sp)
        ld      s8, 23*8(sp)
        ld      s9, 24*8(sp)
        ld     s10, 25*8(sp)
        ld     s11, 26*8(sp)
        ld      t3, 27*8(sp)
        ld      t4, 28*8(sp)
        ld      t5, 29*8(sp)
        ld      t6, 30*8(sp)",
        "ld     sp, 1*8(sp)", // 新sp:用户栈
        // sp:用户栈, sscratch:用户上下文
        "sret",
        options(noreturn)
    )
}

// 中断开始

#[naked]
#[link_section = ".text"]
pub unsafe extern "C" fn from_user_save() -> ! {
    asm!( // sp:用户栈,sscratch:用户上下文
        ".p2align 2",
        "csrrw  sp, sscratch, sp", // 新sscratch:用户栈, 新sp:用户上下文
        "sd     ra, 0*8(sp)
        sd      gp, 2*8(sp)
        sd      tp, 3*8(sp)
        sd      t0, 4*8(sp)
        sd      t1, 5*8(sp)
        sd      t2, 6*8(sp)
        sd      s0, 7*8(sp)
        sd      s1, 8*8(sp)
        sd      a0, 9*8(sp)
        sd      a1, 10*8(sp)
        sd      a2, 11*8(sp)
        sd      a3, 12*8(sp)
        sd      a4, 13*8(sp)
        sd      a5, 14*8(sp)
        sd      a6, 15*8(sp)
        sd      a7, 16*8(sp)
        sd      s2, 17*8(sp)
        sd      s3, 18*8(sp)
        sd      s4, 19*8(sp)
        sd      s5, 20*8(sp)
        sd      s6, 21*8(sp)
        sd      s7, 22*8(sp)
        sd      s8, 23*8(sp)
        sd      s9, 24*8(sp)
        sd     s10, 25*8(sp)
        sd     s11, 26*8(sp)
        sd      t3, 27*8(sp)
        sd      t4, 28*8(sp)
        sd      t5, 29*8(sp)
        sd      t6, 30*8(sp)",
        "csrr   t0, sstatus
        sd      t0, 31*8(sp)",
        "csrr   t1, sepc
        sd      t1, 32*8(sp)",
        // sscratch:用户栈,sp:用户上下文
        "csrrw  t2, sscratch, sp", // 新sscratch:用户上下文,t2:用户栈
        "sd     t2, 1*8(sp)", // 保存用户栈
        "j      {to_kernel_restore}",
        to_kernel_restore = sym to_kernel_restore,
        options(noreturn)
    )
}

#[naked]
#[link_section = ".text"]
unsafe extern "C" fn to_kernel_restore() -> ! {
    asm!( // sscratch:用户上下文
        "csrr   sp, sscratch", // sp:用户上下文
        "ld     sp, 33*8(sp)", // sp:内核栈
        "ld     ra, 0*8(sp)
        ld      gp, 1*8(sp)
        ld      tp, 2*8(sp)
        ld      s0, 3*8(sp)
        ld      s1, 4*8(sp)
        ld      s2, 5*8(sp)
        ld      s3, 6*8(sp)
        ld      s4, 7*8(sp)
        ld      s5, 8*8(sp)
        ld      s6, 9*8(sp)
        ld      s7, 10*8(sp)
        ld      s8, 11*8(sp)
        ld      s9, 12*8(sp)
        ld      s10, 13*8(sp)
        ld      s11, 14*8(sp)", 
        "addi   sp, sp, 15*8", // sp:内核栈顶
        "jr     ra", // 其实就是ret
        options(noreturn)
    )
}
