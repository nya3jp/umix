#ifndef UMEM_H
#define UMEM_H

#include "platter.h"

void umem_alloc(platter_t** phead, int* pcap, int size);
void umem_free(platter_t** phead, int* pcap);
void umem_dup(platter_t** phead_dst, int* pcap_dst, platter_t* head_src, int cap_src);

void umem_stat(void);

#endif // UMEM_H
