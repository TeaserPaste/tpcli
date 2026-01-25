use std::env;
use std::fs;
use std::process::Command;

use anyhow::{Context, Result, anyhow};
use colored::Colorize;

pub fn handle_upgrade() -> Result<()> {
    // 1. Identify the current running executable path
    let current_exe = env::current_exe().context("Failed to get current executable path")?;

    // 2. Define a backup path (e.g., from 'tpc.exe' to 'tpc.old')
    // Windows allows renaming a running executable, but not deleting it.
    // By renaming it, we free up the original name for the new binary.
    let mut backup_exe = current_exe.clone();
    backup_exe.set_extension("old");

    // 3. Clean up any existing backup file from a previous update
    if backup_exe.exists()
        && let Err(e) = fs::remove_file(&backup_exe) {
            // It's possible the old backup is still locked or in use, but we warn and proceed
            eprintln!(
                "{} Failed to remove old backup file: {}",
                "Warning:".yellow(),
                e
            );
        }

    println!("Preparing to upgrade...");

    // 4. Rename the CURRENT running executable to the backup path
    if let Err(e) = fs::rename(&current_exe, &backup_exe) {
        return Err(anyhow!(
            "Failed to rename current executable (locked?): {}",
            e
        ));
    }

    // 5. Execute the external updater command
    // 'tpc-update' will download the new binary and write it to 'current_exe' path
    // because that path is now empty (we just renamed the file occupying it).
    let status_result = Command::new("tpc-update")
        .status()
        .context("Failed to execute 'tpc-update'. Is it installed?");

    match status_result {
        Ok(status) if status.success() => {
            println!(
                "\n{}\n",
                "✅ Upgrade successful! Please restart your terminal.".green()
            );

            // Optional: Try to delete the backup file now.
            // On Windows, this might fail if the process is still attached to the file handle,
            // so we ignore the error. It will be cleaned up on the next run (Step 3).
            let _ = fs::remove_file(&backup_exe);
        }
        _ => {
            // 6. ROLLBACK STRATEGY
            // If the update failed (non-zero exit code) or the command couldn't run:
            println!(
                "\n{}",
                "❌ Upgrade failed. Rolling back to previous version...".red()
            );

            // Check if the updater left a half-written or broken 'tpc.exe' and remove it
            if current_exe.exists()
                && let Err(e) = fs::remove_file(&current_exe) {
                    eprintln!("Rollback Error: Could not remove broken update file: {}", e);
                }

            // Restore the backup file to the original name
            if let Err(e) = fs::rename(&backup_exe, &current_exe) {
                // This is a critical state: neither the new nor old file is in place.
                eprintln!("\n{}", "CRITICAL: Failed to restore backup!".on_red());
                eprintln!(
                    "Your original executable is located at: {}",
                    backup_exe.display()
                );
                eprintln!("Error details: {}", e);
                return Err(anyhow!(
                    "Critical rollback failure. Manual intervention required."
                ));
            } else {
                println!(
                    "{}",
                    "Rollback successful. Your installation is intact.".yellow()
                );
            }

            // Return the original error to the caller
            if let Err(e) = status_result {
                return Err(e);
            } else {
                return Err(anyhow!("Updater exited with error code"));
            }
        }
    }

    Ok(())
}
