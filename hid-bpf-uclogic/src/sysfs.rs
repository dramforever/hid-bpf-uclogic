use std::{
    ffi::{CString, OsString},
    io::{self, Read, Write},
    os::fd::{AsRawFd, FromRawFd},
    path::PathBuf,
    str::FromStr,
};

use eyre::Result;

use openat::Dir;

#[derive(Debug)]
pub(crate) struct Sysfs {
    dir: Dir,
    dev: libc::dev_t,
}

impl Sysfs {
    pub(crate) fn get() -> io::Result<Self> {
        use std::mem::MaybeUninit;

        let dir = Dir::open("/sys")?;
        let fd = dir.as_raw_fd();

        let sfs = unsafe {
            let mut sfs = MaybeUninit::zeroed();
            let res = libc::fstatfs(fd, sfs.as_mut_ptr());
            if res < 0 {
                return Err(std::io::Error::last_os_error());
            }
            sfs.assume_init()
        };

        if sfs.f_type != libc::SYSFS_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "/sys is not sysfs, giving up",
            ));
        }

        let dev = dir.self_metadata()?.stat().st_dev;

        Ok(Self { dir, dev })
    }
}

#[derive(Debug)]
pub(crate) struct SysfsPath<'a> {
    sysfs: &'a Sysfs,
    dir: Dir,
    ino: libc::ino_t,
}

impl PartialEq for SysfsPath<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.ino == other.ino
    }
}

impl Eq for SysfsPath<'_> {}

impl std::hash::Hash for SysfsPath<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ino.hash(state)
    }
}

impl Sysfs {
    pub(crate) fn root(&self) -> io::Result<SysfsPath> {
        let dir = self.dir.try_clone()?;
        let ino = self.dir.self_metadata()?.stat().st_ino;
        Ok(SysfsPath {
            sysfs: self,
            dir,
            ino,
        })
    }
}

fn sub_dir_follow(dir: &Dir, name: impl openat::AsPath<Buffer = CString>) -> io::Result<Dir> {
    use libc::*;

    // Stolen from openat
    #[cfg(target_os = "linux")]
    const BASE_OPEN_FLAGS: c_int = O_PATH | O_CLOEXEC;
    #[cfg(target_os = "freebsd")]
    const BASE_OPEN_FLAGS: c_int = O_DIRECTORY | O_CLOEXEC;
    #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
    const BASE_OPEN_FLAGS: c_int = O_CLOEXEC;

    let fd = unsafe {
        openat(
            dir.as_raw_fd(),
            name.to_path().expect("Invalid file name").as_ptr(),
            BASE_OPEN_FLAGS,
        )
    };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { Dir::from_raw_fd(fd) })
    }
}

impl SysfsPath<'_> {
    pub(crate) fn sub(
        &self,
        name: impl openat::AsPath<Buffer = CString>,
    ) -> io::Result<Option<Self>> {
        let Some(dir) = map_not_found(sub_dir_follow(&self.dir, name))? else {
            return Ok(None);
        };
        let stat = *dir.self_metadata()?.stat();

        let sub = Self {
            sysfs: self.sysfs,
            dir,
            ino: stat.st_ino,
        };

        Ok((stat.st_dev == self.sysfs.dev).then_some(sub))
    }

    pub(crate) fn get_subsystem(&self, name: &str) -> io::Result<Option<Self>> {
        for dir in ["subsytem", "bus", "class", "block"] {
            if let Some(s) = self.sub(dir)? {
                if let Some(s) = s.sub(name)? {
                    return Ok(Some(s));
                }
            }
        }
        Ok(None)
    }

    pub(crate) fn recover_path(&self) -> io::Result<PathBuf> {
        self.dir.recover_path()
    }

    pub(crate) fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.dir.open_file(path)?.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn write(&self, path: &str, data: &[u8]) -> io::Result<()> {
        use libc::*;

        let fd = self.dir.as_raw_fd();
        let file = unsafe {
            let fd = openat(
                fd,
                CString::from_str(path).unwrap().as_ptr(),
                O_WRONLY | O_CLOEXEC | O_NOFOLLOW,
            );
            if fd < 0 {
                return Err(io::Error::last_os_error());
            }
            std::fs::File::from_raw_fd(fd)
        };
        { file }.write_all(data)
    }

    pub(crate) fn property(&self, name: &str) -> io::Result<Option<String>> {
        let Some(file) = map_not_found(self.dir.open_file(name))? else {
            return Ok(None);
        };
        let mut s = String::new();
        { file }.read_to_string(&mut s)?;
        Ok(Some(s))
    }

    pub(crate) fn subsystem(&self) -> io::Result<Option<String>> {
        let Some(link) = map_not_found(self.dir.read_link("subsystem"))? else {
            return Ok(None);
        };
        Ok(Some(link.file_name().unwrap().to_str().unwrap().to_owned()))
    }

    pub(crate) fn devices(&self) -> io::Result<impl Iterator<Item = io::Result<(OsString, Self)>>> {
        let iter = self.dir.list_dir("devices")?;
        let entry = move |ent: openat::Entry| {
            let Some(devices) = self.sub("devices")? else {
                return Ok(None);
            };
            let link = devices.dir.read_link(ent.file_name())?;
            let Some(dev) = devices.sub(&link)? else {
                return Ok(None);
            };
            Ok(Some((ent.file_name().to_os_string(), dev)))
        };
        Ok(iter.filter_map(move |r| r.and_then(entry).transpose()))
    }

    pub(crate) fn property_trim(&self, name: &str) -> io::Result<Option<String>> {
        Ok(self.property(name)?.map(|s| s.trim().to_owned()))
    }

    pub(crate) fn find_map_parent<T, F>(&self, mut f: F) -> io::Result<Option<T>>
    where
        F: FnMut(Self) -> io::Result<Option<T>>,
    {
        let mut dir = self.sub(".")?.unwrap();

        loop {
            let next = dir.sub("..");

            if let Some(res) = f(dir)? {
                return Ok(Some(res));
            }

            dir = if let Some(parent) = next? {
                parent
            } else {
                return Ok(None);
            }
        }
    }
}

fn map_not_found<T>(result: Result<T, io::Error>) -> Result<Option<T>, io::Error> {
    match result {
        Ok(result) => Ok(Some(result)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) if e.kind() == io::ErrorKind::InvalidInput => Ok(None),
        Err(e) => Err(e),
    }
}
