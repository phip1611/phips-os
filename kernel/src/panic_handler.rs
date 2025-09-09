use core::panic::PanicInfo;
use log::error;

#[panic_handler]
fn handle_panic(panic_info: &PanicInfo) -> ! {
    error!("Kernel panic! {}", panic_info);
    loop {
        core::hint::spin_loop()
    }
}
