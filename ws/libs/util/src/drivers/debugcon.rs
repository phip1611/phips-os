use x86::io::outb;

/// Driver to the Debug Connection (debugcon) device, which is typically
/// reachable via I/O port [`DebugCon::PORT`] on x86 in virtual machines.
pub struct DebugCon;

impl DebugCon {
    /// The typical port where we find this device in QEMU or Cloud Hypervisor.
    pub const PORT: u16 = 0xe9;

    /// Writes one byte to the debugcon port I/O device.
    pub fn write(byte: u8) {
        unsafe { outb(Self::PORT, byte) }
    }
}

impl core::fmt::Write for DebugCon {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        s.bytes().for_each(Self::write);
        Ok(())
    }
}
