// Copyright 2021 Computer Architecture and Systems Lab
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// <https://github.com/rust-lang/rust/blob/master/library/panic_unwind/src/dwarf/eh.rs>

use crate::{
    DwarfReader, Encoding, ExceptionHandlingPhase, FrameDescriptionEntry, Peeker,
    PersonalityResult, StackFrame,
};
#[derive(Debug)]
enum EhAction {
    None,
    Cleanup(usize),
    Catch(usize),
    Terminate,
}
use core::arch::asm;

#[lang = "eh_personality"]
#[no_mangle]
#[doc(hidden)]
fn rust_eh_personality(
    phase: ExceptionHandlingPhase,
    fde: &FrameDescriptionEntry,
    frame: &mut StackFrame,
) -> PersonalityResult {
    if let Some(action) = find_eh_action(fde, frame) {
        if matches!(phase, ExceptionHandlingPhase::Search) {
            match action {
                EhAction::None | EhAction::Cleanup(_) => PersonalityResult::Continue,
                EhAction::Catch(_) => PersonalityResult::Stop,
                EhAction::Terminate => PersonalityResult::Error,
            }
        } else {
            match action {
                EhAction::None => PersonalityResult::Continue,
                EhAction::Cleanup(lpad) | EhAction::Catch(lpad) => PersonalityResult::Run(lpad),
                EhAction::Terminate => PersonalityResult::Error,
            }
        }
    } else {
        PersonalityResult::Error
    }
}

const USING_SJLJ_EXCEPTIONS: bool = cfg!(all(target_os = "ios", target_arch = "arm"));

fn find_eh_action(fde: &FrameDescriptionEntry, frame: &StackFrame) -> Option<EhAction> {
    #[derive(Clone)]
    struct Reader;
    impl Peeker for Reader {
        fn read<T>(&self, ofs: usize) -> Option<T>
        where
            T: Copy,
        {
            unsafe { (ofs as *const T).as_ref().cloned() }
        }
    }

    if let Some(lsda) = fde.lsda {
        let mut reader = DwarfReader::from_peeker(lsda, Reader);
        let lpad_base = match reader.read::<u8>()? {
            0xff => fde.pc.start,
            encoding => reader.read_with_encoding(Encoding::from(encoding))?,
        };
        let ttype_encoding = reader.read::<u8>()?;
        if ttype_encoding != 0xff {
            // Rust doesn't analyze exception types, so we don't care about the type table
            reader.read_uleb128();
        }

        let call_site_encoding = Encoding::from(reader.read::<u8>()?);
        let call_site_table_length = reader.read_uleb128()?;
        let action_table = reader.current() + call_site_table_length;
        let ip = match frame.pc() {
            0 => 0,
            pc => pc - 1,
        };
        if !USING_SJLJ_EXCEPTIONS {
            while reader.current() < action_table {
                let cs_start = reader.read_with_encoding(call_site_encoding)?;
                let cs_len = reader.read_with_encoding(call_site_encoding)?;
                let cs_lpad = reader.read_with_encoding(call_site_encoding)?;
                let cs_action = reader.read_uleb128()?;
                // Callsite table is sorted by cs_start, so if we've passed the ip, we
                // may stop searching.
                if ip < fde.pc.start + cs_start {
                    break;
                }
                if ip < fde.pc.start + cs_start + cs_len {
                    if cs_lpad == 0 {
                        return Some(EhAction::None);
                    } else {
                        let lpad = lpad_base + cs_lpad;
                        return Some(interpret_cs_action(cs_action, lpad));
                    }
                }
            }
            // Ip is not present in the table.  This should not happen... but it does: issue
            // #35011. So rather than returning EHAction::Terminate, we do this.
            Some(EhAction::None)
        } else {
            // SjLj version:
            // The "IP" is an index into the call-site table, with two exceptions:
            // -1 means 'no-action', and 0 means 'terminate'.
            match ip as isize {
                -1 => return Some(EhAction::None),
                0 => return Some(EhAction::Terminate),
                _ => (),
            }
            let mut idx = ip;
            loop {
                let cs_lpad = reader.read_uleb128()?;
                let cs_action = reader.read_uleb128()?;
                idx -= 1;
                if idx == 0 {
                    // Can never have null landing pad for sjlj -- that would have
                    // been indicated by a -1 call site index.
                    let lpad = (cs_lpad + 1) as usize;
                    return Some(interpret_cs_action(cs_action, lpad));
                }
            }
        }
    } else {
        Some(EhAction::None)
    }
}

fn interpret_cs_action(cs_action: usize, lpad: usize) -> EhAction {
    if cs_action == 0 {
        // If cs_action is 0 then this is a cleanup (Drop::drop). We run these
        // for both Rust panics and foreign exceptions.
        EhAction::Cleanup(lpad)
    } else {
        // Stop unwinding Rust panics at catch_unwind.
        EhAction::Catch(lpad)
    }
}

#[cfg(target_arch = "x86_64")]
#[no_mangle]
#[naked]
unsafe extern "C" fn _Unwind_Resume(_arg: usize) -> ! {
    asm!("jmp [rdi]", options(noreturn))
}

#[cfg(target_arch = "aarch64")]
#[no_mangle]
#[naked]
unsafe extern "C" fn _Unwind_Resume(_arg: usize) -> ! {
    asm!("ldr x2, [x0]", "br x2", options(noreturn))
}
