#include <stdio.h>
#include <string.h>

#include "umcore.h"
#include "array.h"
#include "io.h"
#include "args.h"

#define MAGIC "UMX\x01"


int save_snapshot(FILE* fp) {
    fwrite(MAGIC, 1, 4, fp);
    umcore_save_snapshot(fp);
    array_save_snapshot(fp);
    io_save_snapshot(fp);
    return 0;
}

int load_snapshot(FILE* fp) {
    char magic[5];
    fread(magic, 1, 4, fp);
    magic[4] = '\0';
    if (strcmp(magic, MAGIC) != 0) {
        printf("corrupted snapshot!\n");
        return -1;
    }
    umcore_load_snapshot(fp);
    array_load_snapshot(fp);
    io_load_snapshot(fp);
    return 0;
}

int load_init_snapshot(void) {
    if (!umxfile)
        return 0;
    if (load_snapshot(umxfile) == -1)
        return -1;
    fclose(umxfile);
    io_print_backlog();
    return 0;
}

