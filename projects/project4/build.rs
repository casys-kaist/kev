use core::panic;

include!("../build.rs");

fn main() {
    if !Path::new("rootfs/gKeOS").exists() {
        let cmd = std::process::Command::new("cargo")
            .current_dir(Path::new("../../guest/project4"))
            .args(["build", "--target=../.cargo/x86_64-unknown-keos.json"])
            .output()
            .expect("Failed to launch cargo to build guest kernel.");
        if !cmd.status.success() {
            panic!(
                "Failed to build guest OS.\n{}",
                std::str::from_utf8(cmd.stderr.as_ref()).unwrap()
            );
        }
        std::fs::rename(
            Path::new("../../guest/target/x86_64-unknown-keos/debug/project4"),
            Path::new("rootfs/gKeOS"),
        )
        .expect("Failed to rename guest kernel to rootfs/gKeOS.");
    }
    build_fs();
}
