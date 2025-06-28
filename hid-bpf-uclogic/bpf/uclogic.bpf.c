// SPDX-License-Identifier: GPL-2.0-only OR BSD-2-Clause

#if __BYTE_ORDER__ != __ORDER_LITTLE_ENDIAN__
# error "Only little-endian is supported"
#endif

// === Constants ===

#define HID_MAX_DESCRIPTOR_SIZE 4096

#define PAD_REPORT_ID 3
#define VENDOR_REPORT_ID 8
#define DIAL_REPORT_ID 0xf0

#define REPORT_SIZE 12

// === General types ===

typedef _Bool bool;
typedef unsigned char __u8;
typedef unsigned short __u16;
typedef unsigned int __u32;
typedef unsigned long long __u64;

typedef signed int __s32;

typedef unsigned long size_t;

_Static_assert(sizeof(__u8) == 1, "");
_Static_assert(sizeof(__u16) == 2, "");
_Static_assert(sizeof(__u32) == 4, "");
_Static_assert(sizeof(__u64) == 8, "");

_Static_assert(sizeof(__s32) == 4, "");

// === Implementation ===

struct state {
	__u8 touch;
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

	struct {
		__u8 report_id;
		__u8 discriminant;
		__u8 _unknown_0;
		__u8 _unknown_1;
		__u8 _unknown_2;
		__u8 position;
		__u8 _unknown[6];
	} __attribute__((packed)) touch;

	struct {
		__u8 report_id;
		__u8 discriminant;
		__u8 _unknown_0;
		__u8 dial_id;
		__u8 _unknown_2;
		bool dial_cw : 1;
		bool dial_ccw : 1;
		__u8 _unknown_3 : 6;
		__u8 _unknown[6];
	} __attribute__((packed)) dial;
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

	struct {
		__u8 report_id;
		__u8 btn_stylus;
		__u8 x;
		__u8 y;
		__u8 _padding_0;
		__u8 delta_1;
		__u8 delta_2;
		__u8 _padding[5];
	} __attribute__((packed)) dial;
} __attribute__((packed));

#define sizeof_member(type, member) \
	sizeof((*(__typeof__(type)*)0).member)

#define REPORT_NUM_BTN_BITS (8 * sizeof_member(union report, pad.btns))

_Static_assert(sizeof_member(union vendor_report, pad) == REPORT_SIZE, "");
_Static_assert(sizeof_member(union vendor_report, touch) == REPORT_SIZE, "");
_Static_assert(sizeof_member(union vendor_report, dial) == REPORT_SIZE, "");
_Static_assert(sizeof_member(union vendor_report, stylus) == REPORT_SIZE, "");
_Static_assert(sizeof_member(union report, pad) == REPORT_SIZE, "");
_Static_assert(sizeof_member(union report, dial) == REPORT_SIZE, "");
_Static_assert(sizeof_member(union report, stylus) == REPORT_SIZE, "");

#ifndef TEST
static inline
#endif
__u8 fixup_report(__u8 *new_report, const __u8 *old_report, struct state *st) {
	const union vendor_report *v = (union vendor_report*)old_report;
	union report *r = (union report *)new_report;

	if (v->report_id != VENDOR_REPORT_ID)
		return 0;

	if (v->discriminant == 0xe0) {
		// Pad event
		r->pad.report_id = PAD_REPORT_ID;
		r->pad.btn_stylus = 0;
		r->pad.x = 0;
		r->pad.y = 0;
		__builtin_memcpy(r->pad.btns, v->pad.btns, sizeof(r->pad.btns));
	} else if (v->discriminant == 0xf0) {
		// Touch event

		// Translate to relative wheel event
		// FIXME: This can't possibly be the right way

		__u8 last_touch = st->touch;
		st->touch = v->touch.position;

		if (st->touch == 0 || last_touch == 0) {
			return 0;
		}

#define abs(x) ((x) > 0 ? (x) : -(x))
		bool dir = (st->touch > last_touch) ^ (abs(st->touch - last_touch) < 4);
#undef abs
		r->dial.delta_1 = dir ? -1 : 1;

		r->dial.report_id = DIAL_REPORT_ID;
		r->dial.btn_stylus = 0;
		r->dial.x = 0;
		r->dial.y = 0;
		r->dial.delta_2 = 0;
	} else if (v->discriminant == 0xf1) {
		// Dial event
		__u8 delta = (__u8)v->dial.dial_cw - (__u8)v->dial.dial_ccw;

		r->dial.delta_1 = v->dial.dial_id == 1 ? delta : 0;
		r->dial.delta_2 = v->dial.dial_id == 2 ? delta : 0;

		r->dial.report_id = DIAL_REPORT_ID;
		r->dial.btn_stylus = 0;
		r->dial.x = 0;
		r->dial.y = 0;
	} else {
		// Stylus event
		__u32 x = ((__u32)v->stylus.x_high << 16) | v->stylus.x_low;
		__u32 y = ((__u32)v->stylus.y_high << 16) | v->stylus.y_low;

		r->stylus.report_id = VENDOR_REPORT_ID;
		r->stylus.flags = v->stylus.flags;
		r->stylus.x = x;
		r->stylus.y = y;
		r->stylus.pressure = v->stylus.pressure;
		r->stylus.x_tilt = v->stylus.x_tilt;
		r->stylus.y_tilt = v->stylus.y_tilt;
	}

	return 1;
}

#ifndef TEST

#define SEC(name) __attribute__((section(name)))

// === Kernel types ===

enum hid_report_type {
	HID_INPUT_REPORT = 0,
	HID_OUTPUT_REPORT = 1,
	HID_FEATURE_REPORT = 2,
	HID_REPORT_TYPES = 3,
};

struct hid_device {
	unsigned int id;
} __attribute__((preserve_access_index));

struct hid_bpf_ctx {
	struct hid_device *hid;
	__u32 allocated_size;
	union {
		__s32 retval;
		__s32 size;
	};
} __attribute__((preserve_access_index));

struct hid_bpf_ops {
	int hid_id;
	int (*hid_device_event)(struct hid_bpf_ctx *, enum hid_report_type, __u64);
	int (*hid_rdesc_fixup)(struct hid_bpf_ctx *);
} __attribute__((preserve_access_index));

// === Helpers ===

extern __u8 *hid_bpf_get_data(struct hid_bpf_ctx *ctx,
	unsigned int offset,
	const size_t __sz) SEC(".ksyms");

// === API ===

SEC(".rodata.uclogic_config")
struct uclogic_config {
	__u32 new_rdesc_size;
	__u8 new_rdesc[384];
} uclogic_config;

SEC("struct_ops/hid_device_event")
int uclogic_fix_event(unsigned long long *ctx)
{
	struct hid_bpf_ctx *hid_ctx = (struct hid_bpf_ctx *)ctx[0];
	enum hid_report_type rtype = (enum hid_report_type)ctx[1];

	if (rtype != HID_INPUT_REPORT)
		return 0;

	__u8 *data = hid_bpf_get_data(hid_ctx, 0, REPORT_SIZE);
	__s32 size = hid_ctx->size;

	if (!data || size < REPORT_SIZE)
		return 0;

	__u8 new_data[REPORT_SIZE];
	static struct state state;

	if (fixup_report(new_data, data, &state)) {
		__builtin_memcpy(data, new_data, REPORT_SIZE);
		return REPORT_SIZE;
	} else {
		return -1;
	}
}

SEC("struct_ops/hid_rdesc_fixup")
int uclogic_fix_rdesc(unsigned long long *ctx)
{
	struct hid_bpf_ctx *hid_ctx = (struct hid_bpf_ctx *)ctx[0];
	__u8 *data = hid_bpf_get_data(hid_ctx, 0, HID_MAX_DESCRIPTOR_SIZE);

	if (!data)
		return 0;

	__builtin_memcpy(data, uclogic_config.new_rdesc, sizeof(uclogic_config.new_rdesc));
	return uclogic_config.new_rdesc_size;
}

SEC(".struct_ops.link")
struct hid_bpf_ops uclogic_ops = {
	.hid_device_event = (void *)uclogic_fix_event,
	.hid_rdesc_fixup = (void *)uclogic_fix_rdesc,
};

// === Footer ===

char _license[] SEC("license") = "Dual BSD/GPL";

#endif // !TEST
