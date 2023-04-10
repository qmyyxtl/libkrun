extern crate seccomp;
// extern crate libc;
extern crate syscalls;

use self::seccomp::*;
use self::syscalls::SyscallNo::*;
pub fn create_default_seccomp_rule(sysno: usize) -> self::seccomp::Rule {
    let rule = Rule::new(sysno,
            Compare::arg(0)
                    .with(0)
                    .using(Op::Ge)
                    .build().unwrap(),
            Action::Allow 
        );
    return rule;
}

pub fn add_seccomp_filter() {
    //add seccomp filter
    let mut ctx = Context::default(Action::KillProcess).unwrap();
    // ctx.add_rule(create_default_seccomp_rule(SYS_rt_sigaction as usize)).unwrap();
    // ctx.add_rule(create_default_seccomp_rule(SYS_mmap as usize)).unwrap();
    // ctx.add_rule(create_default_seccomp_rule(SYS_statx as usize)).unwrap();
    // ctx.add_rule(create_default_seccomp_rule(SYS_msync as usize)).unwrap();
    // ctx.add_rule(create_default_seccomp_rule(SYS_msync as usize)).unwrap();
    // ctx.add_rule(create_default_seccomp_rule(SYS_rt_sigprocmask as usize)).unwrap();
    // ctx.add_rule(create_default_seccomp_rule(SYS_clone as usize)).unwrap();

    // ctx.add_rule(create_default_seccomp_rule(SYS_clock_getres as usize)).unwrap();
    // ctx.add_rule(create_default_seccomp_rule(SYS_clock_gettime as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_fadvise64 as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_fallocate as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_fdatasync as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_fcntl as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_fstat as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_ftruncate as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_preadv as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_pwritev as usize)).unwrap();
    // ctx.add_rule(create_default_seccomp_rule(SYS_readv as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_lseek as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_fsync as usize)).unwrap();
    // ctx.add_rule(create_default_seccomp_rule(SYS_writev as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_mkdirat as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_linkat as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_openat as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_readlinkat as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_unlinkat as usize)).unwrap();
    // ctx.add_rule(create_default_seccomp_rule(SYS_renameat as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_symlinkat as usize)).unwrap();
    // ctx.add_rule(create_default_seccomp_rule(SYS_clock_nanosleep as usize)).unwrap();
    // ctx.add_rule(create_default_seccomp_rule(SYS_sched_yield as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_getrandom as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_utimensat as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_renameat2 as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_socket as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_connect as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_recvfrom as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_sendto as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_shutdown as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_getpeername as usize)).unwrap();
    




    ctx.add_rule(create_default_seccomp_rule(SYS_futex as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_read as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_write as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_epoll_wait as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_epoll_ctl as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_newfstatat as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_umask as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_brk as usize)).unwrap();
    
    
    ctx.add_rule(create_default_seccomp_rule(SYS_close as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_dup as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_fgetxattr as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_madvise as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_exit_group as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_getdents64 as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_ioctl as usize)).unwrap();
    ctx.add_rule(create_default_seccomp_rule(SYS_fchownat as usize)).unwrap();
    ctx.load().unwrap();
}
