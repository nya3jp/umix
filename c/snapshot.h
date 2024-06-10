#ifndef SNAPSHOT_H
#define SNAPSHOT_H

#include <stdio.h>

int save_snapshot(FILE* fp);
int load_snapshot(FILE* fp);
int load_init_snapshot(void);

#endif // SNAPSHOT_H
