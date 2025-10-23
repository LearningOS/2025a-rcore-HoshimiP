//! Process management syscalls
use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, current_task_id, TASK_MANAGER};
use crate::timer::get_time_us;
use crate::mm::{is_user_writable, translate_ptr, is_user_readable};
use crate::mm::MapPermission;
use crate::mm::VirtAddr;
use crate::task::TaskControlBlock;
//use crate::mm::translated_byte_buffer;
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    if !is_user_writable(ts as usize) {
        return -1;
    }
    let pts = translate_ptr(ts);
    unsafe {
        *pts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        0 => {
            if !is_user_readable(id as usize) {
                return -1;
            }
            let pid = translate_ptr(id as *const u8);
            let ptr = pid as *const u8;
            unsafe { core::ptr::read_volatile(ptr) as isize }
        }
        1 => {
            if !is_user_writable(id as usize) {
                return -1;
            }
            let pid = translate_ptr(id as *const u8);
            let ptr = pid as *mut u8;
            unsafe {
                core::ptr::write_volatile(ptr, data as u8);
            }
            0
        }
        2 => {
            let task_id = current_task_id();
            let inner = TASK_MANAGER.inner.exclusive_access();
            let syscall_count = &mut inner.tasks[task_id].inner.exclusive_access().syscall_count;
            syscall_count[id] as isize
        }
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap");
    // 长度为0直接返回错误
    if len == 0 {
        return -1;
    }
    // 检查起始地址页对齐
    if start % crate::config::PAGE_SIZE != 0 {
        return -1;
    }
    // 检查prot合法性
    if prot & !0b111 != 0 {
        return -1;
    }
    if prot == 0 {
        return -1;
    }
    // 构造权限
    let mut perm = MapPermission::U;
    if prot & 0b001 != 0 { perm |= MapPermission::R; }
    if prot & 0b010 != 0 { perm |= MapPermission::W; }
    if prot & 0b100 != 0 { perm |= MapPermission::X; }

    let end = start + len;
    let task: &mut TaskControlBlock = crate::task::current_task().unwrap();
    let memory_set = &mut task.memory_set;
    let start_vpn = VirtAddr::from(start).floor();
    let end_vpn = VirtAddr::from(end).ceil();
    // 检查地址冲突
    for area in &memory_set.areas {
        if !(area.vpn_range.get_end() <= start_vpn || area.vpn_range.get_start() >= end_vpn) {
            return -1;
        }
    }
    memory_set.insert_framed_area(
        VirtAddr::from(start),
        VirtAddr::from(end),
        perm,
    );
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    if len == 0 {
        return -1;
    }
    // 检查起始地址页对齐
    if start % crate::config::PAGE_SIZE != 0 {
        return -1;
    }
    let end = start + len;
    let task = match crate::task::current_task() {
        Some(t) => t,
        None => return -1,
    };
    let memory_set = &mut task.memory_set;
    if memory_set.delete(VirtAddr::from(start), VirtAddr::from(end)) {
        0
    } else {
        -1
    }
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

