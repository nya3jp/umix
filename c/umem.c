#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "platter.h"
#include "umem.h"

static int umem_used = 0;

void umem_alloc(platter_t** phead, int* pcap, int size) {
    *phead = (platter_t*)calloc(size, sizeof(platter_t));
    *pcap = size;
    umem_used += size;
}

void umem_free(platter_t** phead, int* pcap) {
    umem_used -= *pcap;
    free(*phead);
    *phead = NULL;
    *pcap = 0;
}

void umem_dup(platter_t** phead_dst, int* pcap_dst, platter_t* head_src, int cap_src) {
    umem_alloc(phead_dst, pcap_dst, cap_src);
    memcpy(*phead_dst, head_src, cap_src);
}

void umem_stat(void) {
    printf("module umem:\n"
           "\ttotal allocated platters: %d\n",
           umem_used);
}

