mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
pub use memory_set::{MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{
    translated_byte_buffer, translated_byte_buffer_copy, translated_byte_buffer_mut,
    translated_refmut, translated_str, PageTableEntry,
};

pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.lock().activate();
}
