#ifndef IO_H
#define IO_H

#include <stdio.h>

void io_put(int c);
int io_get(void);

void io_print_backlog(void);
void io_clear(void);
void io_save_snapshot(FILE* fp);
void io_load_snapshot(FILE* fp);
void io_feed_paste(char* buf, int size);


#endif // IO_H
