// SPDX-License-Identifier: GPL-2.0-only OR BSD-2-Clause

/*

Usage:

```c
#define DESCRIPTOR_FILE "uclogic_v2.in.h"
#define DESCRIPTOR_NAME rdesc_uclogic_v2
#include "mk_template.h"

// DESCRIPTOR_FILE and DESCRIPTOR_NAME are #undef'd for convenience
```

Given a template.in.h like this:

```c
1, 2, 3, FIELD(__u32, value), 4, 5,
```

We include this file twice, each time with different macros, to get:

```c
struct template {
	__u8 _bytes[sizeof (__u8[]){ 1, 2, 3, }];
	__u32 value;
	__u8 _bytes_after_value[sizeof (__u8[]){ 4, 5, }];
} __attribute__((packed)) template = {
	._bytes = { 1, 2, 3, },
	.value = (__u32)-1, // Placeholder value
	._bytes_after_value = { 4, 5, }
};
```

To use this template:

```c
struct template foo = template;
foo.value = 42;

// Copy to some buffer
memcpy(buf, &foo, sizeof(foo));
```

*/

static const struct DESCRIPTOR_NAME {
	__u8 _bytes[sizeof (__u8[]){

#define FIELD(_type, _name) \
	}]; \
	_type _name; \
	__u8 _bytes_after_##_name[sizeof (__u8[]){

#include DESCRIPTOR_FILE

	}];
} __attribute__((packed)) DESCRIPTOR_NAME = {
	._bytes = {

#undef FIELD
#define FIELD(_type, _name) \
	}, ._name = (_type)-1, ._bytes_after_##_name = {

#include DESCRIPTOR_FILE

	}
};


#undef FIELD
#undef DESCRIPTOR_NAME
#undef DESCRIPTOR_FILE
