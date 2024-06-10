#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "platter.h"
#include "umem.h"
#include "array.h"


/*
 * internal structure of array
 */

struct array {
    platter_t* head;
    int length;
};



/*
 * array variables
 */

static struct array* ar_list = NULL;
static array_t* ar_free = NULL;
static array_t ar_cap = 0;
static array_t ar_fsize = 0;
static array_t nr_cow = 0;
static int ar_loads = 0;
static int ar_cow_brks = 0;


/*
 * functions
 */

static void extend_array_list(void) {
    int ar_new;
    int i;

    ar_new = (ar_cap == 0 ? 1 : ar_cap);
    ar_list = (struct array*)realloc(ar_list, sizeof(struct array)*(ar_cap+ar_new));
    ar_free = (array_t*)realloc(ar_free, sizeof(array_t)*(ar_cap+ar_new));

    for(i = 0; i < ar_new; i++)
        ar_free[ar_fsize+i] = (array_t)(ar_cap+ar_new-1-i);
    memset(ar_list+ar_cap, 0, sizeof(struct array)*ar_new);

    ar_cap += ar_new;
    ar_fsize += ar_new;
}

static void break_cow(void) {
    struct array* ar_copy;

    ar_copy = &ar_list[nr_cow];
    umem_dup(&ar_copy->head, &ar_copy->length, ar_copy->head, ar_copy->length);
    nr_cow = 0;
    ar_cow_brks++;
}


array_t new_array(int size) {
    array_t nr_array;
    struct array* ar;

    if (ar_fsize == 0)
        extend_array_list();

    nr_array = ar_free[--ar_fsize];
    ar = &ar_list[nr_array];
    umem_alloc(&ar->head, &ar->length, size);
    return nr_array;
}

void delete_array(array_t nr_array) {
    struct array* ar;

    if (nr_array == nr_cow) {
        nr_cow = 0;
    }
    else {
        ar = &ar_list[nr_array];
        umem_free(&ar->head, &ar->length);
        ar_free[ar_fsize++] = nr_array;
    }
}

void replace_program(array_t nr_array) {
    struct array* ar_program;
    struct array* ar_copy;

    if (nr_array == 0)
        return;
    ar_program = &ar_list[0];
    ar_copy = &ar_list[nr_array];
    ar_program->head = ar_copy->head;
    ar_program->length = ar_copy->length;
    nr_cow = nr_array;
    ar_loads++;
}

platter_t* get_array(array_t nr_array, int write) {
    if (write && nr_cow != 0 && (nr_array == 0 || nr_array == nr_cow))
        break_cow();
    return ar_list[nr_array].head;
}

int get_array_length(array_t nr_array) {
    return ar_list[nr_array].length;
}

void array_clear(void) {
    struct array* ar;
    if (nr_cow != 0) {
        ar = &ar_list[0];
        ar->head = NULL;
        ar->length = 0;
    }
    for(ar = ar_list; ar < ar_list+ar_cap; ar++)
        umem_free(&ar->head, &ar->length);
    free(ar_list);
    ar_list = NULL;
    free(ar_free);
    ar_free = NULL;
    ar_cap = 0;
    ar_fsize = 0;
    nr_cow = 0;
    ar_loads = 0;
    ar_cow_brks = 0;
}

void array_save_snapshot(FILE* fp) {
    struct array* ar;
    fwrite(&ar_cap, sizeof(int), 1, fp);
    for(ar = ar_list; ar < ar_list+ar_cap; ar++) {
        if (!ar->head) {
            int none = -1;
            fwrite(&none, sizeof(int), 1, fp);
        }
        else {
            fwrite(&ar->length, sizeof(int), 1, fp);
            fwrite(ar->head, sizeof(platter_t), ar->length, fp);
        }
    }
}

void array_load_snapshot(FILE* fp) {
    array_t nr_array;

    array_clear();
    fread(&ar_cap, sizeof(int), 1, fp);
    ar_list = (struct array*)calloc(sizeof(struct array), ar_cap);
    ar_free = (array_t*)malloc(sizeof(array_t)*ar_cap);
    for(nr_array = 0; nr_array < ar_cap; nr_array++) {
        struct array* ar;
        int length;
        fread(&length, sizeof(int), 1, fp);
        if (length == -1) {
            ar_free[ar_fsize++] = nr_array;
        }
        else {
            ar = &ar_list[nr_array];
            umem_alloc(&ar->head, &ar->length, length);
            fread(ar->head, sizeof(platter_t), length, fp);
        }
    }
}

void array_stat(void) {
    printf("module array:\n"
           "\ttotal reserved arrays: %d\n"
           "\ttotal active arrays:   %d\n"
           "\ttotal inactive arrays: %d\n"
           "\tnon-trivial loads:     %d\n"
           "\tcopy-on-write breaks:  %d\n",
           ar_cap, ar_cap-ar_fsize, ar_fsize, ar_loads, ar_cow_brks);
}



