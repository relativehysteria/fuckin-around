//! Buildscript for the bootloader and kernel.
//!
//! Requirements:
//!     nasm
//!     ld.lld

use std::path::Path;
use std::process::Command;
use std::fs::create_dir_all;
use std::env::args;
use std::error::Error;

use elf_parser::ElfParser;

/// Maximum stage0/bootloader size allowed by PXE
const MAX_BOOTLOADER_SIZE: u64 = 32 * 1024;

/// Execution origin of the stage0 binary
const STAGE0_ORIGIN: u64 = 0x7c00;

/// Extract LOADable segments out of an elf file and flatten them into a single
/// image.
///
/// Returns (entry, base, raw_image), where:
///     * `entry` - virtual address of the image's entry point
///     * `base`  - virtual address of where in memory the image is to be loaded
///     * `flat_image` - the flat image bytes
fn flatten_elf<P: AsRef<Path>>(file_path: P) -> Option<(u32, u32, Vec<u8>)> {
    // Parse the ELf
    let elf = std::fs::read(file_path).ok()?;
    let elf = ElfParser::parse(&elf)?;

    // Compute the bounds of the loaded image such that we can find its base,
    // its end, and whether the entry point points to its
    let mut image_start = None;
    let mut image_end   = None;
    elf.headers(|vaddr, memsz, _bytes, _read, _write, _execute| {
        let end = vaddr.checked_add(memsz.checked_sub(1)?)?;

        // Calculate the vaddr of the end of the segment
        // First section initializes the values
        if image_start.is_none() {
            image_start = Some(vaddr);
            image_end   = Some(end);
        }

        // Find the lowest base and the highest end
        image_start = image_start.map(|x| core::cmp::min(x, vaddr));
        image_end   = image_end.map(|x| core::cmp::max(x, end));
        Some(())
    })?;

    // Make sure that we have at least one section.
    let image_start = image_start?;
    let image_end   = image_end?;

    // Calculate the flat image size
    let image_size: usize = image_end.checked_sub(image_start)?.checked_add(1)?
        .try_into().ok()?;

    // Allocate space for the flat image
    let mut flat_image: Vec<u8> = vec![0u8; image_size];

    // Flatten the image
    elf.headers(|vaddr, memsz, bytes, _read, _write, _execute| {
        // Find the offset for this segment in the flat image
        let flat_off: usize = (vaddr - image_start).try_into().ok()?;
        let size:     usize = memsz.try_into().ok()?;

        // Compute the number of bytes to initialize
        let to_copy = std::cmp::min(size, bytes.len());

        // Copy the initialized bytes from the PE into the flattened image
        flat_image[flat_off..flat_off.checked_add(to_copy)?]
            .copy_from_slice(bytes);

        Some(())
    })?;

    // Make sure the entry point points into the image
    let image_start = image_start.try_into().ok()?;
    let image_end   = image_end.try_into().ok()?;
    if elf.entry < image_start || elf.entry > image_end {
        return None;
    }

    // Return the image
    let entry = elf.entry.try_into().ok()?;
    let base  = image_start.try_into().ok()?;
    Some((entry, base, flat_image))
}

fn main() -> Result<(), Box<dyn Error>>{
    // Get the paths to our working directories
    let netboot_path    = Path::new("qemu").join("netboot");
    let build_path      = Path::new("build");
    let bootloader_path = Path::new("bootloader");
    let kernel_path     = Path::new("kernel");

    // If we get `clean`, remove the build directories and exit
    if args().len() == 2 && args().nth(1) == Some("clean".to_string()) {
        // Remove the netboot directory
        let netboot = Command::new("rm")
            .args(["-rf", netboot_path.to_str().unwrap()])
            .status();

        // Remove the build and build directories
        let build = Command::new("rm")
            .args(["-rf", build_path.to_str().unwrap()])
            .status();

        // Clean the bootloader directory
        let bootloader = Command::new("cargo")
            .current_dir(bootloader_path)
            .arg("clean")
            .status();

        println!("Cleaned directories:");
        println!("\t{build_path:?} = {build:?}");
        println!("\t{bootloader_path:?} = {bootloader:?}");
        println!("\t{netboot_path:?} = {netboot:?}");
        return Ok(());
    }

    // Create the needed directories.
    // Directories not created here should already exist by the time this script
    // is run.
    create_dir_all(netboot_path.clone()).unwrap();
    create_dir_all(build_path.clone()).unwrap();

    // Get the path to the realmode.asm assembly and the assembled binary
    let realmode_bin  = build_path.clone().join("realmode");
    let realmode_path = bootloader_path.join("src").join("realmode.asm");

    // Convert the paths to strings
    let realmode_bin = realmode_bin.to_str().unwrap();
    let realmode_path = realmode_path.to_str().unwrap();

    // Assemble the realmode
    Command::new("nasm")
        .args(["-f", "elf32",
              &format!("-Dorigin=0x{STAGE0_ORIGIN:x}"),
              "-o", realmode_bin, realmode_path])
        .status()?;

    // Build the bootloader
    let target = "i586-unknown-linux-gnu";
    Command::new("cargo")
        .current_dir(bootloader_path)
        .args(["build", "--release"])
        .status()?;

    // Flatten the bootloader
    let bootloader_bin = bootloader_path
        .join("target")
        .join(target)
        .join("release")
        .join("bootloader");
    let (flat_entry, flat_base, flat_bytes) = flatten_elf(bootloader_bin)
        .expect("Couldn't flatten the bootloader image.");

    // Print some info about the flattened bootloader
    println!("Flattened Bootloader Image:");
    println!("    Entry Point:      0x{flat_entry:x}");
    println!("    Base Address:     0x{flat_base:x}");
    println!("    Flat Image Size:  0x{:x} ({})",
             flat_bytes.len(), flat_bytes.len());

    // Write the bootloader to the build directory
    std::fs::write(build_path.join("bootloader"), &flat_bytes)?;

    // Get the path to the stage0 assembly and the assembled binary
    let stage0_bin  = build_path.clone().join("stage0");
    let stage0_path = bootloader_path.join("src").join("stage0.asm");

    // Convert the paths to strings
    let stage0_bin = stage0_bin.to_str().unwrap();
    let stage0_path = stage0_path.to_str().unwrap();

    // Assemble the stage0
    Command::new("nasm")
        .args(["-f", "bin",
              &format!("-Dentry_point=0x{flat_entry:x}"),
              &format!("-Dbase_address=0x{flat_base:x}"),
              &format!("-Dorigin=0x{STAGE0_ORIGIN:x}"),
              "-o", stage0_bin, stage0_path])
        .status()?;

    // Print some info about the whole bootloader binary.
    let bloader_meta = std::fs::metadata(stage0_bin)?;
    println!("    Whole Image Size: 0x{:x} ({})",
             bloader_meta.len(), bloader_meta.len());

    // Don't go over the PXE limit
    let used_space = (MAX_BOOTLOADER_SIZE as f64) / (bloader_meta.len() as f64);
    let used_space = 100. / used_space;
    println!("    Used PXE space:   {:0.2}%", used_space);
    if bloader_meta.len() >= MAX_BOOTLOADER_SIZE {
        println!("Maximum bootloader size exceeded! Aborting!");
        std::process::exit(1);
    }

    // Copy it to the netboot directory
    std::fs::copy(stage0_bin, netboot_path.join("bootloader.0"))?;

    // Create the path to the kernel output directories
    let kernel_bin = kernel_path.join("target")
        .join("x86_64-unknown-linux-gnu")
        .join("release")
        .join("kernel");

    // Build the kernel
    Command::new("cargo")
        .current_dir("kernel")
        .args(["build", "--release"])
        .status()?;

    // Copy the kernel to the netboot directory
    std::fs::copy(kernel_bin, netboot_path.join("kernel"))?;

    Ok(())
}
