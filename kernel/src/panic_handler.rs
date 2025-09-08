use core::panic::PanicInfo;

#[panic_handler]
fn handle_panic(_panic_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop()
    }
}
