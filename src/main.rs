//! Buildscript for the bootloader and kernel.
//!
//! Requirements:
//!     nasm
//!     ld.lld

use std::path::Path;
use std::process::Command;
use std::fs::create_dir_all;
use std::env::args;

use elf_parser::ElfParser;

/// Maximum stage0/bootloader size allowed by PXE
const MAX_BOOTLOADER_SIZE: usize = 32 * 1024;

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

fn main() {
    // Get the paths to our working directories
    let netboot_path    = Path::new("qemu").join("netboot");
    let build_path      = Path::new("build");
    let bootloader_path = Path::new("bootloader");

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
        return;
    }

    // Create the needed directories.
    // Directories not created here should already exist by the time this script
    // is run.
    create_dir_all(netboot_path.clone()).unwrap();
    create_dir_all(build_path.clone()).unwrap();

    // Build the bootloader
    let target = "i586-unknown-linux-gnu";
    Command::new("cargo")
        .current_dir(bootloader_path)
        .args(["build", "--release"])
        .status()
        .expect("Couldn't build the bootloader.");

    // Flatten the bootloader
    let bootloader_bin = bootloader_path
        .join("target")
        .join(target)
        .join("release")
        .join("bootloader");
    let (flat_entry, flat_base, flat_bytes) = flatten_elf(bootloader_bin)
        .expect("Couldn't flatten the bootloader image.");

    // Print some info about the bootloader
    println!("Flattened Bootloader Image:");
    println!("\tEntry Point:  0x{flat_entry:x}");
    println!("\tBase Address: 0x{flat_base:x}");
    println!("\tImage Length: 0x{:x} ({})", flat_bytes.len(), flat_bytes.len());

    // Do not go over the PXE limit
    let used_space = (MAX_BOOTLOADER_SIZE as f64) / (flat_bytes.len() as f64);
    let used_space = 100. / used_space;
    println!("\tUsed Space:   {:0.2}%", used_space);
    if flat_bytes.len() >= MAX_BOOTLOADER_SIZE {
        println!("Maximum bootloader size exceeded! Aborting!");
        std::process::exit(1);
    }

    // Write the bootloader to the build directory
    std::fs::write(build_path.join("bootloader"), flat_bytes)
        .expect("Couldn't write the flat bootloader image to the file system.");

    // Get the path to the stage0 assembly and the assembled binary
    let stage0_bin  = build_path.clone().join("stage0");
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
}
