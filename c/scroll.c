#include <stdio.h>
#include <stdlib.h>

#include "platter.h"
#include "array.h"
#include "args.h"


void load_scroll(void) {
    long size;
    long offset;
    array_t nr_array;
    platter_t* head;

    if (fseek(umfile, 0, SEEK_END) == -1) {
        perror("fseek");
        exit(1);
    }
    size = ftell(umfile);
    if (size == -1) {
        perror("ftell");
        exit(1);
    }
    if (fseek(umfile, 0, SEEK_SET) == -1) {
        perror("fseek");
        exit(1);
    }
    size /= sizeof(platter_t);

    nr_array = new_array(size);
    head = get_array(nr_array, 1);

    fread(head, sizeof(platter_t), size, umfile);
    for(offset = 0; offset < size; offset++) {
        head[offset] = (head[offset] >> 16) | (head[offset] << 16);
        head[offset] = ((head[offset] >> 8) & 0x00ff00ff) | ((head[offset] << 8) & 0xff00ff00);
    }

    fclose(umfile);

    replace_program(nr_array);
}
