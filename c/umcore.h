#ifndef UMCORE_H
#define UMCORE_H

#include <stdio.h>

void umcore_run(void);
void umcore_translate(FILE* fp, unsigned int begin, unsigned int end);
void umcore_clear(void);
void umcore_save_snapshot(FILE* fp);
void umcore_load_snapshot(FILE* fp);

void umcore_stat(void);


#endif // UMCORE_H
