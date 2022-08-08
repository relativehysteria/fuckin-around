//! Buildscript for the bootloader and kernel.
//!
//! Requirements:
//!     nasm

use std::path::Path;
use std::process::Command;
use std::fs::create_dir_all;
use std::env::args;

use elf_parser::ElfParser;

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
        let bootloader = Command::new("cargo")
            .current_dir(bootloader_path)
            .arg("clean")
            .status();

        println!("Cleaned directories:");
        println!("\t{netboot_path:?} = {netboot:?}");
        println!("\t{bootloader_path:?} = {bootloader:?}");
        return;
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
    let target = "i586-unknown-linux-gnu";
    Command::new("cargo")
        .current_dir(bootloader_path)
        .args(["build", "--release"])
        .status()
        .expect("Couldn't build the bootloader.");

    // Read the bootloader bytes
    let bootloader_bin = bootloader_path
        .join("target")
        .join(target)
        .join("release")
        .join("bootloader");
    let bootloader_bin = std::fs::read(bootloader_bin)
        .expect("Couldn't read the bootloader binary.");

    // Parse the bootloader bytes
    let parsed_bootloader = ElfParser::parse(&bootloader_bin);
}
