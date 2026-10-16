//! A simple command line utility to update the firmware on a [Glove80](https://www.moergo.com/) device.
//!
//! # Examples
//!
//! Run the firmware updater with default values:
//! ```bash
//! glove80-firmware-updater -f firmware.uf2
//! ```
//! Run the firmware updater with a full path to the firmware file:
//! ```bash
//! glove80-firmware-updater -f /home/user/firmware.uf2
//! ```
//!
//! Run the firmware updater with non-default values:
//! ```bash
//! glove80-firmware-updater -f firmware.uf2 -l GLV80LHBOOT -r GLV80RHBOOT
//! ```
//!

use clap::Parser;
use std::path::PathBuf;
use std::process::exit;
use std::time::Duration;
use std::{fs, thread};
use sysinfo::Disks;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'f', long)]
    file: String,

    #[arg(short = 'l', long, default_value = "GLV80LHBOOT")]
    left_hand_destination: String,

    #[arg(short = 'r', long, default_value = "GLV80RHBOOT")]
    right_hand_destination: String,

    #[arg(long)]
    dry_run: bool,

    /// Print every disk seen and which destination each match satisfied
    #[arg(long)]
    debug: bool,
}

const DEFAULT_SLEEP_DURATION: Duration = Duration::from_secs(5);

fn main() {
    let args = Args::parse();

    // check that the firmware file exists, either relative to the working directory or as a full path
    let firmware_file = PathBuf::from(&args.file);
    if !firmware_file.exists() {
        println!("Error: {} does not exist!", firmware_file.to_str().unwrap());
        exit(1);
    }

    // get the filename from the path
    let firmware_filename = firmware_file
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let mut remaining_destinations =
        vec![&args.left_hand_destination, &args.right_hand_destination];

    let mut disks = Disks::new_with_refreshed_list();

    println!("Ready to copy firmware {:?}", firmware_file);
    while !remaining_destinations.is_empty() {
        disks.refresh_list();
        for disk in &disks {
            if args.debug {
                println!("Disk: {:?}", disk);
            }
            // Glove80 will always be a removable disk
            if !disk.is_removable() {
                continue;
            }
            let mount_point = disk.mount_point();

            // macOS and Windows report the volume label as the disk name, but Linux reports
            // the device path (e.g. /dev/sdb) and the label only appears as the mount
            // directory (e.g. /run/media/<user>/GLV80LHBOOT), so accept either.
            // Each disk is a single device, so it satisfies at most one destination.
            let Some(index) = remaining_destinations.iter().position(|d| {
                disk.name() == d.as_str()
                    || mount_point.file_name().is_some_and(|f| f == d.as_str())
            }) else {
                continue;
            };
            let destination = remaining_destinations.remove(index);

            if args.debug {
                println!("Match {}", destination);
            }
            let dest_path = mount_point.join(&firmware_filename);
            println!("Copying firmware to {}", dest_path.display());

            if !args.dry_run {
                fs::copy(&firmware_file, &dest_path).unwrap();
                println!(
                    "Firmware copied to device {:?} at {:?}!",
                    disk.name(),
                    dest_path
                );
            }

            if remaining_destinations.is_empty() {
                println!("Firmware update complete!");
                exit(0);
            }
        }
        thread::sleep(DEFAULT_SLEEP_DURATION);
    }
}
