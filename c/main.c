#include "args.h"
#include "array.h"
#include "umcore.h"
#include "io.h"
#include "scroll.h"
#include "screen.h"
#include "snapshot.h"


static void all_clear(void) {
    umcore_clear();
    array_clear();
    io_clear();
}

int main(int argc, char** argv) {
    all_clear();
    parse_opts(argc, argv);
    load_scroll();
    screen_reset();
    if (load_init_snapshot() == -1)
        return 1;
    umcore_run();
    return 0;
}
