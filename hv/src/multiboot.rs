//! Minimal Multiboot1 info walker (modules). Freestanding only.

/// Multiboot1 boot-info magic left in EAX by compliant loaders.
pub const MULTIBOOT1_BOOT_MAGIC: u32 = 0x2BAD_B002;

const FLAG_MODS: u32 = 1 << 3;

/// One Multiboot1 module (start/end are physical addresses).
#[derive(Clone, Copy, Debug)]
pub struct Module {
    pub start: u32,
    pub end: u32,
}

impl Module {
    pub fn bytes(self) -> &'static [u8] {
        let len = self.end.saturating_sub(self.start) as usize;
        // SAFETY: Multiboot modules are identity-mapped by the trampoline for the
        // low 1 GiB; loader guarantees the range is valid for the L1 life.
        unsafe { core::slice::from_raw_parts(self.start as usize as *const u8, len) }
    }
}

/// Parse Multiboot1 info at `info_addr`; returns modules in order.
pub fn modules(info_addr: u32) -> ModuleList {
    if info_addr == 0 {
        return ModuleList {
            mods_addr: 0,
            remaining: 0,
        };
    }
    // SAFETY: Multiboot info is in the low identity map.
    let flags = unsafe { *(info_addr as *const u32) };
    if flags & FLAG_MODS == 0 {
        return ModuleList {
            mods_addr: 0,
            remaining: 0,
        };
    }
    let mods_count = unsafe { *((info_addr as usize + 20) as *const u32) };
    let mods_addr = unsafe { *((info_addr as usize + 24) as *const u32) };
    ModuleList {
        mods_addr: mods_addr as usize,
        remaining: mods_count,
    }
}

pub struct ModuleList {
    mods_addr: usize,
    remaining: u32,
}

impl Iterator for ModuleList {
    type Item = Module;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 || self.mods_addr == 0 {
            return None;
        }
        // Multiboot1 module struct: start, end, string, reserved (16 bytes).
        // SAFETY: module list provided by loader in identity-mapped RAM.
        let start = unsafe { *(self.mods_addr as *const u32) };
        let end = unsafe { *((self.mods_addr + 4) as *const u32) };
        self.mods_addr += 16;
        self.remaining -= 1;
        Some(Module { start, end })
    }
}
