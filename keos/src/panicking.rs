//! KEOS panic handler.
use crate::thread::STACK_SIZE;
use addr2line::{Context, Frame};
use alloc::{borrow::Cow, sync::Arc};
use unwind::{DwarfReader, Peeker, StackFrame, UnwindContext};

#[derive(Clone)]
struct EhFrameReader;

impl EhFrameReader {
    fn get_eh_frame_start() -> usize {
        extern "C" {
            static __eh_frame_hdr_start: u8;
        }
        unsafe { &__eh_frame_hdr_start as *const _ as usize }
    }

    fn get_eh_frame_end() -> usize {
        extern "C" {
            static __eh_frame_end: u8;
        }
        unsafe { &__eh_frame_end as *const _ as usize }
    }
}

impl Peeker for EhFrameReader {
    fn read<T>(&self, ofs: usize) -> Option<T>
    where
        T: Copy,
    {
        let (start, end) = (Self::get_eh_frame_start(), Self::get_eh_frame_end());
        if ofs >= start && ofs + core::mem::size_of::<T>() < end {
            unsafe { (ofs as *const T).as_ref().cloned() }
        } else {
            None
        }
    }
}

struct BackTracePrinter<'a>(Frame<'a, gimli::EndianArcSlice<gimli::LittleEndian>>, bool);

impl<'a> core::fmt::Display for BackTracePrinter<'a> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.1 {
            if let Some(Ok(name)) = self.0.function.as_ref().map(|n| n.demangle()) {
                writeln!(formatter, "{}", name)?;
            } else {
                writeln!(formatter, "?")?;
            }
        }
        if let Some(file) = self.0.location.as_ref().and_then(|n| n.file) {
            write!(formatter, "                         at {}:", file)?;
        } else {
            write!(formatter, "                         at ?:")?;
        }
        if let Some(line) = self.0.location.as_ref().and_then(|n| n.line) {
            write!(formatter, "{}:", line)?;
        } else {
            write!(formatter, "?:")?;
        }
        if let Some(col) = self.0.location.as_ref().and_then(|n| n.column) {
            write!(formatter, "{}", col)
        } else {
            write!(formatter, "?")
        }
    }
}

static mut DEBUG_CONTEXT: Option<Context<gimli::EndianArcSlice<gimli::LittleEndian>>> = None;

#[allow(dead_code)]
#[allow(clippy::empty_loop)]
#[inline(never)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!(
        "\n\n========== KERNEL PANIC!!! [core #{}] ==========\nKEOS {}\n",
        abyss::x86_64::intrinsics::cpuid(),
        info
    );

    let frame = unwind::StackFrame::current();
    println!("Stack Backtrace:");

    fn do_backtrace(depth: &mut usize, frame: &StackFrame) {
        let pc = frame.pc() as u64;
        *depth += 1;
        if let Some(ctxt) = unsafe { DEBUG_CONTEXT.as_ref() } {
            if let Ok(mut frames) = ctxt.find_frames(pc) {
                if let Ok(Some(frame)) = frames.next() {
                    println!(
                        "  {:2}: 0x{:016x}  - {}",
                        depth,
                        pc,
                        BackTracePrinter(frame, true)
                    );
                    while let Ok(Some(frame)) = frames.next() {
                        println!("{}", BackTracePrinter(frame, false));
                    }
                    return;
                }
            }
        }
        println!("  {:2}: 0x{:016x}  - ?", depth, pc);
    }

    let sp_hi = frame.sp() & !(STACK_SIZE - 1);
    if unsafe {
        UnwindContext::new_boxed(
            frame,
            sp_hi..sp_hi + STACK_SIZE,
            DwarfReader::from_peeker(EhFrameReader::get_eh_frame_start(), EhFrameReader),
        )
        .unwind_raise_exception_with_hook(
            0,
            |depth, this, _| do_backtrace(depth, &this.frame),
            |_| loop {},
        )
    }
    .is_err()
    {
        println!("?: ? at ?:?:?");
    }
    loop {}
}

/// Load debugging symbols from kernel image
/// # Safety
/// Only be called once
pub unsafe fn load_debug_infos() -> Result<(), ()> {
    use object::{Object, ObjectSection};
    let kernel_disk = abyss::dev::get_bdev(0).ok_or(())?;
    let image_size = kernel_disk.block_cnt() * kernel_disk.block_size();
    let mut kernel_image = alloc::vec![0u8; image_size].into_boxed_slice();
    kernel_disk.read_bios(&mut Some((0, kernel_image.as_mut())).into_iter())?;

    let kernel = object::File::parse(kernel_image.as_ref()).map_err(|_| ())?;
    let dwarf = gimli::Dwarf::load(|id| {
        let data = kernel
            .section_by_name(id.name())
            .and_then(|section| section.uncompressed_data().ok())
            .unwrap_or(Cow::Borrowed(&[]));
        let data: Arc<[u8]> = Arc::from(data.as_ref());
        Ok(gimli::EndianArcSlice::new(data, gimli::LittleEndian))
    })
    .map_err(|_: ()| ())?;
    DEBUG_CONTEXT = Some(Context::from_dwarf(dwarf).map_err(|_| ())?);
    Ok(())
}
