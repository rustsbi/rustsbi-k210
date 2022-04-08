use k210_hal::{clint::msip, clock::Clocks, fpioa, pac, prelude::*};
use riscv::register::{mhartid, mip};
use rustsbi::println;

pub fn init_peripheral() {
    let p = pac::Peripherals::take().unwrap();

    let mut sysctl = p.SYSCTL.constrain();
    let fpioa = p.FPIOA.split(&mut sysctl.apb0);
    let clocks = Clocks::new();
    let _uarths_tx = fpioa.io5.into_function(fpioa::UARTHS_TX);
    let _uarths_rx = fpioa.io4.into_function(fpioa::UARTHS_RX);
    // Configure UART
    let serial = p.UARTHS.configure(115_200.bps(), &clocks);
    let (tx, rx) = serial.split();

    rustsbi::legacy_stdio::init_legacy_stdio_embedded_hal_fuse(tx, rx);
    rustsbi::init_timer(Timer);
    rustsbi::init_reset(Reset);
    rustsbi::init_ipi(Ipi);
}

struct Ipi;

impl rustsbi::Ipi for Ipi {
    fn max_hart_id(&self) -> usize {
        1
    }
    fn send_ipi_many(&self, hart_mask: rustsbi::HartMask) -> rustsbi::SbiRet {
        for i in 0..=1 {
            if hart_mask.has_bit(i) {
                msip::set_ipi(i);
                msip::clear_ipi(i);
            }
        }
        rustsbi::SbiRet::ok(0)
    }
}

struct Timer;

impl rustsbi::Timer for Timer {
    fn set_timer(&self, stime_value: u64) {
        // This function must clear the pending timer interrupt bit as well.
        use k210_hal::clint::mtimecmp;
        mtimecmp::write(mhartid::read(), stime_value);
        unsafe { mip::clear_mtimer() };
    }
}

pub struct Reset;

impl rustsbi::Reset for Reset {
    fn system_reset(&self, reset_type: usize, reset_reason: usize) -> rustsbi::SbiRet {
        println!("[rustsbi] reset triggered! todo: shutdown all harts on k210; program halt. Type: {}, reason: {}", reset_type, reset_reason);
        loop {}
    }
}
