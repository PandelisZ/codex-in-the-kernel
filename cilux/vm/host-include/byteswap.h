#ifndef _CILUX_HOST_BYTESWAP_H
#define _CILUX_HOST_BYTESWAP_H

#include <stdint.h>

#define bswap_16(x) __builtin_bswap16(x)
#define bswap_32(x) __builtin_bswap32(x)
#define bswap_64(x) __builtin_bswap64(x)

#endif
