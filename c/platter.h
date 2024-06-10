#ifndef PLATTER_H
#define PLATTER_H

typedef unsigned int platter_t;

#define PLATTER_OP(p) ((p) >> 28)
#define PLATTER_REG_A(p) (((p) >> 6) & 0x7)
#define PLATTER_REG_B(p) (((p) >> 3) & 0x7)
#define PLATTER_REG_C(p) ((p) & 0x7)
#define PLATTER_IMMEDIATE_VALUE(p) ((p) & 0x01ffffff)
#define PLATTER_IMMEDIATE_REG(p) (((p) >> 25) & 0x7)


#endif // PLATTER_H
