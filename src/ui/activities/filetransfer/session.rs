//! ## FileTransferActivity
//!
//! `filetransfer_activiy` is the module which implements the Filetransfer activity, which is the main activity afterall

/**
 * MIT License
 *
 * termscp - Copyright (c) 2021 Christian Visintin
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
// Deps
extern crate bytesize;
extern crate content_inspector;
extern crate crossterm;
extern crate tempfile;

// Locals
use super::{FileTransferActivity, LogLevel};
use crate::filetransfer::FileTransferError;
use crate::fs::{FsEntry, FsFile};
use crate::host::HostError;
use crate::utils::fmt::fmt_millis;

// Ext
use bytesize::ByteSize;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::fs::OpenOptions;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};
use thiserror::Error;

/// ## TransferErrorReason
///
/// Describes the reason that caused an error during a file transfer
#[derive(Error, Debug)]
enum TransferErrorReason {
    #[error("File transfer aborted")]
    Abrupted,
    #[error("Failed to seek file: {0}")]
    CouldNotRewind(std::io::Error),
    #[error("I/O error on localhost: {0}")]
    LocalIoError(std::io::Error),
    #[error("Host error: {0}")]
    HostError(HostError),
    #[error("I/O error on remote: {0}")]
    RemoteIoError(std::io::Error),
    #[error("File transfer error: {0}")]
    FileTransferError(FileTransferError),
}

impl FileTransferActivity {
    /// ### connect
    ///
    /// Connect to remote
    pub(super) fn connect(&mut self) {
        let params = self.context.as_ref().unwrap().ft_params.as_ref().unwrap();
        let addr: String = params.address.clone();
        let entry_dir: Option<PathBuf> = params.entry_directory.clone();
        // Connect to remote
        match self.client.connect(
            params.address.clone(),
            params.port,
            params.username.clone(),
            params.password.clone(),
        ) {
            Ok(welcome) => {
                if let Some(banner) = welcome {
                    // Log welcome
                    self.log(
                        LogLevel::Info,
                        format!("Established connection with '{}': \"{}\"", addr, banner),
                    );
                }
                // Try to change directory to entry directory
                let mut remote_chdir: Option<PathBuf> = None;
                if let Some(entry_directory) = &entry_dir {
                    remote_chdir = Some(entry_directory.clone());
                }
                if let Some(entry_directory) = remote_chdir {
                    self.remote_changedir(entry_directory.as_path(), false);
                }
                // Set state to explorer
                self.umount_wait();
                self.reload_remote_dir();
                // Update file lists
                self.update_local_filelist();
                self.update_remote_filelist();
            }
            Err(err) => {
                // Set popup fatal error
                self.mount_fatal(&err.to_string());
            }
        }
    }

    /// ### disconnect
    ///
    /// disconnect from remote
    pub(super) fn disconnect(&mut self) {
        let params = self.context.as_ref().unwrap().ft_params.as_ref().unwrap();
        let msg: String = format!("Disconnecting from {}...", params.address);
        // Show popup disconnecting
        self.mount_wait(msg.as_str());
        // Disconnect
        let _ = self.client.disconnect();
        // Quit
        self.exit_reason = Some(super::ExitReason::Disconnect);
    }

    /// ### disconnect_and_quit
    ///
    /// disconnect from remote and then quit
    pub(super) fn disconnect_and_quit(&mut self) {
        self.disconnect();
        self.exit_reason = Some(super::ExitReason::Quit);
    }

    /// ### reload_remote_dir
    ///
    /// Reload remote directory entries
    pub(super) fn reload_remote_dir(&mut self) {
        // Get current entries
        if let Ok(pwd) = self.client.pwd() {
            self.remote_scan(pwd.as_path());
            // Set wrkdir
            self.remote_mut().wrkdir = pwd;
        }
    }

    pub(super) fn reload_local_dir(&mut self) {
        let wrkdir: PathBuf = self.local().wrkdir.clone();
        self.local_scan(wrkdir.as_path());
    }

    /// ### filetransfer_send
    ///
    /// Send fs entry to remote.
    /// If dst_name is Some, entry will be saved with a different name.
    /// If entry is a directory, this applies to directory only
    pub(super) fn filetransfer_send(
        &mut self,
        entry: &FsEntry,
        curr_remote_path: &Path,
        dst_name: Option<String>,
    ) {
        // Write popup
        let file_name: String = match entry {
            FsEntry::Directory(dir) => dir.name.clone(),
            FsEntry::File(file) => file.name.clone(),
        };
        // Get remote path
        let mut remote_path: PathBuf = PathBuf::from(curr_remote_path);
        let remote_file_name: PathBuf = match dst_name {
            Some(s) => PathBuf::from(s.as_str()),
            None => PathBuf::from(file_name.as_str()),
        };
        remote_path.push(remote_file_name);
        // Match entry
        match entry {
            FsEntry::File(file) => {
                if let Err(err) =
                    self.filetransfer_send_file(file, remote_path.as_path(), file_name)
                {
                    // Log error
                    self.log_and_alert(
                        LogLevel::Error,
                        format!("Failed to upload file {}: {}", file.name, err),
                    );
                    // If transfer was abrupted or there was an IO error on remote, remove file
                    if matches!(
                        err,
                        TransferErrorReason::Abrupted | TransferErrorReason::RemoteIoError(_)
                    ) {
                        // Stat file on remote and remove it if exists
                        match self.client.stat(remote_path.as_path()) {
                            Err(err) => self.log(
                                LogLevel::Error,
                                format!(
                                    "Could not remove created file {}: {}",
                                    remote_path.display(),
                                    err
                                ),
                            ),
                            Ok(entry) => {
                                if let Err(err) = self.client.remove(&entry) {
                                    self.log(
                                        LogLevel::Error,
                                        format!(
                                            "Could not remove created file {}: {}",
                                            remote_path.display(),
                                            err
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            FsEntry::Directory(dir) => {
                // Create directory on remote
                match self.client.mkdir(remote_path.as_path()) {
                    Ok(_) => {
                        self.log(
                            LogLevel::Info,
                            format!("Created directory \"{}\"", remote_path.display()),
                        );
                        // Get files in dir
                        match self.host.scan_dir(dir.abs_path.as_path()) {
                            Ok(entries) => {
                                // Iterate over files
                                for entry in entries.iter() {
                                    // If aborted; break
                                    if self.transfer.aborted {
                                        break;
                                    }
                                    // Send entry; name is always None after first call
                                    self.filetransfer_send(&entry, remote_path.as_path(), None);
                                }
                            }
                            Err(err) => {
                                self.log_and_alert(
                                    LogLevel::Error,
                                    format!(
                                        "Could not scan directory \"{}\": {}",
                                        dir.abs_path.display(),
                                        err
                                    ),
                                );
                            }
                        }
                    }
                    Err(err) => {
                        self.log_and_alert(
                            LogLevel::Error,
                            format!(
                                "Failed to create directory \"{}\": {}",
                                remote_path.display(),
                                err
                            ),
                        );
                    }
                }
            }
        }
        // Scan dir on remote
        self.reload_remote_dir();
        // If aborted; show popup
        if self.transfer.aborted {
            // Log abort
            self.log_and_alert(
                LogLevel::Warn,
                format!("Upload aborted for \"{}\"!", entry.get_abs_path().display()),
            );
            // Set aborted to false
            self.transfer.aborted = false;
        } else {
            // @! Successful
            // Eventually, Remove progress bar
            self.umount_progress_bar();
        }
    }

    /// ### filetransfer_recv
    ///
    /// Recv fs entry from remote.
    /// If dst_name is Some, entry will be saved with a different name.
    /// If entry is a directory, this applies to directory only
    pub(super) fn filetransfer_recv(
        &mut self,
        entry: &FsEntry,
        local_path: &Path,
        dst_name: Option<String>,
    ) {
        // Write popup
        let file_name: String = match entry {
            FsEntry::Directory(dir) => dir.name.clone(),
            FsEntry::File(file) => file.name.clone(),
        };
        // Match entry
        match entry {
            FsEntry::File(file) => {
                // Get local file
                let mut local_file_path: PathBuf = PathBuf::from(local_path);
                let local_file_name: String = match dst_name {
                    Some(n) => n,
                    None => file.name.clone(),
                };
                local_file_path.push(local_file_name.as_str());
                // Download file
                if let Err(err) =
                    self.filetransfer_recv_file(local_file_path.as_path(), file, file_name)
                {
                    self.log_and_alert(
                        LogLevel::Error,
                        format!("Could not download file {}: {}", file.name, err),
                    );
                    // If transfer was abrupted or there was an IO error on remote, remove file
                    if matches!(
                        err,
                        TransferErrorReason::Abrupted | TransferErrorReason::LocalIoError(_)
                    ) {
                        // Stat file
                        match self.host.stat(local_file_path.as_path()) {
                            Err(err) => self.log(
                                LogLevel::Error,
                                format!(
                                    "Could not remove created file {}: {}",
                                    local_file_path.display(),
                                    err
                                ),
                            ),
                            Ok(entry) => {
                                if let Err(err) = self.host.remove(&entry) {
                                    self.log(
                                        LogLevel::Error,
                                        format!(
                                            "Could not remove created file {}: {}",
                                            local_file_path.display(),
                                            err
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            FsEntry::Directory(dir) => {
                // Get dir name
                let mut local_dir_path: PathBuf = PathBuf::from(local_path);
                match dst_name {
                    Some(name) => local_dir_path.push(name),
                    None => local_dir_path.push(dir.name.as_str()),
                }
                // Create directory on local
                match self.host.mkdir_ex(local_dir_path.as_path(), true) {
                    Ok(_) => {
                        // Apply file mode to directory
                        #[cfg(any(target_os = "unix", target_os = "macos", target_os = "linux"))]
                        if let Some(pex) = dir.unix_pex {
                            if let Err(err) = self.host.chmod(local_dir_path.as_path(), pex) {
                                self.log(
                                    LogLevel::Error,
                                    format!(
                                        "Could not apply file mode {:?} to \"{}\": {}",
                                        pex,
                                        local_dir_path.display(),
                                        err
                                    ),
                                );
                            }
                        }
                        self.log(
                            LogLevel::Info,
                            format!("Created directory \"{}\"", local_dir_path.display()),
                        );
                        // Get files in dir
                        match self.client.list_dir(dir.abs_path.as_path()) {
                            Ok(entries) => {
                                // Iterate over files
                                for entry in entries.iter() {
                                    // If transfer has been aborted; break
                                    if self.transfer.aborted {
                                        break;
                                    }
                                    // Receive entry; name is always None after first call
                                    // Local path becomes local_dir_path
                                    self.filetransfer_recv(&entry, local_dir_path.as_path(), None);
                                }
                            }
                            Err(err) => {
                                self.log_and_alert(
                                    LogLevel::Error,
                                    format!(
                                        "Could not scan directory \"{}\": {}",
                                        dir.abs_path.display(),
                                        err
                                    ),
                                );
                            }
                        }
                    }
                    Err(err) => {
                        self.log(
                            LogLevel::Error,
                            format!(
                                "Failed to create directory \"{}\": {}",
                                local_dir_path.display(),
                                err
                            ),
                        );
                    }
                }
            }
        }
        // Reload directory on local
        self.local_scan(local_path);
        // if aborted; show alert
        if self.transfer.aborted {
            // Log abort
            self.log_and_alert(
                LogLevel::Warn,
                format!(
                    "Download aborted for \"{}\"!",
                    entry.get_abs_path().display()
                ),
            );
            // Reset aborted to false
            self.transfer.aborted = false;
        } else {
            // Eventually, Reset input mode to explorer
            self.umount_progress_bar();
        }
    }

    /// ### filetransfer_send_file
    ///
    /// Send local file and write it to remote path
    fn filetransfer_send_file(
        &mut self,
        local: &FsFile,
        remote: &Path,
        file_name: String,
    ) -> Result<(), TransferErrorReason> {
        // Upload file
        // Try to open local file
        match self.host.open_file_read(local.abs_path.as_path()) {
            Ok(mut fhnd) => match self.client.send_file(local, remote) {
                Ok(mut rhnd) => {
                    // Write file
                    let file_size: usize =
                        fhnd.seek(std::io::SeekFrom::End(0)).unwrap_or(0) as usize;
                    // rewind
                    if let Err(err) = fhnd.seek(std::io::SeekFrom::Start(0)) {
                        return Err(TransferErrorReason::CouldNotRewind(err));
                    }
                    // Write remote file
                    let mut total_bytes_written: usize = 0;
                    // Reset transfer states
                    self.transfer.reset();
                    let mut last_progress_val: f64 = 0.0;
                    let mut last_input_event_fetch: Instant = Instant::now();
                    // Mount progress bar
                    self.mount_progress_bar();
                    // While the entire file hasn't been completely written,
                    // Or filetransfer has been aborted
                    while total_bytes_written < file_size && !self.transfer.aborted {
                        // Handle input events (each 500ms)
                        if last_input_event_fetch.elapsed().as_millis() >= 500 {
                            // Read events
                            self.read_input_event();
                            // Reset instant
                            last_input_event_fetch = Instant::now();
                        }
                        // Read till you can
                        let mut buffer: [u8; 65536] = [0; 65536];
                        match fhnd.read(&mut buffer) {
                            Ok(bytes_read) => {
                                total_bytes_written += bytes_read;
                                if bytes_read == 0 {
                                    continue;
                                } else {
                                    let mut buf_start: usize = 0;
                                    while buf_start < bytes_read {
                                        // Write bytes
                                        match rhnd.write(&buffer[buf_start..bytes_read]) {
                                            Ok(bytes) => {
                                                buf_start += bytes;
                                            }
                                            Err(err) => {
                                                self.umount_progress_bar();
                                                return Err(TransferErrorReason::RemoteIoError(
                                                    err,
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                self.umount_progress_bar();
                                return Err(TransferErrorReason::LocalIoError(err));
                            }
                        }
                        // Increase progress
                        self.transfer.set_progress(total_bytes_written, file_size);
                        // Draw only if a significant progress has been made (performance improvement)
                        if last_progress_val < self.transfer.progress - 1.0 {
                            // Draw
                            self.update_progress_bar(format!("Uploading \"{}\"...", file_name));
                            self.view();
                            last_progress_val = self.transfer.progress;
                        }
                    }
                    // Umount progress bar
                    self.umount_progress_bar();
                    // Finalize stream
                    if let Err(err) = self.client.on_sent(rhnd) {
                        self.log(
                            LogLevel::Warn,
                            format!("Could not finalize remote stream: \"{}\"", err),
                        );
                    }
                    // if upload was abrupted, return error
                    if self.transfer.aborted {
                        return Err(TransferErrorReason::Abrupted);
                    }
                    self.log(
                        LogLevel::Info,
                        format!(
                            "Saved file \"{}\" to \"{}\" (took {} seconds; at {}/s)",
                            local.abs_path.display(),
                            remote.display(),
                            fmt_millis(self.transfer.started.elapsed()),
                            ByteSize(self.transfer.bytes_per_second()),
                        ),
                    );
                }
                Err(err) => return Err(TransferErrorReason::FileTransferError(err)),
            },
            Err(err) => return Err(TransferErrorReason::HostError(err)),
        }
        Ok(())
    }

    /// ### filetransfer_recv_file
    ///
    /// Receive file from remote and write it to local path
    fn filetransfer_recv_file(
        &mut self,
        local: &Path,
        remote: &FsFile,
        file_name: String,
    ) -> Result<(), TransferErrorReason> {
        // Try to open local file
        match self.host.open_file_write(local) {
            Ok(mut local_file) => {
                // Download file from remote
                match self.client.recv_file(remote) {
                    Ok(mut rhnd) => {
                        let mut total_bytes_written: usize = 0;
                        // Reset transfer states
                        self.transfer.reset();
                        // Write local file
                        let mut last_progress_val: f64 = 0.0;
                        let mut last_input_event_fetch: Instant = Instant::now();
                        // Mount progress bar
                        self.mount_progress_bar();
                        // While the entire file hasn't been completely read,
                        // Or filetransfer has been aborted
                        while total_bytes_written < remote.size && !self.transfer.aborted {
                            // Handle input events (each 500 ms)
                            if last_input_event_fetch.elapsed().as_millis() >= 500 {
                                // Read events
                                self.read_input_event();
                                // Reset instant
                                last_input_event_fetch = Instant::now();
                            }
                            // Read till you can
                            let mut buffer: [u8; 65536] = [0; 65536];
                            match rhnd.read(&mut buffer) {
                                Ok(bytes_read) => {
                                    total_bytes_written += bytes_read;
                                    if bytes_read == 0 {
                                        continue;
                                    } else {
                                        let mut buf_start: usize = 0;
                                        while buf_start < bytes_read {
                                            // Write bytes
                                            match local_file.write(&buffer[buf_start..bytes_read]) {
                                                Ok(bytes) => buf_start += bytes,
                                                Err(err) => {
                                                    self.umount_progress_bar();
                                                    return Err(TransferErrorReason::LocalIoError(
                                                        err,
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(err) => {
                                    self.umount_progress_bar();
                                    return Err(TransferErrorReason::RemoteIoError(err));
                                }
                            }
                            // Set progress
                            self.transfer.set_progress(total_bytes_written, remote.size);
                            // Draw only if a significant progress has been made (performance improvement)
                            if last_progress_val < self.transfer.progress - 1.0 {
                                // Draw
                                self.update_progress_bar(format!("Downloading \"{}\"", file_name));
                                self.view();
                                last_progress_val = self.transfer.progress;
                            }
                        }
                        // Umount progress bar
                        self.umount_progress_bar();
                        // Finalize stream
                        if let Err(err) = self.client.on_recv(rhnd) {
                            self.log(
                                LogLevel::Warn,
                                format!("Could not finalize remote stream: \"{}\"", err),
                            );
                        }
                        // If download was abrupted, return Error
                        if self.transfer.aborted {
                            return Err(TransferErrorReason::Abrupted);
                        }
                        // Apply file mode to file
                        #[cfg(any(target_os = "unix", target_os = "macos", target_os = "linux"))]
                        if let Some(pex) = remote.unix_pex {
                            if let Err(err) = self.host.chmod(local, pex) {
                                self.log(
                                    LogLevel::Error,
                                    format!(
                                        "Could not apply file mode {:?} to \"{}\": {}",
                                        pex,
                                        local.display(),
                                        err
                                    ),
                                );
                            }
                        }
                        // Log
                        self.log(
                            LogLevel::Info,
                            format!(
                                "Saved file \"{}\" to \"{}\" (took {} seconds; at {}/s)",
                                remote.abs_path.display(),
                                local.display(),
                                fmt_millis(self.transfer.started.elapsed()),
                                ByteSize(self.transfer.bytes_per_second()),
                            ),
                        );
                    }
                    Err(err) => return Err(TransferErrorReason::FileTransferError(err)),
                }
            }
            Err(err) => return Err(TransferErrorReason::HostError(err)),
        }
        Ok(())
    }

    /// ### local_scan
    ///
    /// Scan current local directory
    pub(super) fn local_scan(&mut self, path: &Path) {
        match self.host.scan_dir(path) {
            Ok(files) => {
                // Set files and sort (sorting is implicit)
                self.local_mut().set_files(files);
            }
            Err(err) => {
                self.log_and_alert(
                    LogLevel::Error,
                    format!("Could not scan current directory: {}", err),
                );
            }
        }
    }

    /// ### remote_scan
    ///
    /// Scan current remote directory
    pub(super) fn remote_scan(&mut self, path: &Path) {
        match self.client.list_dir(path) {
            Ok(files) => {
                // Set files and sort (sorting is implicit)
                self.remote_mut().set_files(files);
            }
            Err(err) => {
                self.log_and_alert(
                    LogLevel::Error,
                    format!("Could not scan current directory: {}", err),
                );
            }
        }
    }

    /// ### local_changedir
    ///
    /// Change directory for local
    pub(super) fn local_changedir(&mut self, path: &Path, push: bool) {
        // Get current directory
        let prev_dir: PathBuf = self.local().wrkdir.clone();
        // Change directory
        match self.host.change_wrkdir(path) {
            Ok(_) => {
                self.log(
                    LogLevel::Info,
                    format!("Changed directory on local: {}", path.display()),
                );
                // Reload files
                self.local_scan(path);
                // Set wrkdir
                self.local_mut().wrkdir = PathBuf::from(path);
                // Push prev_dir to stack
                if push {
                    self.local_mut().pushd(prev_dir.as_path())
                }
            }
            Err(err) => {
                // Report err
                self.log_and_alert(
                    LogLevel::Error,
                    format!("Could not change working directory: {}", err),
                );
            }
        }
    }

    pub(super) fn remote_changedir(&mut self, path: &Path, push: bool) {
        // Get current directory
        let prev_dir: PathBuf = self.remote().wrkdir.clone();
        // Change directory
        match self.client.as_mut().change_dir(path) {
            Ok(_) => {
                self.log(
                    LogLevel::Info,
                    format!("Changed directory on remote: {}", path.display()),
                );
                // Update files
                self.remote_scan(path);
                // Set wrkdir
                self.remote_mut().wrkdir = PathBuf::from(path);
                // Push prev_dir to stack
                if push {
                    self.remote_mut().pushd(prev_dir.as_path())
                }
            }
            Err(err) => {
                // Report err
                self.log_and_alert(
                    LogLevel::Error,
                    format!("Could not change working directory: {}", err),
                );
            }
        }
    }

    /// ### edit_local_file
    ///
    /// Edit a file on localhost
    pub(super) fn edit_local_file(&mut self, path: &Path) -> Result<(), String> {
        // Read first 2048 bytes or less from file to check if it is textual
        match OpenOptions::new().read(true).open(path) {
            Ok(mut f) => {
                // Read
                let mut buff: [u8; 2048] = [0; 2048];
                match f.read(&mut buff) {
                    Ok(size) => {
                        if content_inspector::inspect(&buff[0..size]).is_binary() {
                            return Err("Could not open file in editor: file is binary".to_string());
                        }
                    }
                    Err(err) => {
                        return Err(format!("Could not read file: {}", err));
                    }
                }
            }
            Err(err) => {
                return Err(format!("Could not read file: {}", err));
            }
        }
        // Put input mode back to normal
        let _ = disable_raw_mode();
        // Leave alternate mode
        if let Some(ctx) = self.context.as_mut() {
            ctx.leave_alternate_screen();
        }
        // Open editor
        match edit::edit_file(path) {
            Ok(_) => self.log(
                LogLevel::Info,
                format!(
                    "Changes performed through editor saved to \"{}\"!",
                    path.display()
                ),
            ),
            Err(err) => return Err(format!("Could not open editor: {}", err)),
        }
        if let Some(ctx) = self.context.as_mut() {
            // Clear screen
            ctx.clear_screen();
            // Enter alternate mode
            ctx.enter_alternate_screen();
        }
        // Re-enable raw mode
        let _ = enable_raw_mode();
        Ok(())
    }

    /// ### edit_remote_file
    ///
    /// Edit file on remote host
    pub(super) fn edit_remote_file(&mut self, file: &FsFile) -> Result<(), String> {
        // Create temp file
        let tmpfile: tempfile::NamedTempFile = match tempfile::NamedTempFile::new() {
            Ok(f) => f,
            Err(err) => {
                return Err(format!("Could not create temporary file: {}", err));
            }
        };
        // Download file
        if let Err(err) = self.filetransfer_recv_file(tmpfile.path(), file, file.name.clone()) {
            return Err(format!("Could not open file {}: {}", file.name, err));
        }
        // Get current file modification time
        let prev_mtime: SystemTime = match self.host.stat(tmpfile.path()) {
            Ok(e) => e.get_last_change_time(),
            Err(err) => {
                return Err(format!(
                    "Could not stat \"{}\": {}",
                    tmpfile.path().display(),
                    err
                ))
            }
        };
        // Edit file
        if let Err(err) = self.edit_local_file(tmpfile.path()) {
            return Err(err);
        }
        // Get local fs entry
        let tmpfile_entry: FsEntry = match self.host.stat(tmpfile.path()) {
            Ok(e) => e,
            Err(err) => {
                return Err(format!(
                    "Could not stat \"{}\": {}",
                    tmpfile.path().display(),
                    err
                ))
            }
        };
        // Check if file has changed
        match prev_mtime != tmpfile_entry.get_last_change_time() {
            true => {
                self.log(
                    LogLevel::Info,
                    format!(
                        "File \"{}\" has changed; writing changes to remote",
                        file.abs_path.display()
                    ),
                );
                // Get local fs entry
                let tmpfile_entry: FsEntry = match self.host.stat(tmpfile.path()) {
                    Ok(e) => e,
                    Err(err) => {
                        return Err(format!(
                            "Could not stat \"{}\": {}",
                            tmpfile.path().display(),
                            err
                        ))
                    }
                };
                // Write file
                let tmpfile_entry: &FsFile = match &tmpfile_entry {
                    FsEntry::Directory(_) => panic!("tempfile is a directory for some reason"),
                    FsEntry::File(f) => f,
                };
                // Send file
                if let Err(err) = self.filetransfer_send_file(
                    tmpfile_entry,
                    file.abs_path.as_path(),
                    file.name.clone(),
                ) {
                    return Err(format!(
                        "Could not write file {}: {}",
                        file.abs_path.display(),
                        err
                    ));
                }
            }
            false => {
                self.log(
                    LogLevel::Info,
                    format!("File \"{}\" hasn't changed", file.abs_path.display()),
                );
            }
        }
        Ok(())
    }
}
