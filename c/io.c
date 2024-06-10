#include <stdio.h>
#include <string.h>

#include "console.h"
#include "io.h"

#define ESCAPE_CHAR '!'

#define BACKLOG_CAPACITY 4096
#define PASTE_CAPACITY (1024*1024)

#define min(a,b) ((a) < (b) ? (a) : (b))

static char backlog[BACKLOG_CAPACITY];
static int backlog_offset;
static char paste[PASTE_CAPACITY];
static int paste_offset;
static int paste_size;


static void feed_backlog(int c) {
    backlog[backlog_offset++] = c;
    if (backlog_offset == BACKLOG_CAPACITY)
        backlog_offset = 0;
}

static int shift_paste(void) {
    int c;
    c = (int)paste[paste_offset++] & 0xff;
    if (paste_offset == PASTE_CAPACITY)
        paste_offset = 0;
    paste_size--;
    return c;
}

void io_feed_paste(char* buf, int size) {
    size = min(size, PASTE_CAPACITY-paste_size);
    while(size > 0) {
        int block_size;
        int paste_tail;
        paste_tail = (paste_offset+paste_size)%PASTE_CAPACITY;
        block_size = min(size, PASTE_CAPACITY-paste_tail);
        memcpy(paste+paste_tail, buf, block_size);
        buf += block_size;
        size -= block_size;
        paste_size += block_size;
    }
}

void io_put(int c) {
    putchar(c);
    feed_backlog(c);
}

int io_get(void) {
    int c;
    do {
        if (paste_size > 0) {
            c = shift_paste();
            putchar(c);
            break;
        }
        fflush(stdout);
        c = getchar();
        if (c == ESCAPE_CHAR)
            console_enter();
    } while(c == ESCAPE_CHAR);
    feed_backlog(c);
    return c;
}

void io_print_backlog(void) {
    fwrite(backlog+backlog_offset, 1, BACKLOG_CAPACITY-backlog_offset, stdout);
    fwrite(backlog, 1, backlog_offset, stdout);
}

void io_clear(void) {
    memset(backlog, 0, sizeof(backlog));
    backlog_offset = 0;
    memset(paste, 0, sizeof(paste));
    paste_offset = 0;
    paste_size = 0;
}

void io_save_snapshot(FILE* fp) {
    fwrite(backlog+backlog_offset, 1, BACKLOG_CAPACITY-backlog_offset, fp);
    fwrite(backlog, 1, backlog_offset, fp);
    fwrite(&paste_offset, sizeof(paste_offset), 1, fp);
    fwrite(&paste_size, sizeof(paste_size), 1, fp);
    fwrite(paste, 1, PASTE_CAPACITY, fp);
}

void io_load_snapshot(FILE* fp) {
    io_clear();
    fread(backlog, sizeof(backlog[0]), BACKLOG_CAPACITY, fp);
    backlog_offset = 0;
    fread(&paste_offset, sizeof(paste_offset), 1, fp);
    fread(&paste_size, sizeof(paste_size), 1, fp);
    fread(paste, 1, PASTE_CAPACITY, fp);
}


