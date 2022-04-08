mod base_extension;
mod catch_page_fault;
mod delegate_trap;
mod sfence_vma;

pub use base_extension::test_base_extension;
pub use catch_page_fault::test_catch_page_fault;
pub use delegate_trap::test_delegate_trap;
pub use sfence_vma::test_sfence_vma;
