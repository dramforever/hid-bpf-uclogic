// SPDX-License-Identifier: GPL-2.0-only

#include "vmlinux.h"
#include "hid_bpf.h"
#include "hid_bpf_helpers.h"
#include "hid_report_helpers.h"
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_endian.h>

#define NUM_BTN_MISC 10
#define NUM_BTN_GAMEPAD 15
#define MAX_NUM_BTNS (NUM_BTN_MISC + NUM_BTN_GAMEPAD)

#define USB_VENDOR_ID_HUION		0x256c
#define USB_DEVICE_ID_HUION_TABLET3	0x0064

HID_BPF_CONFIG(
	HID_DEVICE(BUS_USB, HID_GROUP_GENERIC,
		USB_VENDOR_ID_HUION, USB_DEVICE_ID_HUION_TABLET3)
);

/* Filled in by udev-hid-bpf */
char UDEV_PROP_HUION_FIRMWARE_ID[64];
char UDEV_PROP_HUION_MAGIC_BYTES[64];
char UDEV_PROP_HUION_PAD_MODE[64];

static inline __u8 magic_bytes_get_u8(size_t index)
{
	char *str = UDEV_PROP_HUION_MAGIC_BYTES + index * 2;
	unsigned long res;
	long ret = bpf_strtoul(str, 2, 16, &res);
	return res;
}

struct magic_info {
	__u32 lmax_x;
	__u32 lmax_y;
	__u16 lmax_pressure;
	__u16 resolution;
	__u8 num_btns;
};

static inline bool is_hex(char c)
{
	return ('0' <= c && c <= '9')
		|| ('a' <= c && c <= 'f')
		|| ('A' <= c && c <= 'F');
}

static inline long magic_bytes_len()
{
	char *magic = UDEV_PROP_HUION_MAGIC_BYTES;

	if (! is_hex(magic[0]) || ! is_hex(magic[1]))
		return -EINVAL;

	__u8 len = magic_bytes_get_u8(0);

	#pragma unroll
	for (size_t i = 0; i != sizeof(UDEV_PROP_HUION_MAGIC_BYTES); i ++) {
		char b = magic[i];
		if (i < len * 2) {
			if (! is_hex(b))
				return -EINVAL;
		} else {
			if (b != 0)
				return -EINVAL;
		}
	}

	return len;
}

static inline int parse_magic_bytes_v2(unsigned int id, struct magic_info *info)
{
	long len = magic_bytes_len();

	if (len < 0) {
		bpf_printk("Error parsing magic bytes");
		return -EINVAL;
	}

	if (len < 18) {
		bpf_printk("Magic bytes too short for v2");
		return -EINVAL;
	}

#define M(i) ((__u32)magic_bytes_get_u8(i))

	info->lmax_x = M(2)| (M(3) << 8) | (M(4) << 16);
	info->lmax_y = M(5)| (M(6) << 8) | (M(7) << 16);
	info->lmax_pressure = M(8) | (M(9) << 8);
	info->resolution = M(10) | (M(11) << 8);
	info->num_btns = M(13);

#undef M

	return 0;
}

static __always_inline int probe_device(
	unsigned int id,
	__u8 *rdesc,
	unsigned int size,
	struct magic_info *info)
{
	if (UDEV_PROP_HUION_FIRMWARE_ID[0] == 0) {
		bpf_printk(
			"%04x: No HUION_FIRMWARE_ID found, "
			"huion-switcher udev rules missing or failed?",
			id);

		return -EINVAL;
	}

	if (UDEV_PROP_HUION_PAD_MODE[0] != 0) {
		bpf_printk("%04x: Device is v1, not implemented!", id);
		return -EINVAL;
	}

	int ret = parse_magic_bytes_v2(id, info);

	if (ret < 0) {
		return ret;
	}

	bpf_printk(
		"%04x: Found v2 tablet, max x = %u, max y = %u, "
		"max pressure = %u, resolution = %d, num of buttons = %u",
		id, info->lmax_x, info->lmax_y,
		info->lmax_pressure, info->resolution, info->num_btns);

	if (info->num_btns > MAX_NUM_BTNS) {
		bpf_printk("%04x: Too many buttons, have %d, max %d",
			info->num_btns, MAX_NUM_BTNS);
		return -EINVAL;
	}

	if (size < 3) {
		bpf_printk("%04x: Descriptor too short", id);
		return -EINVAL;
	}

	if (__builtin_memcmp(rdesc, (__u8[]){ 0x06, 0x00, 0xff }, 3) == 0) {
		bpf_printk("%04x: Vendor interface found, will fixup", id);
		return 1;
	} else {
		bpf_printk("%04x: Standard interface found, will disable", id);
		return 0;
	}
}

#define PAD_REPORT_ID 3
#define VENDOR_REPORT_ID 8
#define REPORT_SIZE 12

#define DESCRIPTOR_FILE "uclogic_v2.in.h"
#define DESCRIPTOR_NAME rdesc_uclogic_v2
#include "mk_template.h"

#define use_btn_gamepad
#define DESCRIPTOR_FILE "uclogic_v2.in.h"
#define DESCRIPTOR_NAME rdesc_uclogic_v2_gamepad
#include "mk_template.h"
#undef use_btn_gamepad

static const __u8 disabled_rdesc[] = {
	FixedSizeVendorReport(64)
};

struct stylus_flags {
	bool tip_switch: 1;
	bool barrel_switch: 1;
	bool secondary_barrel_switch: 1;
	__u8 _padding: 4;
	bool in_range: 1;
} __attribute__((packed));

union vendor_report {
	struct {
		__u8 report_id;
		__u8 discriminant;
	} __attribute__((packed));

	struct {
		__u8 report_id;
		struct stylus_flags flags;
		__u16 x_low;
		__u16 y_low;
		__u16 pressure;
		__u8 x_high;
		__u8 y_high;
		__u8 x_tilt;
		__u8 y_tilt;
	} __attribute__((packed)) stylus;

	struct {
		__u8 report_id;
		__u8 discriminant;
		__u8 _padding_0;
		__u8 _padding_1;
		__u8 btns[8];
	} __attribute__((packed)) pad;
} __attribute__((packed));

union report {
	struct {
		__u8 report_id;
		struct stylus_flags flags;
		__u32 x: 24;
		__u32 y: 24;
		__u16 pressure;
		__u8 x_tilt;
		__u8 y_tilt;
	} __attribute__((packed)) stylus;

	struct {
		__u8 report_id;
		__u8 btn_stylus;
		__u8 x;
		__u8 y;
		__u8 btns[8];
	} __attribute__((packed)) pad;
} __attribute__((packed));

#define REPORT_NUM_BTN_BITS ((sizeof(union vendor_report) - 4) * 8)

bool should_fix_event;

SEC(HID_BPF_DEVICE_EVENT)
int BPF_PROG(uclogic_fix_event, struct hid_bpf_ctx *hid_ctx)
{
	if (!should_fix_event)
		return 0;

	__u8 *data = hid_bpf_get_data(hid_ctx, 0, REPORT_SIZE);
	__s32 size = hid_ctx->size;

	if (!data || size != REPORT_SIZE)
		return 0;

	union vendor_report v = *(union vendor_report*)data;
	union report *r = (union report *)data;

	if (v.report_id != VENDOR_REPORT_ID)
		return 0;

	if (v.discriminant == 0xe0) {
		// Pad event
		_Static_assert(sizeof(v.pad) == REPORT_SIZE, "");
		_Static_assert(sizeof(r->pad) == REPORT_SIZE, "");

		r->pad.report_id = PAD_REPORT_ID;
		r->pad.btn_stylus = 0;
		r->pad.x = 0;
		r->pad.y = 0;
		__builtin_memcpy(r->pad.btns, v.pad.btns, sizeof(r->pad.btns));
	} else {
		// Stylus event
		_Static_assert(sizeof(v.stylus) == REPORT_SIZE, "");
		_Static_assert(sizeof(r->stylus) == REPORT_SIZE, "");

		__u32 x = ((__u32)v.stylus.x_high << 16) | v.stylus.x_low;
		__u32 y = ((__u32)v.stylus.y_high << 16) | v.stylus.y_low;

		r->stylus.report_id = VENDOR_REPORT_ID;
		r->stylus.flags = v.stylus.flags;
		r->stylus.x = x;
		r->stylus.y = y;
		r->stylus.pressure = v.stylus.pressure;
		r->stylus.x_tilt = v.stylus.x_tilt;
		r->stylus.y_tilt = v.stylus.y_tilt;
	}

	return sizeof(*r);
}

SEC(HID_BPF_RDESC_FIXUP)
int BPF_PROG(uclogic_fix_rdesc, struct hid_bpf_ctx *hid_ctx)
{
	__u8 *data = hid_bpf_get_data(hid_ctx, 0, HID_MAX_DESCRIPTOR_SIZE);
	__s32 size = hid_ctx->size;

	if (!data || size < 0)
		return 0;

	struct magic_info info;
	int ret = probe_device(hid_ctx->hid->id, data, size, &info);

	if (ret < 0) {
		return 0;
	} else if (ret) {
		// Vendor interface, patch
		should_fix_event = true;

		__u32 pmax_x, pmax_y;
		if (info.resolution) {
			pmax_x = info.lmax_x * 1000 / info.resolution;
			pmax_y = info.lmax_y * 1000 / info.resolution;
		} else {
			pmax_x = 0;
			pmax_y = 0;
		}

		__u8 num_btn_padding = REPORT_NUM_BTN_BITS - info.num_btns;

		if (info.num_btns > NUM_BTN_MISC) {
			bpf_printk("%04x: Using both BTN_MISC and BTN_GAMEPAD",
				hid_ctx->hid->id);
			struct rdesc_uclogic_v2_gamepad rdesc =
				rdesc_uclogic_v2_gamepad;
			rdesc.lmax_x = info.lmax_x;
			rdesc.lmax_y = info.lmax_y;
			rdesc.pmax_x = pmax_x;
			rdesc.pmax_y = pmax_y;
			rdesc.lmax_pressure = info.lmax_pressure;
			rdesc.num_btn_misc_1 = NUM_BTN_MISC;
			rdesc.num_btn_misc_2 = NUM_BTN_MISC;
			rdesc.num_btn_gamepad_1 = info.num_btns - NUM_BTN_MISC;
			rdesc.num_btn_gamepad_2 = info.num_btns - NUM_BTN_MISC;
			rdesc.num_btn_padding = num_btn_padding;
			__builtin_memcpy(data, &rdesc,  sizeof(rdesc));
			return sizeof(rdesc);
		} else {
			bpf_printk("%04x: Using only BTN_MISC",
				hid_ctx->hid->id);
			struct rdesc_uclogic_v2 rdesc = rdesc_uclogic_v2;
			rdesc.lmax_x = info.lmax_x;
			rdesc.lmax_y = info.lmax_y;
			rdesc.pmax_x = pmax_x;
			rdesc.pmax_y = pmax_y;
			rdesc.lmax_pressure = info.lmax_pressure;
			rdesc.num_btn_misc_1 = info.num_btns;
			rdesc.num_btn_misc_2 = info.num_btns;
			rdesc.num_btn_padding = num_btn_padding;
			__builtin_memcpy(data, &rdesc, sizeof(rdesc));
			return sizeof(rdesc);
		}
	} else {
		// Standard interface, disable
		__builtin_memcpy(data, disabled_rdesc, sizeof(disabled_rdesc));
		return sizeof(disabled_rdesc);
	}

	return 0;
}

HID_BPF_OPS(uclogic) = {
	.hid_device_event = (void *)uclogic_fix_event,
	.hid_rdesc_fixup = (void *)uclogic_fix_rdesc,
};

SEC("syscall")
int probe(struct hid_bpf_probe_args *ctx)
{
	static struct magic_info info;
	int ret = probe_device(ctx->hid, ctx->rdesc, ctx->rdesc_size, &info);

	if (ret < 0)
		ctx->retval = ret;
	else
		ctx->retval = 0;

	return 0;
}

char _license[] SEC("license") = "GPL";
