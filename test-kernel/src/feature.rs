mod base_extension;
mod delegate_trap;
mod sfence_vma;
mod catch_page_fault;

pub use base_extension::test_base_extension;
pub use delegate_trap::test_delegate_trap;
pub use sfence_vma::test_sfence_vma;
pub use catch_page_fault::test_catch_page_fault;
