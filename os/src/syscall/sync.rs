use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    let id = if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() - 1
    };
    while process_inner.allocation.len() <= tid {
        process_inner
            .allocation
            .push(vec![Vec::new(), Vec::new()]);
        process_inner
            .need
            .push(vec![Vec::new(), Vec::new()]);
    }
    for i in 0..process_inner.allocation.len() {
        if process_inner.allocation[i][0].len() <= id {
            process_inner.allocation[i][0].resize(id + 1, 0);
        }
        if process_inner.need[i][0].len() <= id {
            process_inner.need[i][0].resize(id + 1, 0);
        }
    }
    if process_inner.available[0].len() <= id {
        process_inner.available[0].resize(id + 1, 0);
    }
    process_inner.available[0][id] = 1;

    id as isize
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    if process_inner.available[0][mutex_id] >= 1 {
        process_inner.available[0][mutex_id] -= 1;
        process_inner.allocation[tid][0][mutex_id] += 1;
    } else {
        process_inner.need[tid][0][mutex_id] += 1;
        let mut work = process_inner.available[0].clone();
        let mut finish = vec![false; process_inner.allocation.len()];
        let n = process_inner.allocation.len();
        let m = work.len();
        loop {
            let mut found = false;
            for i in 0..n {
                if finish[i] {
                    continue;
                }
                let mut ok = true;
                for j in 0..m {
                    let need_ij = if j < process_inner.need[i][0].len() {
                        process_inner.need[i][0][j]
                    } else {
                        0
                    };
                    if need_ij > work[j] {
                        ok = false;
                        break;
                    }
                }
                if ok {
                    for j in 0..m {
                        let alloc_ij = if j < process_inner.allocation[i][0].len() {
                            process_inner.allocation[i][0][j]
                        } else {
                            0
                        };
                        work[j] += alloc_ij;
                    }
                    finish[i] = true;
                    found = true;
                }
            }
            if !found {
                break;
            }
        }
        let safe = finish.iter().all(|&f| f);
        if process_inner.deadlock_detect_enabled && !safe {
            process_inner.need[tid][0][mutex_id] -= 1;
            drop(process_inner);
            return -0xdead;
        }
    }
    drop(process_inner);
    drop(process);
    mutex.lock();
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    process_inner.available[0][mutex_id] += 1;
    process_inner.allocation[tid][0][mutex_id] -= 1;
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    let i = process_inner.allocation[tid][1].len();
    for _ in i..res_count {
        process_inner.allocation[tid][1].push(0);
        process_inner.need[tid][1].push(0);
    }
    if process_inner.available[1].len() > id {
        process_inner.available[1][id] = res_count;
    } else {
        process_inner.available[1].resize(id + 1, 0);
        process_inner.available[1][id] = res_count;
    }
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    process_inner.available[1][sem_id] += 1;
    process_inner.allocation[tid][1][sem_id] -= 1;
    drop(process_inner);
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    if process_inner.available[1][sem_id] >= 1 {
        process_inner.available[1][sem_id] -= 1;
        process_inner.allocation[tid][1][sem_id] += 1;
    } else {
        process_inner.need[tid][1][sem_id] += 1;
        let mut work = process_inner.available[1].clone();
        let mut finish = vec![false; process_inner.allocation.len()];
        let n = process_inner.allocation.len();
        let m = work.len();
        loop {
            let mut found = false;
            for i in 0..n {
                if finish[i] {
                    continue;
                }
                let mut ok = true;
                for j in 0..m {
                    let need_ij = if j < process_inner.need[i][1].len() {
                        process_inner.need[i][1][j]
                    } else {
                        0
                    };
                    if need_ij > work[j] {
                        ok = false;
                        break;
                    }
                }
                if ok {
                    for j in 0..m {
                        let alloc_ij = if j < process_inner.allocation[i][1].len() {
                            process_inner.allocation[i][1][j]
                        } else {
                            0
                        };
                        work[j] += alloc_ij;
                    }
                    finish[i] = true;
                    found = true;
                }
            }
            if !found {
                break;
            }
        }
        let safe = finish.iter().all(|&f| f);
        if process_inner.deadlock_detect_enabled && !safe {
            process_inner.need[tid][1][sem_id] -= 1;
            drop(process_inner);
            return -0xdead;
        }
    }
    drop(process_inner);
    sem.down();
    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect");
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.deadlock_detect_enabled = enabled != 0;
    0
}
