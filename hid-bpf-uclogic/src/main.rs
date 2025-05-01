mod descriptor;
mod sysfs;

use libbpf_rs::{Link, MapCore, ObjectBuilder};
use std::{
    collections::HashMap,
    error::Error,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};

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

fn main() -> Result<(), Box<dyn Error>> {
    use clap::{Arg, ArgAction};

    libbpf_rs::set_print(Some((libbpf_rs::PrintLevel::Info, |level, s| {
        s.lines().for_each(|l| eprintln!("libbpf: {level:?} {l}"))
    })));

    let args = clap::Command::new("hid-bpf-uclogic")
        .about("Load helper for hid-bpf-uclogic")
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

    sysfs::find_sysfs()?;
    sysfs::find_bpffs()?;

    if args.get_flag("list-devices") || args.get_flag("list-devices-all") {
        let show_all = args.get_flag("list-devices-all");

        list_devices(show_all)?;
    } else {
        load(&Args {
            device: args.get_one::<OsString>("device").unwrap().clone(),
            with_huion_switcher: args.get_one("with-huion-switcher").cloned(),
            device_info: args.get_one("device-info").cloned(),
            force: args.get_flag("force"),
            wait: args.get_flag("wait"),
            quiet: args.get_flag("quiet"),
        })?;
    }
    Ok(())
}

fn load(args: &Args) -> Result<(), Box<dyn Error>> {
    use std::os::unix::ffi::OsStrExt;

    let device: &Path = args.device.as_ref();
    let device = device.canonicalize()?;

    let mut hid_dev: Option<(i32, PathBuf)> = None;

    let Some(mut hids) = find_usb_hid()?.get(&device).cloned() else {
        Err("Device does not seem to have HID child devices")?
    };

    if !args.force {
        if !usb_supported(&device)? {
            Err("Device is not supported")?;
        }
    }

    let device_info = if let Some(device_info) = &args.device_info {
        std::fs::read_to_string(device_info)?
    } else {
        let huion_switcher = args
            .with_huion_switcher
            .clone()
            .unwrap_or("huion-switcher".into());

        call_huion_switcher(&device, huion_switcher, args.quiet)?
    };

    let info = descriptor::DeviceInfo::from_str(&device_info)?;

    if !args.quiet {
        eprintln!("Found device id {:?}", info.firmware);
    }

    if !args.force && !SUPPORTED_FIRMWARE.contains(&info.firmware.as_str()) {
        Err(format!("Unsupported device {:?}", info.firmware))?
    }

    let parsed = info.parse()?;
    if !args.quiet {
        eprintln!("{}", parsed);
    }
    let new_rdesc = parsed.descriptor()?;

    // Iterate by decreasing path length so we unbind child devices first
    hids.sort_by_key(|(_, p)| p.as_os_str().len());

    for (num, hid) in hids.into_iter().rev() {
        if num == 0 {
            hid_dev = Some((sysfs::hid_device_id(&hid)?, hid));
            continue;
        }

        if !args.quiet {
            eprintln!("Unbinding compatibility device {}", hid.display());
        }
        let base = hid.file_name().unwrap();
        let res = fs::write(hid.join("driver/unbind"), base.to_os_string().as_bytes());
        match res {
            Ok(()) => (),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (),
            Err(e) => Err(e)?,
        }
    }

    let Some((hid_id, hid_dev)) = hid_dev else {
        Err("No vendor interface found")?
    };

    let bpffs_name = format!("/sys/fs/bpf/hid-bpf-uclogic-{hid_id:04X}");

    if PathBuf::from(&bpffs_name).exists() {
        if args.force {
            if ! args.quiet {
                eprintln!("Driver already exists at {bpffs_name}");
            }
        } else {
            Err(format!("Driver already exists, to remove: rm {bpffs_name}"))?;
        }
    }

    if !args.force {
        let orig_rdesc = fs::read(hid_dev.join("report_descriptor"))?;

        if orig_rdesc[..3] != [0x06, 0x00, 0xff] {
            Err("Found HID device is not what was unexpected.")?
        }
    }

    let link = fixup_device(hid_id, &new_rdesc)?;

    if args.wait {
        eprintln!("Driver loaded, Ctrl-C to terminate and unload");
        loop {
            // SAFETY: This function is safe
            unsafe {
                libc::pause();
            }
        }
    } else {
        { link }.pin(&bpffs_name)?;
        if !args.quiet {
            eprintln!("Driver loaded, to unload: rm {bpffs_name}");
        }
    }

    Ok(())
}

fn call_huion_switcher(
    device: &Path,
    huion_switcher: OsString,
    quiet: bool,
) -> Result<String, Box<dyn Error>> {
    let mut command = std::process::Command::new(&huion_switcher);
    command
        .arg(device)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    if !quiet {
        eprintln!(
            "Running: {} {}",
            huion_switcher.to_string_lossy(),
            device.display()
        );
        eprintln!("\n!!! Device will require unplugging and replugging to reset\n");
    }
    let output = command.output()?;
    if !output.stderr.is_empty() {
        Err(format!(
            "huion-switcher failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))?;
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn list_devices(show_all: bool) -> Result<(), Box<dyn Error>> {
    let devs = find_usb_hid()?;
    for (usb, hids) in devs {
        if !usb_supported(&usb)? && !show_all {
            continue;
        }
        let usb = usb.canonicalize()?;
        let base = usb.file_name().unwrap();

        let prop = |name, default: &str| {
            sysfs::property_trim(&usb, name).map(|p| p.unwrap_or(default.to_owned()))
        };
        let vid = prop("idVendor", "????")?;
        let pid = prop("idProduct", "????")?;
        let manufacturer = prop("manufacturer", "(Unknown manufacturer)")?;
        let product = prop("product", "(Unknown product)")?;

        println!(
            "- USB {base} {manufacturer} {product} ({vid}:{pid})
  syspath {usb}",
            base = base.to_string_lossy(),
            usb = usb.to_string_lossy(),
        );

        for (num, hid) in hids {
            let hid = hid.canonicalize()?;
            let id = sysfs::hid_device_id(&hid)?;
            println!("  - .{num} HID {id:04X} {hid}", hid = hid.display());
        }
    }
    Ok(())
}

fn usb_supported(device: &Path) -> Result<bool, Box<dyn Error>> {
    if sysfs::get_subsystem(device)? != Some("usb".to_owned()) {
        return Ok(false);
    }

    let Some(vid) = sysfs::property_trim(device, "idVendor")? else {
        return Ok(false);
    };
    let Ok(vid) = u32::from_str_radix(&vid, 16) else {
        return Ok(false);
    };
    let Some(pid) = sysfs::property_trim(device, "idProduct")? else {
        return Ok(false);
    };
    let Ok(pid) = u32::from_str_radix(&pid, 16) else {
        return Ok(false);
    };

    Ok(SUPPORTED_VID_PID.contains(&(vid, pid)))
}

fn find_usb_hid() -> Result<HashMap<PathBuf, Vec<(u8, PathBuf)>>, Box<dyn Error>> {
    let sys = sysfs::find_sysfs()?;
    let sys_usb = sysfs::find_subsystem(&sys, "usb")?.expect("No usb subsystem found");
    let sys_hid = sysfs::find_subsystem(&sys, "hid")?.expect("No hid subsystem found");
    let mut devices: HashMap<PathBuf, Vec<(u8, PathBuf)>> = HashMap::new();
    for dev in sysfs::subsystem_devices(&sys_usb)? {
        if sysfs::property(&dev, "devnum")?.is_some() {
            devices.insert(dev.canonicalize()?, Vec::new());
        }
    }
    for dev in sysfs::subsystem_devices(&sys_hid)? {
        let dev = dev.canonicalize()?;
        if let Some(parent) = dev.ancestors().find(|&p| devices.contains_key(p)) {
            let Some(interface_num) = sysfs::find_map_parent(&dev, |p| {
                if sysfs::get_subsystem(p)? != Some("usb".to_owned()) {
                    return Ok(None);
                }
                Ok(sysfs::property_trim(p, "bInterfaceNumber")?
                    .and_then(|s| u8::from_str_radix(&s, 16).ok()))
            })?
            else {
                // No interface number...?
                continue;
            };

            devices.get_mut(parent).unwrap().push((interface_num, dev));
        }
    }

    // Make it look better
    devices.values_mut().for_each(|x| x.sort_by_key(|p| p.0));

    // Only keep USB devices with HID children
    devices.retain(|_, devs| !devs.is_empty());
    Ok(devices)
}

fn fixup_device(hid_id: i32, rdesc: &[u8]) -> Result<Link, Box<dyn Error>> {
    let mut open_obj = ObjectBuilder::default().open_file("uclogic.bpf.o")?;
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
