#[inline(always)]
pub(crate) fn read_hardware_counter() -> u64 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        let mut aux = 0;
        std::arch::x86_64::__rdtscp(&mut aux)
    }
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let timer: u64;
        std::arch::asm!(
            "isb",
            "mrs {}, cntvct_el0",
            out(reg) timer,
            options(nomem, nostack, preserves_flags)
        );
        timer
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        compile_error!("Cycle counting not implemented for this architecture.");
    }
}

const SHIFT: u32 = 32;

pub struct TscTimer {
    ns_per_tsc_scaled: u128,
}

impl TscTimer {
    pub fn calibrate() -> Self {
        #[cfg(target_arch = "aarch64")]
        {
            let freq: u64;
            unsafe {
                std::arch::asm!("mrs {}, cntfrq_el0", out(reg) freq);
            }
            // freq is ticks/sec, we want ns per tick scaled
            // ns_per_tick = 1_000_000_000 / freq
            let ns_per_tsc_scaled = (1_000_000_000u128 << SHIFT) / freq as u128;
            return Self { ns_per_tsc_scaled };
        }

        #[cfg(target_arch = "x86_64")]
        {
            let tsc_per_ns = (0..15)
                .map(|_| {
                    let t0_wall = std::time::Instant::now();
                    let t0_tsc = read_hardware_counter();
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    let t1_tsc = read_hardware_counter();
                    let t1_wall = std::time::Instant::now();
                    (t1_tsc - t0_tsc, t1_wall.duration_since(t0_wall).as_nanos())
                })
                .max_by_key(|(tsc, _)| *tsc) // max TSC delta = least OS jitter
                .unwrap();

            let ns_per_tsc_scaled = ((tsc_per_ns.1 as u128) << SHIFT) / tsc_per_ns.0 as u128;
            Self { ns_per_tsc_scaled }
        }
    }

    #[inline(always)]
    pub fn tsc_to_ns(&self, tsc_delta: u64) -> u64 {
        // NOTE: No cross-core TSC guard. Invariant TSC is standard since ~2008
        // (Intel Nehalem / AMD Bulldozer). Any hardware older than ~2014 is
        // unsupported and will simply get inaccurate measurements, not a crash.
        ((tsc_delta as u128 * self.ns_per_tsc_scaled) >> SHIFT) as u64
    }

    #[inline(always)]
    pub fn measure<F: FnOnce() -> R, R>(&self, f: F) -> (R, u64) {
        let start = read_hardware_counter();
        let result = f();
        let end = read_hardware_counter();
        (result, self.tsc_to_ns(end - start))
    }
}
