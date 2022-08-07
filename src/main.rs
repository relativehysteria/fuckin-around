//! Buildscript for the bootloader and kernel.
//!
//! Requirements:
//!     nasm

use std::path::Path;
use std::process::Command;
use std::fs::create_dir_all;
use std::env::args;

fn main() {
    // Get the path to the PXE netboot directory
    let netboot_path = Path::new("qemu").join("netboot");

    // If we get `clean`, remove the build directories and exit
    if args().len() == 2 && args().nth(1) == Some("clean".to_string()) {
        Command::new("rm")
            .args(["-rf", netboot_path.to_str().unwrap()])
            .status()
            .expect("Couldn't remove {netboot_path}.");
        std::process::exit(0);
    }

    // Create the needed directories
    create_dir_all(netboot_path.clone()).unwrap();

    // Get the path to the stage0 assembly and the assembled binary
    let stage0_bin  = netboot_path.clone().join("bootloader.0");
    let stage0_path = Path::new("bootloader").join("src").join("stage0.asm");

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
}
