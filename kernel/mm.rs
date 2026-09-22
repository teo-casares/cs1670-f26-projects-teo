// Function to zero out a block of memory, implemented in
// assembly in `mm.S`.
unsafe extern "C" {
    pub fn memzero(ptr: *mut u8, len: usize);
}
