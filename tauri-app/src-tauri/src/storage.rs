use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn atomic_write_string(path: &Path, contents: &str) -> Result<(), String> {
    atomic_write(path, contents.as_bytes())
}

pub fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "failed to create atomic write directory {}: {error}",
            parent.display()
        )
    })?;

    let temp_path = temporary_path(path, parent)?;
    let result = (|| {
        let mut temp_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .map_err(|error| {
                format!(
                    "failed to create temporary file {}: {error}",
                    temp_path.display()
                )
            })?;
        temp_file.write_all(contents).map_err(|error| {
            format!(
                "failed to write temporary file {}: {error}",
                temp_path.display()
            )
        })?;
        temp_file.sync_all().map_err(|error| {
            format!(
                "failed to flush temporary file {}: {error}",
                temp_path.display()
            )
        })?;
        drop(temp_file);
        replace_file(&temp_path, path)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn temporary_path(path: &Path, parent: &Path) -> Result<PathBuf, String> {
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("atomic write target has no file name: {}", path.display()))?;
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(format!(".{}.{}.tmp", std::process::id(), sequence));
    Ok(parent.join(temp_name))
}

#[cfg(windows)]
fn replace_file(temp_path: &Path, target_path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{ReplaceFileW, REPLACEFILE_WRITE_THROUGH};

    if !target_path.exists() {
        return fs::rename(temp_path, target_path).map_err(|error| {
            format!(
                "failed to move {} to {}: {error}",
                temp_path.display(),
                target_path.display()
            )
        });
    }

    let target_wide: Vec<u16> = target_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let temp_wide: Vec<u16> = temp_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let replaced = unsafe {
        ReplaceFileW(
            target_wide.as_ptr(),
            temp_wide.as_ptr(),
            std::ptr::null(),
            REPLACEFILE_WRITE_THROUGH,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if replaced == 0 {
        return Err(format!(
            "failed to atomically replace {}: {}",
            target_path.display(),
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_file(temp_path: &Path, target_path: &Path) -> Result<(), String> {
    fs::rename(temp_path, target_path).map_err(|error| {
        format!(
            "failed to atomically replace {} with {}: {error}",
            target_path.display(),
            temp_path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::atomic_write_string;
    use std::fs;

    #[test]
    fn replaces_existing_content() {
        let directory = std::env::temp_dir().join(format!(
            "floating-input-tool-storage-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("value.txt");
        fs::write(&path, "before").unwrap();

        atomic_write_string(&path, "after").unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "after");
        fs::remove_dir_all(directory).unwrap();
    }
}
