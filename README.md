# hid-bpf-uclogic

*Work in progress*

HID-BPF reimplementation of hid-uclogic, which provides support for UC-Logic (rebranded as Huion, Gaomon, etc.) drawing tablets.

## Device support

Tested on these devices:

- Gaomon M7 (`256c:0064`)

Device feature support

- [X] Stylus (including stylus buttons)
- [X] Button pad
- [ ] Dial
- [ ] Touch ring
- [ ] Touch strip
- [ ] Battery information

## Installation and usage

Requirements:

- [udev-hid-bpf] including headers from it (TODO: how?)
- A BPF-targeting compiler ([Clang] in examples below)
- [libbpf]
- [bpftool]

[udev-hid-bpf]: https://gitlab.freedesktop.org/libevdev/udev-hid-bpf
[Clang]: https://clang.llvm.org
[libbpf]: https://libbpf.readthedocs.io/en/latest/api.html
[bpftool]: https://bpftool.dev

Building `uclogic.bpf.o`

```console
$ clang -target bpf -O2 -g -c -o uclogic.bpf.unstripped.o src/uclogic.bpf.c
$ bpftool gen object uclogic.bpf.o uclogic.bpf.unstripped.o
```

Loading `uclogic.bpf.o` on known devices:

```console
$ sudo udev-hid-bpf --verbose add - uclogic.bpf.o
```

Loading `uclogic.bpf.o` manually:

```console
$ udev-hid-bpf list-devices
$ # Find *all* syspath attributes of desired device
$ sudo udev-hid-bpf --verbose add /sys/bus/hid/devices/... - uclogic.bpf.o

```

## Debugging

Informational messages are printed to `bpf_trace_printk`. To see them:

```console
$ sudo cat /sys/kernel/debug/tracing/trace_pipe
```

## More information

See `doc/` directory in repository.
