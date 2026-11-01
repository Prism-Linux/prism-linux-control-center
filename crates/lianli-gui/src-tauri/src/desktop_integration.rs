use anyhow::{ensure, Context, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

const AUTOSTART_FILE: &str = "com.sgtaziz.lianlilinux.desktop";
const AUTOSTART_MARKER: &str = "X-Lian-Li-Linux-Managed=true";

pub struct GuiInstanceLock {
    _file: std::fs::File,
}

impl GuiInstanceLock {
    pub fn acquire() -> Result<Option<Self>> {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let path = runtime.join(format!("lianli-gui-{}.lock", unsafe { libc::getuid() }));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o600)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)
            .with_context(|| format!("opening GUI instance lock {}", path.display()))?;
        ensure!(
            file.metadata()?.is_file(),
            "GUI instance lock is not a regular file"
        );
        let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if result == 0 {
            Ok(Some(Self { _file: file }))
        } else {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::EWOULDBLOCK) {
                Ok(None)
            } else {
                Err(error).context("locking GUI instance")
            }
        }
    }
}

pub fn sync_autostart(enabled: bool, launch_minimized: bool) -> Result<()> {
    let config_home = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(
            std::env::var_os("HOME").context("HOME is unavailable for desktop autostart")?,
        )
        .join(".config"),
    };
    ensure!(
        config_home.is_absolute(),
        "XDG_CONFIG_HOME must be an absolute path"
    );
    sync_autostart_in(
        &config_home,
        &std::env::current_exe()?,
        enabled,
        launch_minimized,
    )
}

fn sync_autostart_in(
    config_home: &Path,
    executable: &Path,
    enabled: bool,
    launch_minimized: bool,
) -> Result<()> {
    let directory = config_home.join("autostart");
    if !enabled && !directory.exists() {
        return Ok(());
    }
    fs::create_dir_all(&directory).with_context(|| format!("creating {}", directory.display()))?;
    let directory_meta = fs::symlink_metadata(&directory)?;
    ensure!(
        directory_meta.is_dir() && !directory_meta.file_type().is_symlink(),
        "autostart path is not a real directory"
    );

    let entry = directory.join(AUTOSTART_FILE);
    if !enabled {
        match fs::symlink_metadata(&entry) {
            Ok(metadata) => {
                ensure!(
                    metadata.is_file() && !metadata.file_type().is_symlink(),
                    "refusing to remove a non-regular autostart entry"
                );
                let contents = fs::read_to_string(&entry)?;
                if contents.contains(AUTOSTART_MARKER) {
                    fs::remove_file(entry)?;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        return Ok(());
    }

    match fs::symlink_metadata(&entry) {
        Ok(metadata) => {
            ensure!(
                metadata.is_file() && !metadata.file_type().is_symlink(),
                "refusing to replace a non-regular autostart entry"
            );
            let contents = fs::read_to_string(&entry)?;
            ensure!(
                contents.contains(AUTOSTART_MARKER),
                "an unmanaged desktop autostart entry already exists"
            );
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    let executable = desktop_exec_arg(executable);
    let minimized = if launch_minimized { " --minimized" } else { "" };
    let contents = format!(
        "[Desktop Entry]\nType=Application\nName=Lian Li Linux\nComment=Hardware lighting and cooling dashboard\nExec={executable}{minimized}\nIcon=com.sgtaziz.lianlilinux\nTerminal=false\nStartupNotify=false\nX-GNOME-Autostart-enabled=true\n{AUTOSTART_MARKER}\n"
    );
    let temp = directory.join(format!(".{AUTOSTART_FILE}.{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o644)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(&temp)
        .with_context(|| format!("creating temporary autostart file {}", temp.display()))?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    fs::set_permissions(&temp, fs::Permissions::from_mode(0o644))?;
    fs::rename(&temp, &entry)
        .with_context(|| format!("publishing autostart file {}", entry.display()))?;
    Ok(())
}

fn desktop_exec_arg(path: &Path) -> String {
    format!(
        "\"{}\"",
        path.to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autostart_entry_reflects_minimized_setting_and_removes_only_its_own_file() {
        let root = tempfile::tempdir().unwrap();
        let executable = Path::new("/opt/Lian Li/lianli-gui");
        sync_autostart_in(root.path(), executable, true, true).unwrap();
        let entry = root.path().join("autostart").join(AUTOSTART_FILE);
        let contents = fs::read_to_string(&entry).unwrap();
        assert!(contents.contains("Exec=\"/opt/Lian Li/lianli-gui\" --minimized"));
        assert!(contents.contains(AUTOSTART_MARKER));

        sync_autostart_in(root.path(), executable, true, false).unwrap();
        assert!(!fs::read_to_string(&entry).unwrap().contains("--minimized"));
        sync_autostart_in(root.path(), executable, false, false).unwrap();
        assert!(!entry.exists());
    }

    #[test]
    fn autostart_sync_preserves_unmanaged_entries() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("autostart");
        fs::create_dir_all(&directory).unwrap();
        let entry = directory.join(AUTOSTART_FILE);
        fs::write(&entry, "[Desktop Entry]\nName=Personal launcher\n").unwrap();
        assert!(
            sync_autostart_in(root.path(), Path::new("/usr/bin/lianli-gui"), true, false).is_err()
        );
        assert_eq!(
            fs::read_to_string(entry).unwrap(),
            "[Desktop Entry]\nName=Personal launcher\n"
        );
    }
}
