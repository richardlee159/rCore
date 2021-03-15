use crate::batch;

pub fn sys_exit(exit_code: i32) -> ! {
    info!("Application exited with code {}", exit_code);
    batch::run_next_app();
}