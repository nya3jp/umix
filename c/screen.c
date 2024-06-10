#include <stdio.h>

#include "screen.h"


void screen_reset(void) {
    printf("\033c");
}
