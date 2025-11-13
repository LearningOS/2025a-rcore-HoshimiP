//! Memory management implementation
//!
//! SV39 page-based virtual-memory architecture for RV64 systems, and
//! everything about memory management, like frame allocator, page table,
//! map area and memory set, is implemented here.
//!
//! Every task or process has a memory_set to control its virtual memory.

mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

use address::VPNRange;
pub use address::{PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
pub use frame_allocator::{frame_alloc, frame_dealloc, FrameTracker};
pub use memory_set::remap_test;
pub use memory_set::{kernel_token, MapPermission, MemorySet, KERNEL_SPACE};
use page_table::PTEFlags;
pub use page_table::{
    translated_byte_buffer, translated_ref, translated_refmut, translated_str, PageTable,
    PageTableEntry, UserBuffer, UserBufferIterator,
};
use crate::task::current_user_token;
/// initiate heap allocator, frame allocator and kernel space
pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.exclusive_access().activate();
}
/// 判断给定虚拟地址是否为用户态可写
pub fn is_user_writable(va: usize) -> bool {
    let page_table = PageTable::from_token(current_user_token());
    if let Some(pte) = page_table.translate(VirtAddr::from(va).floor()) {
        pte.flags().contains(PTEFlags::U | PTEFlags::W)
    } else {
        false
    }
}
/// 将用户虚拟地址指针转换为内核可访问的物理地址指针
pub fn translate_ptr<T>(ptr: *const T) -> *mut T {
    let page_table: PageTable = PageTable::from_token(current_user_token());
    let start: usize = ptr as usize;
    let start_va: VirtAddr = VirtAddr::from(start);
    let vpn: VirtPageNum = start_va.floor();
    let ppn: PhysAddr = page_table.translate(vpn).unwrap().ppn().into();
    let offset: usize = start_va.page_offset();
    let phys_addr: usize = ppn.into();
    let phys_ptr: *mut T = (phys_addr + offset) as *mut T;
    phys_ptr
}
