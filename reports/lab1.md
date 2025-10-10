#### 增加

在TaskControlBlock结构体中添加了inner为每一个系统调用计数 并为其实现zero_init()将count初始化为0

包装了current_task_id()获取当前任务的 ID

在syscall分发器进行计数

#### 问答

1. ch2b_bad_address 输出 Segmentation fault (core dumped)
  
  ch2b_bad_instructions 输出 Illegal instruction (core dumped)

  bad_register 输出 Illegal instruction (core dumped)

sbi-rt = { version = "0.0.2", features = ["legacy"] }

2. 加电后程序执行至地址0x1014 执行汇编指令`jr t0` 此时寄存器t0值为0x80000000 即跳转到rustsbi入口

通过执行execute::execute_supervisor(0x80200000, hartid, opqaue, HSM.clone());将模式设置为S 并跳转到内核入口0x80200000