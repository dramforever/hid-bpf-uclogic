# hid-bpf-uclogic

*Work in progress*

HID-BPF reimplementation of hid-uclogic, providing support for Huion/Gaomon/... drawing tablets.

## Device support

Known working devices:

- Gaomon M7 (`256c:0064`)

Device feature support

- [X] Stylus (including stylus buttons)
- [X] Button pad
- [X] Touch ring/strip (Partial support: Translated to mouse wheel)
- [ ] Dial
- [ ] Battery information

## Installation and usage

Requirements:

- [huion-switcher]
- [udev-hid-bpf] including headers from it (TODO: fix the headers part)
- A BPF-targeting C compiler ([Clang] in examples below)
- [libbpf]
- [bpftool]

[huion-switcher]: https://github.com/whot/huion-switcher
[udev-hid-bpf]: https://gitlab.freedesktop.org/libevdev/udev-hid-bpf
[Clang]: https://clang.llvm.org
[libbpf]: https://libbpf.readthedocs.io/en/latest/api.html
[bpftool]: https://bpftool.dev

### Building

To build `uclogic.bpf.o`

```console
$ clang -target bpf -O2 -g -c -o uclogic.bpf.unstripped.o src/uclogic.bpf.c
$ bpftool gen object uclogic.bpf.o uclogic.bpf.unstripped.o
```

### Manual load

The effects in this section are not presistent. The easiest way to undo everything here is to unplug and replug the device.

Find the sysfs paths of your device. One physical device often corresponds to multiple HID devices.

```console
$ udev-hid-bpf list-devices
```

For *any one* `syspath` value corresponding to the device (`syspath` should look like `/sys/bus/hid/devices/0003:256C:0064.000A`), run the following, replacing `{syspath-here}`:

```console
$ sudo huion-switcher {syspath-here}
```

The output should look like

```
HUION_FIRMWARE_ID={string-here}
HUION_MAGIC_BYTES={hex-digits-here}
```

(If you also see `HUION_PAD_MODE`, it is not supported by hid-bpf-uclogic yet.)

**Note**: At this point, the tablet will no longer respond to any input. Do not panic.

For *each* `syspath` value corresponding to the device, run the following, replacing `{syspath-here}`, and also replacing `{string-here}`, `{hex-digits-here}` with output from `sudo huion-switcher`

```console
$ sudo udev-hid-bpf --verbose add -p "HUION_FIRMWARE_ID={string-here}" -p "HUION_MAGIC_BYTES={hex-digits-here}" {syspath} - uclogic.bpf.o
```

If that worked successfully, the tablet should now be fully functional, and you should see a new tablet device in your tablet settings in your desktop environment (if applicable).

### Loading with udev rules

If you have the udev rules of `huion-switcher` installed, you can load hid-bpf-uclogic on all known devices:

```console
$ sudo udev-hid-bpf --verbose add - uclogic.bpf.o
```

(TODO: How about udev-rules for hid-bpf-uclogic?)

## Debugging

Informational messages are printed to `bpf_trace_printk`. To see them:

```console
$ sudo cat /sys/kernel/debug/tracing/trace_pipe
```

## More information

See `doc/` directory in repository.
