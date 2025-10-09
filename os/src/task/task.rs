//! Types related to task management

use super::TaskContext;
use crate::sync::UPSafeCell;
use super::TASK_MANAGER;

/// The task control block (TCB) of a task.
#[derive(Debug)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// 内部同步数据
    pub inner: UPSafeCell<TaskControlBlockInner>,
}
#[derive(Debug)]
pub struct TaskControlBlockInner {
    pub syscall_count: [usize; 512],
}
impl TaskControlBlock {
    /// 创建一个所有字段初始化为零的 TaskControlBlock
    pub fn zero_init() -> Self {
        TaskControlBlock {
            task_status: TaskStatus::UnInit,
            task_cx: TaskContext::zero_init(),
            inner: unsafe {
                UPSafeCell::new(TaskControlBlockInner {
                    syscall_count: [0; 512],
                })
            },
        }
    }
}
/// 获取当前任务的 ID
pub fn current_task_id() -> usize {
    TASK_MANAGER.inner.exclusive_access().current_task
}
/// The status of a task
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
