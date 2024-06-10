#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef unsigned int platter_t;
#define PLATTER_OP(p) ((p) >> 28)
#define PLATTER_REG_A(p) (((p) >> 6) & 0x7)
#define PLATTER_REG_B(p) (((p) >> 3) & 0x7)
#define PLATTER_REG_C(p) ((p) & 0x7)
#define PLATTER_IMMEDIATE_VALUE(p) ((p) & 0x01ffffff)
#define PLATTER_IMMEDIATE_REG(p) (((p) >> 25) & 0x7)

typedef struct {
  platter_t* data;
  unsigned int size;
} page_t;

page_t* memory[65536];
int memory_size = 1;
int free_pages[65536];
int num_free_pages = 0;

page_t* read_page(char* filename) {
  FILE* fp = fopen(filename, "r");
  page_t* page = malloc(sizeof(page_t));
  int n;
  int i;
  fseek(fp, 0, SEEK_END);
  n = ftell(fp) / sizeof(platter_t);
  fseek(fp, 0, SEEK_SET);
  page->size = n;
  page->data = calloc(n, sizeof(platter_t));
  fread(page->data, sizeof(platter_t), n, fp);
  fclose(fp);
  for (i = 0; i < n; ++i) {
    page->data[i] = (page->data[i] >> 16) | (page->data[i] << 16);
    page->data[i] = (((page->data[i] << 8) & 0xff00ff00) |
                     ((page->data[i] >> 8) & 0x00ff00ff));
  }
  return page;
}

int main(int argc, char** argv) {
  int pc = 0;
  platter_t regs[8] = {0, 0, 0, 0, 0, 0, 0, 0};
  page_t* program = read_page(argv[1]);
  memory[0] = program;

  for(;;) {
    platter_t cmd = program->data[pc];
    unsigned int op = PLATTER_OP(cmd);
    if (op == 12) {
      unsigned int m = regs[PLATTER_REG_B(cmd)];
      if (m != 0) {
        free(program->data);
        program->data = calloc(memory[m]->size, sizeof(platter_t));
        memcpy(program->data, memory[m]->data, sizeof(platter_t) * memory[m]->size);
        program->size = memory[m]->size;
      }
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
          regs[reg_a] = memory[regs[reg_b]]->data[regs[reg_c]];
          break;
        case 2: // array amendment
          memory[regs[reg_a]]->data[regs[reg_b]] = regs[reg_c];
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
          {
            int n = regs[reg_c];
            page_t* page = malloc(sizeof(page_t));
            int m;
            if (num_free_pages == 0) {
              m = memory_size++;
            } else {
              m = free_pages[--num_free_pages];
            }
            page->data = calloc(n, sizeof(platter_t));
            page->size = n;
            memory[m] = page;
            regs[reg_b] = m;
          }
          break;
        case 9: // abandonment
          {
            int m = regs[reg_c];
            free(memory[m]->data);
            free(memory[m]);
            free_pages[num_free_pages++] = m;
          }
          break;
        case 10: // output
          fputc(regs[reg_c], stdout);
          break;
        case 11: // input
          regs[reg_c] = fgetc(stdin);
          break;
        default:
          fprintf(stderr, "umcore_run: unknown command %08x\n", cmd);
          exit(1);
      }
      pc++;
    }
  }
HALT:
  return 0;
}
