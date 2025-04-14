// SPDX-License-Identifier: GPL-2.0-only

#define DescriptorTemplate_begin \
        static const struct DESCRIPTOR_NAME { \
                __u8 _bytes[sizeof (__u8[]){

#define FIELD(_type, _name) \
                }]; \
                _type _name; \
                __u8 _bytes_after_##_name[sizeof (__u8[]){

#define DescriptorTemplate_end \
                }]; \
        } __attribute__((packed)) DESCRIPTOR_NAME =

#define DescriptorTemplate(...) \
        DescriptorTemplate_begin \
        __VA_ARGS__ \
        DescriptorTemplate_end

#include DESCRIPTOR_FILE

#undef DescriptorTemplate_begin
#undef DescriptorTemplate_end

#undef DescriptorTemplate
#undef FIELD

#define DescriptorTemplate(...) \
        { ._bytes = { __VA_ARGS__ } };
#define FIELD(_type, _name) \
        }, ._name = (_type)-1, ._bytes_after_##_name = {

#include DESCRIPTOR_FILE

#undef DescriptorTemplate
#undef FIELD

#undef DESCRIPTOR_NAME
#undef DESCRIPTOR_FILE
