//! Welcome to the KeV project.
//!
//! Virtualization is an increasingly ubiquitous feature of modern computer systems,
//! and a rapidly evolving part of the system stack. Hardware vendors are adding new features to support more efficient virtualization,
//! OS designs are adapting to perform better in VMs, and VMs are an essential component in cloud computing.
//! Thus, understanding how VMs work is essential to a complete education in computer systems.
//!
//! In this project, you will skim through the basic components that runs on real virtual machine monitor like KVM.
//! From what you learn, you will build your own type 2 hypervisor and finally extend the hypervisor
//! as an open-ended course project.
//!
//! In KeV project, we will not bother you from the time-consuming edge case handling and the hidden test cases.
//! The score that you see when run the grading scripts is your final score.
//! We want to keep this project as easy as possible.
//! If you have suggestions on how we can reduce the unnecessary overhead of assignments,
//! cutting them down to the important underlying issues, please let us know.
//!
//! ## Projects
//! The KeV project consists of 5 projects.
//!
//! 1. [KeOS]
//! 2. [VMCS and VMexits]
//! 3. [Hardware virtualization]
//! 4. [Interrupt and I/O virtualization]
//! 5. [Final project]
//!
//! ### Rust
//! We pick the Rust as a language for project. This is because we believe that after overcome the barriers to learn,
//! memory safety and ownership rule of Rust could significantly reduce the debugging time while implement an operating system.
//!
//! ## Getting Started
//! You can bootstrap your KEOS project with following command lines.
//! ```/bin/bash
//! $ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
//! $ sudo apt install -yy qemu-system-x86 grub xorriso grub2-common grub-pc mtools
//! $ git clone https://github.com/casys-kaist/kev
//! ```
//!
//! **PLEASE DO NOT MAKE ANY PUBLIC FORK OF THIS PROJECT.**
//! This is strongly denied from the license of the KeV Project. You **MUST** not redistribute
//! the your works based on the given template.
//!
//! ### Enable nested virtualization
//! See the following docs: <https://docs.fedoraproject.org/en-US/quick-docs/using-nested-virtualization-in-kvm/>
//!
//! ## Notes
//! ### Tips for projects
//! We recommend to do `todo`-oriented implementations. Run the project and fill the reached `todo!()`.
//! After that, run the project again and fill a new `todo!()`. Do this iteration until you passed all the testcases!
//! Thanks to the rich backtrace, you can easily follow the call stack of the confronted `todos`.
//!
//! ### Grading
//! When we grade (except the final project), we overwrite the all the files of followings:
//! - `abyss/*`
//! - `keos/*`
//! - `fs/*`
//! - `lib/*`
//! - `projects/project*/src/main.rs`
//!
//! That is, your code MUST PASS the test cases without any change of the listed files.
//! If your code is not compiled, you will get 0pts.
//! Also cheating is strongly prohibited by **TODO**.
//!
//! ## Debugging with GDB
//!
//! ### Play with GDB
//!
//! Once you runs `$ GDB=1 cargo run` in each `project` directory, QEMU waits for a GDB attach from TCP port 1234.
//! The command also creates a `.gdbinit` script that connects to TCP port 1234 and initializes several debug configurations.
//! With a new terminal, run `$ gdb keos_kernel` in each project directory will immediately start the debugging process.
//!
//! Before running the `gdb`, you need to edit the `~/.gdbinit` file to allow `gdbinit` script to be run.
//! Add the following line in your `~/.gdbinit` file:
//! ```
//! set auto-load safe-path /
//! ```
//!
//! After running `gdb`, you will see that execution stops at the initial stage, as shown below:
//!
//! ```bash
//! $ gdb
//! warning: No executable has been specified and target does not support
//! determining executable automatically.  Try using the "file" command.
//! 0x000000000000fff0 in ?? ()
//! (gdb)
//! ```
//!
//! Now, you can continue to execute keos by type `c`.
//!
//! #### Inspect each core
//! With QEMU, GDB treats each CPU core as a thread. When debugging with a multi-core environment, you should consider each CPU core independently.
//! When some cores are normal, other cores may already be panicked.
//! You can see the state of each core by running the following command:
//! `(gdb) info threads`
//!
//! The output will show the state of each thread, which CPU core it belongs to, and what stack frame each core resides in.
//! Here's an example of the initial state of all cores:
//!
//! ```
//! (gdb) info threads
//! Id   Target Id         Frame
//! * 1    Thread 1 (CPU#0 [running]) 0x000000000000fff0 in ?? ()
//! 2    Thread 2 (CPU#1 [running]) 0x000000000000fff0 in ?? ()
//! 3    Thread 3 (CPU#2 [running]) 0x000000000000fff0 in ?? ()
//! 4    Thread 4 (CPU#3 [running]) 0x000000000000fff0 in ?? ()
//! ```
//!
//! To switch to a specific thread, use the following command:
//!
//! `(gdb) thread {thread_id}`
//!
//! Replace `{thread_id}` with the ID of the thread you're interested in.
//!
//! #### Backtrace & Frame
//! The `backtrace` command shows the call stack of the current thread.
//! The call stack is divided into several stack frames, each of which has its own state information about the execution stack and registers when calling the function which is matched to the next upper frame.
//! To show the call stack of the current thread, use the following command:
//!
//! ```
//! (gdb) bt
//! ```
//!
//! To switch to a specific frame from the backtrace result, use the following command:
//!
//! ```
//! (gdb) frame {frame_id}
//! ```
//!
//! Replace `{frame_id}` with the ID of the frame you're interested in. After switching to the target frame, you can extract the frame context using the following commands:
//!
//! ```
//! (gdb) info args
//! (gdb) info locals
//! (gdb) i r
//! ```
//!
//! When you encounter a panic message during a test, the first step is to identify the thread that triggered the panic and switch to it.
//! Next, using the backtrace command, you can identify the frames that are likely to have caused the panic and examine the local variables, arguments, and other registers.
//! By following the steps, you can narrow down the potential locations of the bug and ultimately pinpoint the source of the error.
//!
//! Normal breakpoints may not be work. You should use hardware breakpoints to set up a breakpoint as follow.
//! ```
//! (gdb) hb * {address of breakpoint}
//! ```
//!
//! You also see the source that the current cpu is executed by typing follow commands:
//! ```
//! (gdb) layout asm
//! (gdb) layout src
//! ```
//!
//! ### Examples
//!
//! ```
//! (gdb) hbreak function_name 	# ex) (gdb) hbreak Rounrobin::new
//! (gdb) hbreak *address		# ex) (gdb) hbreak *0x1000
//! (gdb) hbreak (file:)line	# ex) (gdb) hbreak rr.rs:95		// file name can be ommitted
//! ```
//!
//! #### Example 1
//!
//! Assume you want to debug from the test case `check_affinity` in project1, and the code spot you want to debug is the closure entry at `main.rs:115`. You can easily set a breakpoint with the commands below.
//!
//! ```
//! (gdb) hbreak main.rs:115
//! or
//! (gdb) hbreak check_affinity::{{closure}}
//! ```
//!
//! If you want to care only one core, it would be nice to set a breakpoint with `thread apply` like below.
//!
//! ```
//! (gdb) thread apply 1 hbreak main.rs:115
//! (gdb) c
//! Continuing.
//! Thread 1 hit Breakpoint 1, project1::round_robin::check_affinity::{{closure}} () at project1/src/main.rs:115
//! 115                     let _p = InterruptGuard::new();
//! ```
//!
//! If you want to set an additional breakpoint for the same function, peek some source and then set a breakpoint with only the line number.
//!
//! ```
//! (gdb) l
//! 110             for i in 0..MAX_CPU {
//! 111                 // Diable all cores' interrupt.
//! 112                 let cnt = cnt.clone();
//! 113                 let scheduler = scheduler.clone();
//! 114                 let handle = ThreadBuilder::new(format!("t{}", i)).spawn(move || {
//! 115                     let _p = InterruptGuard::new();
//! < ....>
//! 127                         } else if *c % MAX_CPU == cid {
//! 128                             scheduler.push_to_queue(Thread::new(cid.to_string()));
//! 129                             *c += 1;
//! (gdb) thread apply 1 break 128
//! (gdb) c
//! Continuing.
//! Thread 1 hit Breakpoint 1, project1::round_robin::check_affinity::{{closure}} () at project1/src/main.rs:128
//! 128                             scheduler.push_to_queue(Thread::new(cid.to_string()));
//! ```
//!
//! #### Example 2
//!
//! With a hardware breakpoint for an address, you can stop in a guest code section.
//!
//! In project2, the host copies guest code to newly allocated pages which start with GVA: `0x4000`. If you set a breakpoint with the command `(gdb) hbreak *0x4000`, you can stop at the entry of the guest code.
//!
//! ```
//! <.....>
//! 0x000000000000fff0 in ?? ()
//! (gdb) hbreak *0x4000
//! Hardware assisted breakpoint 1 at 0x4000
//! (gdb) c
//! Continuing.
//!
//! Thread 1 hit Breakpoint 1, 0x0000000000004000 in ?? ()
//! (gdb) x/4i $rip
//! => 0x4000:      mov    $0xcafe,%edi
//!  0x4005:      xor    %eax,%eax
//!  0x4007:      vmcall
//!  0x400a:      add    %al,(%rax)
//! (gdb)
//! ```
//!
//! The above shows that the guest will execute a `vmcall` instruction to stop the vcpu execution with an exit code 0xcafe, and the instructions being executed are fetched from GVA: `0x4000`.
//!
//! #### Example 3
//!
//! If you want to stop at a breakpoint for certain situation, you can add some condition on the breakpoint.
//!
//! The example below stops at walk when the parameter gpa passed is 0xcafe0000.
//!
//! ```
//! (gdb) hbreak walk if gpa.__0 == 0xcafe0000
//! Hardware assisted breakpoint 3 at 0xffffff0000197721: file project3/src/ept.rs, line 539.
//! (gdb) c
//! Continuing.
//!
//! Thread 2 hit Breakpoint 3, project3::ept::ExtendedPageTable::walk (self=0xffffff0002cad338, gpa=kev::vm::Gpa (3405643776))
//!   at project3/src/ept.rs:539
//! 539             if gpa_ & 0xFFF != 0 {
//! ```
//!
//! With breakpoint condition, you can skip other function call with arguments that are not what you are interested.
//!
//! [KeOS]: ../project1
//! [VMCS and VMexits]: ../project2
//! [Hardware virtualization]: ../project3
//! [Interrupt and I/O virtualization]: ../project4
//! [Final project]: ../project5

#![no_std]

