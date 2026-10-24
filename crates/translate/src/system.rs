/// Hardware probe for the setup dialog: RAM (total + available) and logical cores.
/// Pure-Rust reads only (\`sysinfo\` system feature) — no C dependency, no unsafe in our
/// code. A probe failure is a normal state (\`None\`), not an error; the dialog then says
/// the hardware check is unavailable. On wasm this is always \`None\`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryInfo {
    pub total: u64,
    pub available: u64,
}

/// Bytes of RAM (total, available). \`None\` on wasm or when the platform probe fails.
pub fn memory_info() -> Option<MemoryInfo> {
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // ponytail: refreshes the whole system list each call; the dialog opens rarely, so
        // the few-ms cost is fine — cache it if this ever lands on a hot path.
        use sysinfo::System;
        let mut sys = System::new();
        sys.refresh_memory();
        let total = sys.total_memory();
        let available = sys.available_memory();
        (total > 0).then_some(MemoryInfo { total, available })
    }
}

/// Logical cores (\`std::thread::available_parallelism\`), minimum 1.
pub fn cpu_cores() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cores_are_at_least_one() {
        assert!(cpu_cores() >= 1);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn memory_probe_returns_real_ram_on_native() {
        let m = memory_info().expect("native probe");
        assert!(m.total > 0);
        assert!(m.available <= m.total);
    }
}
