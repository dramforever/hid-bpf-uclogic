mod descriptor;
mod sysfs;

use libbpf_rs::{Link, MapCore, ObjectBuilder};
use std::{
    collections::HashMap,
    ffi::{CStr, OsStr, OsString},
    io,
    path::{Path, PathBuf},
};
use sysfs::{Sysfs, SysfsPath};

use eyre::{Context, OptionExt, Result, bail, eyre};

static SUPPORTED_VID_PID: &[(u32, u32)] = &[
    // Gaomon M6
    // Gaomon M7
    // Huion HC16
    (0x256c, 0x0064),
];

static SUPPORTED_FIRMWARE: &[&str] = &[
    "GM001_T207_210524", // Gaomon M7
    "HUION_T18C_211220", // Huion HC16
];

struct Args {
    device: OsString,
    with_huion_switcher: Option<OsString>,
    device_info: Option<OsString>,
    force: bool,
    quiet: bool,
    wait: bool,
}

fn main() -> Result<()> {
    use clap::{Arg, ArgAction};

    libbpf_rs::set_print(Some((libbpf_rs::PrintLevel::Info, |level, s| {
        s.lines().for_each(|l| eprintln!("libbpf: {level:?} {l}"))
    })));

    let args = clap::Command::new("hid-bpf-uclogic")
        .version(env!("CARGO_PKG_VERSION"))
        .arg_required_else_help(true)
        .arg(
            Arg::new("device")
                .required(true)
                .long("device")
                .value_name("syspath")
                .help("/sys path of device")
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new("quiet")
                .long("quiet")
                .action(ArgAction::SetTrue)
                .help("Omit informational messages"),
        )
        .arg(
            Arg::new("wait")
                .long("wait")
                .action(ArgAction::SetTrue)
                .help("Do not pin and exit after loading"),
        )
        .arg(
            Arg::new("device-info")
                .long("device-info")
                .value_name("file")
                .help("File to read instead of calling huion-switcher")
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new("with-huion-switcher")
                .long("with-huion-switcher")
                .value_name("path")
                .value_parser(clap::value_parser!(OsString))
                .help("Path to huion-switcher"),
        )
        .arg(
            Arg::new("force")
                .long("force")
                .action(ArgAction::SetTrue)
                .help("Even on unsupported device"),
        )
        .arg(
            Arg::new("list-devices")
                .exclusive(true)
                .long("list-devices")
                .action(ArgAction::SetTrue)
                .help("List supported USB HID devices"),
        )
        .arg(
            Arg::new("list-devices-all")
                .exclusive(true)
                .long("list-devices-all")
                .action(ArgAction::SetTrue)
                .help("List all USB HID devices"),
        )
        .get_matches();

    let sysfs = Sysfs::get()?;
    let sysfs = sysfs.root()?;

    if args.get_flag("list-devices") || args.get_flag("list-devices-all") {
        let show_all = args.get_flag("list-devices-all");

        list_devices(&sysfs, show_all)?;
    } else {
        load(
            &sysfs,
            &Args {
                device: args.get_one::<OsString>("device").unwrap().clone(),
                with_huion_switcher: args.get_one("with-huion-switcher").cloned(),
                device_info: args.get_one("device-info").cloned(),
                force: args.get_flag("force"),
                wait: args.get_flag("wait"),
                quiet: args.get_flag("quiet"),
            },
        )?;
    }
    Ok(())
}

fn load(sysfs: &SysfsPath, args: &Args) -> Result<()> {
    use std::os::unix::ffi::OsStrExt;

    if !args.wait {
        find_bpffs()?;
    }

    let device = sysfs
        .sub(&PathBuf::from(&args.device))?
        .ok_or_eyre("Device not found")?;

    let usb_hid = find_usb_hid(sysfs)?;

    let Some(hids) = usb_hid.get(&device) else {
        bail!("Device does not seem to be a USB HID device");
    };

    if !args.force {
        if !usb_supported(&device)? {
            bail!("Device is not supported");
        }
    }

    print_usb_device(&device)?;

    let device_info = if let Some(device_info) = &args.device_info {
        std::fs::read_to_string(device_info).wrap_err_with(|| {
            format!("Reading device info from {}", device_info.to_string_lossy())
        })?
    } else {
        let huion_switcher = args
            .with_huion_switcher
            .clone()
            .unwrap_or("huion-switcher".into());

        call_huion_switcher(&PathBuf::from(&args.device), huion_switcher, args.quiet)
            .wrap_err("Error running huion-switcher")?
    };

    let info =
        descriptor::DeviceInfo::from_str(&device_info).wrap_err("Failed to parse device info")?;

    if !args.quiet {
        eprintln!("Found device id {:?}", info.firmware);
    }

    if !args.force && !SUPPORTED_FIRMWARE.contains(&info.firmware.as_str()) {
        bail!(format!("Unsupported device {:?}", info.firmware));
    }

    let parsed = info.parse()?;
    if !args.quiet {
        eprintln!("{}", parsed);
    }
    let new_rdesc = parsed.descriptor()?;

    let mut hid_dev: Option<(i32, &SysfsPath)> = None;

    for (num, hid_name, hid) in hids.into_iter().rev() {
        if *num == 0 {
            let id = parse_hid_id(&hid_name)
                .ok_or_else(|| eyre!("Unexpected HID device name {:?}", hid_name))?;
            hid_dev = Some((id, hid));
            continue;
        }

        if !args.quiet {
            eprintln!(
                "Unbinding compatibility device {}",
                hid.recover_path()?.display()
            );
        }

        let res = hid.write("driver/unbind", hid_name.as_bytes());
        match res {
            Ok(()) => (),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => Err(e)?,
        }
    }

    let Some((hid_id, hid_dev)) = hid_dev else {
        bail!("No vendor interface found");
    };

    let bpffs_name = format!("/sys/fs/bpf/hid-bpf-uclogic-{hid_id:04X}");

    if PathBuf::from(&bpffs_name).exists() {
        if args.force {
            if !args.quiet {
                eprintln!("Driver already exists at {bpffs_name}");
            }
        } else {
            bail!(format!("Driver already exists, to remove: rm {bpffs_name}"));
        }
    }

    if !args.force {
        let orig_rdesc = hid_dev.read("report_descriptor")?;

        if orig_rdesc[..3] != [0x06, 0x00, 0xff] {
            bail!("Found HID device is not what was unexpected.");
        }
    }

    let link = fixup_device(hid_id, &new_rdesc).map_err(|e| {
        match e.downcast_ref::<libbpf_rs::Error>() {
            Some(ioe) if ioe.kind() == libbpf_rs::ErrorKind::PermissionDenied => {
                e.wrap_err("Cannot load BPF (Try running as root?)")
            }
            _ => e.wrap_err("Cannot load BPF"),
        }
    })?;

    if args.wait {
        eprintln!("Driver loaded, Ctrl-C to terminate and unload");
        loop {
            // SAFETY: This function is safe
            unsafe {
                libc::pause();
            }
        }
    } else {
        { link }
            .pin(&bpffs_name)
            .wrap_err("Failed to pin BPF link")?;
        if !args.quiet {
            eprintln!("Driver loaded, to unload: rm {bpffs_name}");
        }
    }

    Ok(())
}

fn parse_hid_id(name: &OsStr) -> Option<i32> {
    let id = name.to_str()?.split('.').last()?;
    Some(i32::from_str_radix(id, 16).ok()?)
}

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

fn find_bpffs() -> Result<PathBuf> {
    if fstype(c"/sys/fs/bpf")? != libc::BPF_FS_MAGIC {
        bail!("/sys/fs/bpf is not bpffs, please mount it with: mount -t bpf bpffs /sys/fs/bpf");
    }
    Ok("/sys/fs/bpf".into())
}

fn call_huion_switcher(device: &Path, huion_switcher: OsString, quiet: bool) -> Result<String> {
    let mut command = std::process::Command::new(&huion_switcher);
    command
        .arg(device)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    if !quiet {
        eprintln!(
            r#"
!!! Default device functionality will be disabled, unplug and replug to reset
"#
        );
        eprintln!(
            "Running: {} {}",
            huion_switcher.to_string_lossy(),
            device.display()
        );
    }
    let output = command.output()?;
    if !output.stderr.is_empty() {
        bail!(format!(
            "huion-switcher failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn list_devices(sysfs: &SysfsPath, show_all: bool) -> Result<()> {
    let devs = find_usb_hid(sysfs)?;
    for (usb, hids) in devs {
        if !show_all && !usb_supported(&usb).unwrap_or_default() {
            continue;
        }

        print_usb_device(&usb)?;

        for (num, name, hid) in hids {
            let Some(id) = parse_hid_id(&name) else {
                continue;
            };
            println!(
                "  - .{num} HID {id:04X} {hid}",
                hid = hid.recover_path()?.display()
            );
        }
    }
    Ok(())
}

fn print_usb_device(usb: &SysfsPath) -> Result<()> {
    let usb_path = usb.recover_path()?;
    let base = usb_path.file_name().unwrap();
    let prop = |name, default: &str| {
        usb.property_trim(name)
            .map(|p| p.unwrap_or(default.to_owned()))
    };
    let vid = prop("idVendor", "????")?;
    let pid = prop("idProduct", "????")?;
    let manufacturer = prop("manufacturer", "(Unknown manufacturer)")?;
    let product = prop("product", "(Unknown product)")?;
    println!(
        "- USB {base} {manufacturer} {product} ({vid}:{pid})
  syspath {usb}",
        base = base.to_string_lossy(),
        usb = usb_path.to_string_lossy(),
    );
    Ok(())
}

fn usb_supported(device: &SysfsPath) -> Result<bool> {
    if device.subsystem()? != Some("usb".to_owned()) {
        return Ok(false);
    }

    let Some(vid) = device.property_trim("idVendor")? else {
        return Ok(false);
    };
    let Ok(vid) = u32::from_str_radix(&vid, 16) else {
        return Ok(false);
    };
    let Some(pid) = device.property_trim("idProduct")? else {
        return Ok(false);
    };
    let Ok(pid) = u32::from_str_radix(&pid, 16) else {
        return Ok(false);
    };

    Ok(SUPPORTED_VID_PID.contains(&(vid, pid)))
}

fn find_usb_hid<'a>(
    sysfs: &SysfsPath<'a>,
) -> Result<HashMap<SysfsPath<'a>, Vec<(u8, OsString, SysfsPath<'a>)>>> {
    let sys_usb = sysfs
        .get_subsystem("usb")?
        .ok_or_eyre("No usb subsystem found in sysfs")?;
    let sys_hid = sysfs
        .get_subsystem("hid")?
        .ok_or_eyre("No hid subsystem found in sysfs")?;

    let mut devices: HashMap<SysfsPath, Vec<(u8, OsString, SysfsPath)>> = HashMap::new();

    for dev in sys_usb.devices()? {
        let (_, dev) = dev?;
        if dev.property("devnum")?.is_some() {
            devices.insert(dev, Vec::new());
        }
    }

    for dev in sys_hid.devices()? {
        let (name, dev) = dev?;

        if let Some(parent) = dev.find_map_parent(|p| Ok(devices.contains_key(&p).then_some(p)))? {
            let Some(interface_num) = dev.find_map_parent(|p| {
                if p.subsystem()? != Some("usb".to_owned()) {
                    return Ok(None);
                }
                Ok(p.property_trim("bInterfaceNumber")?
                    .and_then(|s| u8::from_str_radix(&s, 16).ok()))
            })?
            else {
                // No interface number...?
                continue;
            };

            devices
                .get_mut(&parent)
                .unwrap()
                .push((interface_num, name, dev));
        }
    }

    // Make it look better
    devices.values_mut().for_each(|x| x.sort_by_key(|p| p.0));

    // Only keep USB devices with HID children
    devices.retain(|_, devs| !devs.is_empty());
    Ok(devices)
}

fn fixup_device(hid_id: i32, rdesc: &[u8]) -> Result<Link> {
    let mut open_obj = ObjectBuilder::default()
        .open_memory(include_bytes!(concat!(env!("OUT_DIR"), "/uclogic.bpf.o")))?;
    let mut config = open_obj
        .maps_mut()
        .find(|m| m.name() == ".rodata.uclogic_config")
        .unwrap();
    let (conf_size, conf_data) = config.initial_value_mut().unwrap().split_at_mut(4);
    conf_size.copy_from_slice(&u32::try_from(rdesc.len()).unwrap().to_ne_bytes());
    conf_data[..rdesc.len()].copy_from_slice(rdesc);
    let mut ops = open_obj
        .maps_mut()
        .find(|m| m.name() == "uclogic_ops")
        .unwrap();
    let ops = ops.initial_value_mut().unwrap();
    ops[..4].copy_from_slice(&hid_id.to_ne_bytes());
    let mut obj = open_obj.load()?;
    let mut ops = obj.maps_mut().find(|m| m.name() == "uclogic_ops").unwrap();
    let link = ops.attach_struct_ops()?;
    Ok(link)
}
