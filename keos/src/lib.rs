//! KAIST educational Operating System.

#![no_std]
#![feature(
    asm_const,
    alloc_error_handler,
    alloc_layout_extra,
    naked_functions,
    lang_items,
    new_uninit,
    const_mut_refs
)]
#![deny(missing_docs)]

#[macro_use]
extern crate abyss;
extern crate alloc;

pub mod fs;
pub mod interrupt;
pub mod mm;
pub mod panicking;
pub mod sync;
pub mod thread;

pub use abyss::{addressing, debug, info, print, println, spin_lock, warning, MAX_CPU};

/// The first function of rust world.
#[no_mangle]
unsafe fn rust_main(core_id: usize, regions: abyss::boot::Regions) {
    info!("boot KeOS...");
    // Init memory.
    crate::mm::init_mm(regions);
    // Init pci device
    info!("initialize devices...");
    abyss::dev::pci::init();
    // Load debug symbols
    info!("load debug symbols...");
    if crate::panicking::load_debug_infos().is_err() {
        warning!("Failed to read kernel image. Disabling stack backtrace.");
    }
    info!("initialize fs...");
    crate::fs::init_fs();

    extern "Rust" {
        fn main();
    }
    main();

    #[cfg(feature = "smp")]
    abyss::boot::bootup_mps();

    // Now kernel is ready to serve task.
    crate::thread::scheduler::start_idle(core_id);
}

/// The first function of rust world for ap.
#[no_mangle]
#[cfg(feature = "smp")]
unsafe fn rust_ap_main(core_id: usize) {
    extern "Rust" {
        fn ap_main();
    }
    ap_main();
    crate::thread::scheduler::start_idle(core_id);
}

// Test utilities
#[doc(hidden)]
pub trait TestFn
where
    Self: Sync + Send,
{
    fn run(&'static self) -> bool;
}

/// Run the given tests.
pub fn do_tests(tests: &'static [&'static dyn TestFn]) {
    impl<T> TestFn for T
    where
        T: Fn() + Send + Sync + 'static,
    {
        fn run(&'static self) -> bool {
            print!("test {} ... ", core::any::type_name::<T>());
            if crate::thread::ThreadBuilder::new(core::any::type_name::<T>())
                .spawn(|| self())
                .join()
                == 0
            {
                println!("ok");
                true
            } else {
                println!("FAILED");
                false
            }
        }
    }
    crate::thread::ThreadBuilder::new("test_main").spawn(move || {
        let (total, mut succ) = (tests.len(), 0);
        println!(
            "running {} test{}",
            total,
            if total == 1 { "" } else { "s" }
        );

        for test in tests {
            if test.run() {
                succ += 1;
            }
        }
        println!(
            "test result: {}. {} passed; {} failed",
            if total == succ { "ok" } else { "FAILED" },
            succ,
            total - succ
        );

        use abyss::x86_64::pio::Pio;
        #[cfg(feature = "exit_on_qemu")]
        Pio::new(0x604).write_u32(0 | 0x2000);
    });
}

pub use abyss::x86_64::intrinsics;
