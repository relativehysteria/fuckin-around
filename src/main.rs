//! Buildscript for the bootloader and kernel.
//!
//! Requirements:
//!     nasm

use std::path::Path;
use std::process::Command;
use std::fs::create_dir_all;
use std::env::args;
use std::process::ExitStatus;

fn main() {
    // Get the paths to our working directories
    let netboot_path    = Path::new("qemu").join("netboot");
    let bootloader_path = Path::new("bootloader");

    // If we get `clean`, remove the build directories and exit
    if args().len() == 2 && args().nth(1) == Some("clean".to_string()) {
        // Remove the netboot directory
        let netboot = Command::new("rm")
            .args(["-rf", netboot_path.to_str().unwrap()])
            .status();

        // Clean the bootloader directory
        let bootloader = cargo_clean(bootloader_path);

        println!("Cleaned directories:");
        println!("\t{netboot_path:?} = {netboot:?}");
        println!("\t{bootloader_path:?} = {bootloader:?}");
        std::process::exit(0);
    }

    // Create the needed directories.
    // Directories not created here should already exist.
    create_dir_all(netboot_path.clone()).unwrap();

    // Get the path to the stage0 assembly and the assembled binary
    let stage0_bin  = netboot_path.clone().join("bootloader.0");
    let stage0_path = bootloader_path.join("src").join("stage0.asm");

    // Convert the paths to strings
    let stage0_bin = stage0_bin.to_str()
        .expect("Couldn't get the path to the stage0 output directory");
    let stage0_path = stage0_path.to_str()
        .expect("Couldn't find the path to the stage0 assembly.");

    // Assemble the stage0
    Command::new("nasm")
        .args(["-f", "bin", "-o", stage0_bin, stage0_path])
        .status()
        .expect("Couldn't assemble the stage0.");

    // Build the bootloader
    cargo_build(&bootloader_path)
        .expect("Couldn't build the bootloader");
}

fn cargo_build<T: AsRef<Path>>(cargo_dir: T) -> std::io::Result<ExitStatus> {
    #[cfg(debug_assertions)]
    let args = ["rustc", "--", "-C", "link-arg=-nostartfiles"];
    #[cfg(not(debug_assertions))]
    let args = ["rustc", "--release", "--", "-C", "link-arg=-nostartfiles"];

    Command::new("cargo")
        .current_dir(cargo_dir)
        .args(args)
        .status()
}

fn cargo_clean<T: AsRef<Path>>(cargo_dir: T) -> std::io::Result<ExitStatus> {
    Command::new("cargo")
        .current_dir(cargo_dir)
        .arg("clean")
        .status()
}
