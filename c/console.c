#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

#include "array.h"
#include "umem.h"
#include "umcore.h"
#include "io.h"
#include "snapshot.h"
#include "screen.h"

#include "console.h"

#define SNAPSHOT_DEFAULT_FILENAME "snapshot.umx"


static void command_stat(void) {
    array_stat();
    umem_stat();
    umcore_stat();
}

static void command_save(char* filename) {
    FILE* fp;
    if (!filename)
        filename = SNAPSHOT_DEFAULT_FILENAME;
    fp = fopen(filename, "w");
    if (!fp) {
        perror("opening file");
        return;
    }
    if (save_snapshot(fp) == 0)
        printf("saved to %s, %ld bytes.\n", filename, ftell(fp));
    fclose(fp);
}

static void command_load(char* filename) {
    FILE* fp;
    if (!filename)
        filename = SNAPSHOT_DEFAULT_FILENAME;
    fp = fopen(filename, "r");
    if (!fp) {
        perror("opening file");
        return;
    }
    if (load_snapshot(fp) == 0)
        printf("loaded from %s, %ld bytes.\n", filename, ftell(fp));
    fclose(fp);
}

static void command_send(char* filename) {
    FILE* fp;
    char buf[1024];
    int len;

    if (!filename) {
        printf("no filename specified!\n");
        return;
    }
    fp = fopen(filename, "r");
    if (!fp) {
        perror("opening file");
        return;
    }

    for(;;) {
        len = fread(buf, 1, sizeof(buf), fp);
        if (len == 0)
            break;
        io_feed_paste(buf, len);
    }
    fclose(fp);
}

static int parse_command(char* cmd) {
    if (strcmp(cmd, "stat") == 0) {
        command_stat();
    }
    else if (strcmp(cmd, "save") == 0) {
        command_save(strtok(NULL, ""));
    }
    else if (strcmp(cmd, "load") == 0) {
        command_load(strtok(NULL, ""));
        return 1;
    }
    else if (strcmp(cmd, "send") == 0) {
        command_send(strtok(NULL, ""));
    }
    else if (strcmp(cmd, "halt") == 0 || strcmp(cmd, "quit") == 0 || strcmp(cmd, "q") == 0) {
        exit(0);
    }
    else if (strcmp(cmd, "exit") == 0 || strcmp(cmd, "x") == 0) {
        return -1;
    }
    else {
        printf("unknown command: %s\n", cmd);
    }
    return 0;
}

static int readline(char* cmdline, int size) {
    if (!fgets(cmdline, size, stdin))
        return -1;

    size = strlen(cmdline);
    while(size > 0 && isspace(cmdline[size-1]))
        size--;
    cmdline[size] = '\0';
    return 0;
}

void console_enter(void) {
    char cmdline[256];
    char* cmd;

    if (readline(cmdline, sizeof(cmdline)) == -1)
        return;
    cmd = strtok(cmdline, " ");
    if (cmd) {
        if (parse_command(cmd) == 1) {
            screen_reset();
            io_print_backlog();
        }
        return;
    }

    screen_reset();
    for(;;) {
        printf("um> ");
        fflush(stdout);
        if (readline(cmdline, sizeof(cmdline)) == -1)
            return;
        cmd = strtok(cmdline, " ");
        if (!cmd)
            continue;
        if (parse_command(cmd) == -1)
            break;
    }

    screen_reset();
    io_print_backlog();
}


