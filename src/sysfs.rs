use std::{
    error::Error,
    ffi::CStr,
    fs, io,
    path::{Path, PathBuf},
};

fn fstype(path: &CStr) -> Result<libc::__fsword_t, io::Error> {
    use std::mem::MaybeUninit;

    let sfs = unsafe {
        let mut sfs = MaybeUninit::zeroed();
        let res = libc::statfs(path.as_ptr(), sfs.as_mut_ptr());
        if res < 0 {
            return Err(std::io::Error::last_os_error());
        }
        sfs.assume_init()
    };

    Ok(sfs.f_type)
}

pub(crate) fn find_sysfs() -> Result<PathBuf, Box<dyn Error>> {
    if fstype(c"/sys")? != libc::SYSFS_MAGIC {
        Err("/sys is not sysfs, please mount it with: mount -t sysfs sysfs /sys")?;
    }
    Ok("/sys".into())
}

pub(crate) fn find_bpffs() -> Result<PathBuf, Box<dyn Error>> {
    if fstype(c"/sys/fs/bpf")? != libc::BPF_FS_MAGIC {
        Err("/sys/fs/bpf is not bpffs, please mount it with: mount -t bpf bpffs /sys/fs/bpf")?;
    }
    Ok("/sys/fs/bpf".into())
}

pub(crate) fn find_subsystem(
    sysfs: &Path,
    subsystem: &str,
) -> Result<Option<PathBuf>, Box<dyn Error>> {
    for dir in ["subsytem", "bus", "class", "block"] {
        let path = sysfs.join(dir).join(subsystem);
        if fs::exists(&path)? {
            return Ok(Some(path));
        }
    }

    Ok(None)
}

pub(crate) fn subsystem_devices(subsystem: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let iter = subsystem.join("devices").read_dir()?;
    let mut result = Vec::new();
    for entry in iter {
        result.push(entry?.path());
    }
    Ok(result)
}

fn map_not_found<T>(result: Result<T, io::Error>) -> Result<Option<T>, io::Error> {
    match result {
        Ok(result) => Ok(Some(result)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) if e.kind() == io::ErrorKind::InvalidInput => Ok(None),
        Err(e) => Err(e),
    }
}

pub(crate) fn property(device: &Path, name: &str) -> Result<Option<String>, Box<dyn Error>> {
    Ok(map_not_found(fs::read_to_string(device.join(name)))?)
}

pub(crate) fn property_trim(device: &Path, name: &str) -> Result<Option<String>, Box<dyn Error>> {
    property(device, name).map(|m| m.map(|s| s.trim().to_owned()))
}

pub(crate) fn get_subsystem(device: &Path) -> Result<Option<String>, Box<dyn Error>> {
    let subsystem_link = device.join("subsystem");

    Ok(map_not_found(subsystem_link.read_link())?.map(|p| {
        p.file_name()
            .expect("Unexpected subsystem symlink")
            .to_string_lossy()
            .into_owned()
    }))
}

pub(crate) fn find_map_parent<T>(
    device: &Path,
    mut f: impl FnMut(&Path) -> Result<Option<T>, Box<dyn Error>>,
) -> Result<Option<T>, Box<dyn Error>> {
    for ancestor in device.canonicalize()?.ancestors() {
        if let Some(res) = f(ancestor)? {
            return Ok(Some(res));
        }
    }
    return Ok(None);
}

pub(crate) fn hid_device_id(device: &Path) -> Result<i32, Box<dyn Error>> {
    let device = device.canonicalize()?;
    let base = device.file_name().expect("Unexpected device directory");
    let id = base.to_string_lossy();
    let id = id.split(['.']).last().unwrap();
    Ok(i32::from_str_radix(id.trim(), 16)?)
}
