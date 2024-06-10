#include <unistd.h>
#include <stdlib.h>

#include "args.h"

FILE* umfile = NULL;
FILE* umxfile = NULL;

void parse_opts(int argc, char** argv) {
    int opt;

    while((opt = getopt(argc, argv, "f:s:")) != -1) {
        switch(opt) {
        case 'f':
            if (umfile) {
                fprintf(stderr, "multiple -f option!\n");
                exit(1);
            }
            umfile = fopen(optarg, "r");
            if (!umfile) {
                perror("opening um");
                exit(1);
            }
            break;
        case 's':
            if (umxfile) {
                fprintf(stderr, "multiple -s option!\n");
                exit(1);
            }
            umxfile = fopen(optarg, "r");
            if (!umxfile) {
                perror("opening umx");
                exit(1);
            }
            break;
        default:
            fprintf(stderr, "unknown option: -%c\n", (char)opt);
            exit(1);
        }
    }

    argc -= optind;
    argv += optind;
    if (argc > 0) {
        fprintf(stderr, "unknown argument: %s\n", argv[0]);
        exit(1);
    }

    if (!umfile) {
        umfile = fopen("umix.um", "r");
        if (!umfile) {
            perror("opening um");
            exit(1);
        }
    }
}

