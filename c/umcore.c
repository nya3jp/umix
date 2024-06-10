#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "platter.h"
#include "array.h"
#include "io.h"

#include "umcore.h"

static unsigned long long um_insts = 0;

static platter_t regs[8];
static unsigned int pc = 0;


void umcore_run(void) {

    for(;;) {
        platter_t cmd = program[pc];
        unsigned int op = PLATTER_OP(cmd);
        um_insts++;
        if (op == 12) {
            replace_program(regs[PLATTER_REG_B(cmd)]);
            pc = regs[PLATTER_REG_C(cmd)];
        }
        else if (op == 13) {
            regs[PLATTER_IMMEDIATE_REG(cmd)] = PLATTER_IMMEDIATE_VALUE(cmd);
            pc++;
        }
        else {
            unsigned int reg_a = PLATTER_REG_A(cmd);
            unsigned int reg_b = PLATTER_REG_B(cmd);
            unsigned int reg_c = PLATTER_REG_C(cmd);
            switch(op) {
            case 0: // conditional move
                if (regs[reg_c] != 0)
                    regs[reg_a] = regs[reg_b];
                break;
            case 1: // array index
                regs[reg_a] = (get_array(regs[reg_b], 0))[regs[reg_c]];
                break;
            case 2: // array amendment
                (get_array(regs[reg_a], 1))[regs[reg_b]] = regs[reg_c];
                break;
            case 3: // addition
                regs[reg_a] = regs[reg_b] + regs[reg_c];
                break;
            case 4: // multiplication
                regs[reg_a] = regs[reg_b] * regs[reg_c];
                break;
            case 5: // division
                regs[reg_a] = regs[reg_b] / regs[reg_c];
                break;
            case 6: // not-and
                regs[reg_a] = ~(regs[reg_b] & regs[reg_c]);
                break;
            case 7: // halt
                goto HALT;
            case 8: // allocation
                regs[reg_b] = (platter_t)new_array(regs[reg_c]);
                break;
            case 9: // abandonment
                delete_array((array_t)regs[reg_c]);
                break;
            case 10: // output
                io_put(regs[reg_c]);
                break;
            case 11: // input
                regs[reg_c] = io_get();
                break;
            default:
                fprintf(stderr, "umcore_run: unknown command %08x\n", cmd);
                exit(1);
            }
            pc++;
        }
    }
HALT:;
}


void umcore_clear(void) {
    memset(regs, 0, sizeof(regs));
    pc = 0;
}

void umcore_save_snapshot(FILE* fp) {
    fwrite(&pc, sizeof(pc), 1, fp);
    fwrite(regs, sizeof(regs[0]), sizeof(regs)/sizeof(regs[0]), fp);
}

void umcore_load_snapshot(FILE* fp) {
    fread(&pc, sizeof(pc), 1, fp);
    fread(regs, sizeof(regs[0]), sizeof(regs)/sizeof(regs[0]), fp);
}

void umcore_stat(void) {
    printf("module umcore:\n"
           "\texecuted instructions: %llu\n", 
           um_insts);
}

