# hid-bpf-uclogic

*Work in progress*

HID-BPF reimplementation of hid-uclogic, providing support for Huion/Gaomon/... drawing tablets.

## Device support

Known working devices:

- Gaomon M7 (`256c:0064`)
- Huion HC16 (`256c:0064`)

Device feature support

- [X] Stylus (including stylus buttons)
- [X] Button pad
- [X] Touch ring/strip (Partial support: Translated to relative wheel)
- [X] Dial
- [ ] Battery information

## Requirements

Runtime requirements:

- A recent enough Linux on a little-endian machine
- [huion-switcher]

[huion-switcher]: https://github.com/whot/huion-switcher

Build requirements:

- A Rust environment (Tested on 1.86.0)
- A BPF-targeting C compiler (For example, Clang)

If you have a different BPF compiler, set `$BPF_CC` to it; otherwise, the default is `clang -target bpfel`.

## Usage

To use `cargo run` for the following:

```console
$ cargo --config 'target."cfg(all())".runner = "sudo"' run -- <args>
```

If you want to replace the kernel hid-uclogic driver with hid-bpf-uclogic, "blacklist" the `hid-uclogic` module first. See [modprobe.d(5)].

[modprobe.d(5)]: https://man7.org/linux/man-pages/man5/modprobe.d.5.html

Firstly, find the desired device from `hid-bpf-uclogic --list-devices`:

```console
$ hid-bpf-uclogic --list-devices
- USB 1-4 GAOMON Gaomon Tablet_M7 (256c:0064)
  syspath /sys/devices/pci0000:00/0000:00:14.0/usb1/1-4
  - .0 HID 004C /sys/devices/pci0000:00/0000:00:14.0/usb1/1-4/1-4:1.0/0003:256C:0064.004C
  - .1 HID 004D /sys/devices/pci0000:00/0000:00:14.0/usb1/1-4/1-4:1.1/0003:256C:0064.004D
  - .2 HID 004E /sys/devices/pci0000:00/0000:00:14.0/usb1/1-4/1-4:1.2/0003:256C:0064.004E
```

If your device is not listed, check `hid-bpf-uclogic --list-devices-all` for all USB HID devices. If your device has vendor `256c` (shows `(256c:<something>)`) and/or responds to huion-switcher, it might work. Please [open an issue] and report your experience, if nobody else has already done so.

[open an issue]: https://github.com/dramforever/hid-bpf-uclogic/issues

Then, run `hid-bpf-uclogic --device` with the `syspath` from above. When testing, add `--wait` for convenience.

```console
$ sudo hid-bpf-uclogic --device /sys/devices/pci0000:00/0000:00:14.0/usb1/1-4 --wait
- USB 1-4 GAOMON Gaomon Tablet_M7 (256c:0064)
  syspath /sys/devices/pci0000:00/0000:00:14.0/usb1/1-4

!!! Default device functionality will be disabled, unplug and replug to reset

Running: huion-switcher /sys/devices/pci0000:00/0000:00:14.0/usb1/1-4
Found device id "GM001_T207_210524"
Device with 13 buttons, max pen pressure 8191, logical size (51689, 34308), resolution 5080, physical size in inches (10.18, 6.75)
Unbinding compatibility device /sys/devices/pci0000:00/0000:00:14.0/usb1/1-4/1-4:1.2/0003:256C:0064.004E
Unbinding compatibility device /sys/devices/pci0000:00/0000:00:14.0/usb1/1-4/1-4:1.1/0003:256C:0064.004D
Driver loaded, Ctrl-C to terminate and unload
```

At this point, the device driver should be loaded. Terminating the process will unload the driver.

If the driver fails to load or is unloaded, the device will not be functional. Unplug and replug to get back to the default state.

To load the driver independently of hid-bpf-uclogic running, run without `--wait`:

```console
$ sudo hid-bpf-uclogic --device /sys/devices/pci0000:00/0000:00:14.0/usb1/1-4
[...]
Driver loaded, to unload: rm /sys/fs/bpf/hid-bpf-uclogic-004C
```

To unload the driver, remove the file from bpffs:

```
$ sudo rm /sys/fs/bpf/hid-bpf-uclogic-004C
$ sudo rm /sys/fs/bpf/hid-bpf-uclogic-*    # Remove all
```

## Udev setup

(TODO)

## Development tips

Add this to your `.git/config` to have `git diff` show HID report descriptor comparisons:

```
[diff "hid-rdesc"]
    textconv = sh -c 'hid-decode \"$1\" | grep -F //' ''
    cachetextconv = true
```

## More information

See `doc/` directory in repository.
