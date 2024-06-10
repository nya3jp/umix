#ifndef ARRAY_H
#define ARRAY_H

#include <stdio.h>

#include "platter.h"

typedef unsigned int array_t;

array_t new_array(int size);
void delete_array(array_t nr_array);
void replace_program(array_t nr_array);
platter_t* get_array(array_t nr_array, int write);
int get_array_length(array_t nr_array);
void array_clear(void);
void array_save_snapshot(FILE* fp);
void array_load_snapshot(FILE* fp);

void array_stat(void);

#define program (get_array(0, 0))


#endif // ARRAY_H
