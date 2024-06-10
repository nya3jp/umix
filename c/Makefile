EXE := um
CC := gcc
OBJS := args.o array.o main.o scroll.o umcore.o umem.o io.o console.o snapshot.o screen.o
CFLAGS := -Wall -Wextra -g -O2

all: $(EXE)

$(EXE):	$(OBJS)
	$(CC) -o $@ $(CFLAGS) $(LDFLAGS) $^

.c.o:
	$(CC) -c $(CFLAGS) $^

clean:
	rm -f $(EXE) *.o

.PHONY: all clean
