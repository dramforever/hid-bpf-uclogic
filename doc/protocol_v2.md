# Protocol v2 description

The information here is gathered based on code and documentation of [DIGImend] or `hid-uclogic` in Linux, [huion-switcher], [udev-hid-bpf], and my own reverse-engineering.

[DIGImend]: https://github.com/DIGImend/digimend-kernel-drivers
[huion-switcher]: https://github.com/whot/huion-switcher
[udev-hid-bpf]: https://gitlab.freedesktop.org/libevdev/udev-hid-bpf

## Notational conventions

Device names are written as "Name (`VID:PID`)", e.g. "Gaomon M7 (`256c:0064`)". Note that USB VID:PID pairs do not uniquely identify a device.

Bytes are octets.

For convenience, all C structures shown below are packed (no padding, byte-alignment) and little-endian.

`__u8` and `__u16` are what C99 would call `uint8_t` and `uint16_t`, respectively.

```c
#include <stdint.h>

typedef uint8_t __u8;
typedef uint16_t __u16;
```

`__u24` is a 24-bit integer type. It can be represented in C as:

```c
typedef struct {
    unsigned val : 24;
} __attribute__((packed)) __u24;

_Static_assert(sizeof(__u24) == 3);
_Static_assert(_Alignof(__u24) == 1);
```

## String descriptors

Certain string descriptors with language `0x0409` ("English (United States)") have vendor-specific meaning.

### Firmware version

Seems to be the firmware version. The format is unknown.

Examples:

- Gaomon M7 (`256c:0064`): `GM001_T207_210524`
- Huion Keydial KD20 (`256c:0069`): `HUION_T21h_230511`

The udev rules provided by huion-switcher sets `HUION_FIRMWARE_ID` (and also `HID_UNIQ` and `UNIQ`) to this string.

### Magic bytes

A description of the tablet's parameters. The format is partially known (see below).

Examples (hex encoded):

- Gaomon M7 (`256c:0064`): `1303e9c900048600ff1fd813030d1000043c3e`
- Huion Keydial KD20 (`256c:0069`): `1403010000010000000000000013008040002808`


This string is binary data and does not follow proper encoding. The udev rules provided by huion-switcher sets `HUION_MAGIC_BYTES` to the hexadecimal encoding of this "string".

## Magic bytes

The magic bytes "string" should be at least 18 bytes in size.

```c
struct magic_v2 {
    __u8 total_length;
    __u8 _unknown_0;
    __u24 x_max;
    __u24 y_max;
    __u16 pressure_max;
    __u16 resolution;
    __u8 _unknown_1;
    __u8 num_btns;
    __u8 _unknown[];
} __attribute__((packed));

_Static_assert(sizeof(struct magic_v2) == 18);
```

The meanings of fields are as follows:

- `total_length`: Total length of the magic bytes, in bytes
- `_unknown_0`
- `x_max`: Logical maximum of tablet X position
- `y_max`: Logical maximum of tablet Y position
- `pressure_max`: Logical maximum of tablet pen pressure
- `resolution`: Resolution in units per inch
- `_unknown_1`
- `num_btns`: Number of buttons
- `_unknown[]`

Example: Gaomon M7 (`256c:0064`)

```
13          Total length (0x13 = 19)
03          ?
e9 c9 00    X logical max (0xc9e9 = 51689)
04 86 00    Y logical max (0x8604 = 34308)
ff 1f       Pressure logical max (0x1fff = 8191)
d8 13       Resolution (0x13d8 = 5080)
03          ?
0d          Number of buttons (0xd = 13)
10 00 04 3c 3e      ?
```

(According to the magic bytes, the Gaomon M7 has size 10.2x6.8 in, which roughly matches the advertised 10x6.25 in plus some margin.)

Example: Huion Keydial KD20 (`256c:0069`), which does not have a tablet

```
14          Total length (0x14 = 20)
03          ?
01 00 00    X logical max (1)
01 00 00    Y logical max (1)
00 00       Pressure logical max (0)
00 00       Resolution (0)
00          ?
13          Number of buttons (0x13 = 19)
00 80 40 00 28 08   ?
```

## Input reports

Reports should be at least 12 bytes in length. All reports seem to all use the same `report_id`, namely `0x08`, and encodes the type of report some other way.

### Button pad report

```c
struct {
    __u8 report_id;
    __u8 discriminant;
    __u8 _unknown_0;
    __u8 _unknown_1;
    __u8 btns[];
} __attribute__((packed));
```

The meanings of fields are as follows:

- `report_id`: `0x08`
- `discriminant`: `0xe0`
- `_unknown_0`: `0x01`, possibly some sort of ID?
- `_unknown_1`: `0x01`, possibly some sort of ID?
- `btns[]`: A bitmap of button presses, starting at LSB of first byte

Example: Gaomon M7 (`256c:0064`)

```
08 e0 01 01 01 00 00 00 00 00 00 00     Button 1 press
08 e0 01 01 00 00 00 00 00 00 00 00     Button 1 release
08 e0 01 01 00 10 00 00 00 00 00 00     Button 13 press
08 e0 01 01 00 00 00 00 00 00 00 00     Button 13 release
```

## Dial report

TODO

```c
struct {
    __u8 report_id;
    __u8 discriminant;
    __u8 _unknown_0;
    __u8 _unknown_1;
    __u8 _unknown_2;
    bool dial_cw : 1;
    bool dial_ccw : 1;
    __u8 _unknown_3 : 6;
    __u8 _unknown[];
} __attribute__((packed));
```

The meanings of fields are as follows:

- `report_id`: `0x08`
- `discriminant`: `0xf1`
- `_unknown_0`: possibly some sort of ID?
- `_unknown_1`: possibly some sort of ID?
- `_unknown_2`
- `dial_cw`: Dial turning clockwise
- `dial_ccw`: Dial turning counter-clockwise
- `_unknown_3`: padding?
- `_unknown[]`

Example: Huion Kamvas 13 (Gen 3) (`256c:2008`)

```
08 f1 01 01 00 01 00 00 00 00 00 00 00 00   Top wheel CW
08 f1 01 01 00 02 00 00 00 00 00 00 00 00   Top wheel CCW
08 f1 01 02 00 01 00 00 00 00 00 00 00 00   Bottom wheel CW
08 f1 01 02 00 02 00 00 00 00 00 00 00 00   Bottom wheel CCW
```

Example: Huion Keydial KD20 (`256c:0069`)

```
08 f1 01 01 00 01 00 00 00 00 00 00
08 f1 01 01 00 02 00 00 00 00 00 00
```

### Stylus report

```c
struct stylus_report {
	__u8 report_id;
	bool tip_switch : 1;
	bool barrel_switch : 1;
	bool secondary_barrel_switch : 1;
	__u8 _unknown_0 : 4;
	bool in_range : 1;
	__u16 x_low;
	__u16 y_low;
	__u16 pressure;
	__u8 x_high;
	__u8 y_high;
	__u8 x_tilt;
	__u8 y_tilt;
    __u8 _unknown[];
} __attribute__((packed));

assert(sizeof(struct stylus_report) == 12);
```

The meanings of fields are as follows:

- `report_id`: `0x08`
- `tip_switch`: Whether the stylus tip is pressed
- `barrel_switch`: Button on stylus
- `secondary_barrel_switch`: Button on stylus
- `_unknown_0`: Possibly eraser or extra buttons?
- `in_range`: Stylus is connected
- `x_low`: Low 16 bits of X position
- `y_low`: Low 16 bits of Y position
- `pressure`: Pen tip pressure
- `x_high`: High 8 bits of X position
- `y_high`: High 8 bits of Y position
- `x_tilt`: Stylus tilt on X axis (two's complement, `[-60, 60]` range)
- `y_tilt`: Stylus tilt on Y axis (two's complement, `[-60, 60]` range)
- `_unknown[]`

Example: Gaomon M7 (`256c:0064`)

```
08 80 a0 05 08 0a 00 00 00 00 00 00     Pen hovering near top left
08 00 a0 05 08 0a 00 00 00 00 00 00     Pen away
08 80 e9 c9 04 86 00 00 00 00 00 00     Pen hovering near bottom right
08 00 e9 c9 04 86 00 00 00 00 00 00     Pen away

(All following done with pen near top left)

08 82 d8 00 77 07 00 00 00 00 00 00     Press lower button
08 84 22 06 26 0c 00 00 00 00 00 00     Press upper button
08 80 92 16 b6 0c 00 00 00 00 da 00     Tilt left
08 80 0c 0a 5a 0d 00 00 00 00 2e 00     Tilt right
08 80 d1 07 aa 10 00 00 00 00 00 29     Tilt up
08 00 91 0d 63 08 00 00 00 00 00 d9     Tilt down
08 81 03 00 64 09 21 03 00 00 00 00     Tap low pressure
08 81 e0 03 8b 0d ff 1f 00 00 00 00     Tap max pressure
```

Example: Huion Kamvas 13 (Gen 3) (`256c:2008`)

```
08 81 d2 66 04 40 ff 3f 00 00 00 08 03 00       Tap max pressure
08 81 9d 69 47 3e 82 04 00 00 00 07 03 00       Tap low pressure
```

(The meaning of the trailing `03 00` is unknown.)

### Touch ring report

TODO

### Touch strip report

TODO

### Battery report

TODO
