"""UMIX emulator in RPython.

UMIX is a virtual machine spec developed for ICFP Programming Contest 2006.
Home: http://www.boundvariable.org/
Spec: http://www.boundvariable.org/um-spec.txt
Benchmark binary: http://www.boundvariable.org/sandmark.umz

This emulator is written in RPython. It can run as a Python program with
(1) CPython or (2) PyPy, but also can be compiled into a static binary with
(3) RPython translator, even with (4) JIT support.

Currently, speed is (fastest) 4 > 3 > 2 >>> 1 (slowest).

A quick benchmark in Macbook Air mid-2013:

    ----------------------------
    CPython     ~7500s  ~290.00x
    PyPy          294s    11.31x
    RPython        33s     1.27x
    C (um.c)       26s     1.00x
    RPython+JIT    22s     0.84x
    ----------------------------

1. Run with CPython:
  $ python um.py sandmark.umz

2. Run with PyPy:
  $ pypy um.py sandmark.umz

3. Compile as RPython:
  $ rpython um.py
  $ ./um-c sandmark.umz

4. Compile as RPython as JIT:
  $ rpython --opt=jit um.py
  $ ./um-c sandmark.umz
  $ PYPYLOG=jit-log-opt:logfile ./um-c sandmark.umz    # Save JIT log
"""

import os
import sys

try:
  from rpython.rlib import jit
except ImportError:
  class jit(object):
    class JitDriver(object):
      def __init__(self, **kwargs): pass
      def jit_merge_point(self, **kwargs): pass
      def can_enter_jit(self, **kwargs): pass
    @staticmethod
    def elidable(f): return f
    @staticmethod
    def elidable_promote(**kwargs): return lambda x: x
    @staticmethod
    def promote(x): return x


class Memory(object):
  def __init__(self):
    self._pages = [None]
    self._unused_pages = []

  def get_page(self, index):
    return self._pages[index]

  def new_page(self, size):
    if self._unused_pages:
      index = self._unused_pages.pop()
    else:
      index = len(self._pages)
      self._pages.append(None)
    self._pages[index] = [0] * size
    return index

  def delete_page(self, index):
    self._pages[index] = None
    self._unused_pages.append(index)


def read_program(filename):
  fd = os.open(filename, os.O_RDONLY, 0777)
  data = os.read(fd, 1024*1024*1024)
  os.close(fd)
  return [(ord(data[i*4+0]) << 24) |
          (ord(data[i*4+1]) << 16) |
          (ord(data[i*4+2]) <<  8) |
          (ord(data[i*4+3]) <<  0)
          for i in range(len(data) / 4)]


def get_printable_location(pc, ver, program):
  instruction = program[pc]
  s = '(%d)@%d: ' % (ver, pc)
  op = instruction >> 28
  a = (instruction >> 6) & 7
  b = (instruction >> 3) & 7
  c = (instruction >> 0) & 7
  if op == 0:
    s += 'if (r%d) r%d = r%d' % (c, a, b)
  elif op == 1:
    s += 'r%d = mem[r%d][r%d]' % (a, b, c)
  elif op == 2:
    s += 'mem[r%d][r%d] = r%d' % (a, b, c)
  elif op == 3:
    s += 'r%d = r%d + r%d' % (a, b, c)
  elif op == 4:
    s += 'r%d = r%d * r%d' % (a, b, c)
  elif op == 5:
    s += 'r%d = r%d / r%d' % (a, b, c)
  elif op == 6:
    s += 'r%d = ~(r%d & r%d)' % (a, b, c)
  elif op == 7:
    s += 'halt'
  elif op == 8:
    s += 'r%d = malloc(r%d)' % (b, c)
  elif op == 9:
    s += 'free(r%d)' % c
  elif op == 10:
    s += 'put(r%d)' % c
  elif op == 11:
    s += 'r%d = get()' % c
  elif op == 12:
    s += 'jump(r%d, r%d)' % (b, c)
  elif op == 13:
    x = (instruction >> 25) & 7
    immediate = instruction & 0x1ffffff
    s += 'r%d = %d' % (x, immediate)
  else:
    s += '???'
  return s


jitdriver = jit.JitDriver(
    greens=['pc', 'ver', 'program'], reds=['ppc', 'registers', 'memory'],
    get_printable_location=get_printable_location)


@jit.elidable_promote()
def load_instruction(program, pc):
  return program[pc]


def run(program):
  memory = Memory()
  registers = [0, 0, 0, 0, 0, 0, 0, 0]
  pc = 0
  ver = 0
  ppc = -1

  while True:
    if pc < ppc:
      jitdriver.can_enter_jit(
          pc=pc, ver=ver, program=program,
          ppc=ppc, registers=registers, memory=memory)

    jitdriver.jit_merge_point(
        pc=pc, ver=ver, program=program,
        ppc=ppc, registers=registers, memory=memory)

    #print get_printable_location(pc, ver, program)

    # CAUTION!!!
    # Assuming that the user program does not perform self-rewriting.
    # Otherwise this optimization can lead to crash.
    instruction = load_instruction(program, pc)

    # Safer way.
    #instruction = program[pc]
    #jit.promote(instruction)

    ppc = pc
    pc += 1
    op = instruction >> 28
    a = (instruction >> 6) & 7
    b = (instruction >> 3) & 7
    c = (instruction >> 0) & 7

    if op == 0:
      if registers[c] != 0:
        registers[a] = registers[b]
    elif op == 1:
      if registers[b] == 0:
        registers[a] = program[registers[c]]
      else:
        registers[a] = memory.get_page(registers[b])[registers[c]]
    elif op == 2:
      if registers[a] == 0:
        program[registers[b]] = registers[c]
        # We can increment version here for safety, but it effectively disables
        # JIT because known UMIX programs frequently write to 0-array.
        #ver += 1
      else:
        memory.get_page(registers[a])[registers[b]] = registers[c]
    elif op == 3:
      registers[a] = (registers[b] + registers[c]) & 0xffffffff
    elif op == 4:
      registers[a] = (registers[b] * registers[c]) & 0xffffffff
    elif op == 5:
      registers[a] = (registers[b] / registers[c]) & 0xffffffff
    elif op == 6:
      registers[a] = (registers[b] & registers[c]) ^ 0xffffffff
    elif op == 7:
      break
    elif op == 8:
      registers[b] = memory.new_page(registers[c])
    elif op == 9:
      memory.delete_page(registers[c])
    elif op == 10:
      os.write(1, chr(registers[c]))
    elif op == 11:
      r = os.read(0, 1)
      registers[c] = ord(r[0]) if r else 0xffffffff
    elif op == 12:
      if registers[b] != 0:
        del program[:]
        program.extend(memory.get_page(registers[b]))
        ver += 1
      pc = registers[c]
    elif op == 13:
      x = (instruction >> 25) & 7
      immediate = instruction & 0x1ffffff
      registers[x] = immediate


def main(argv):
  if len(argv) != 2:
    print 'usage: %s foo.um' % argv[0]
    return 1
  program = read_program(argv[1])
  run(program)
  return 0


def jitpolicy(driver):
  from rpython.jit.codewriter.policy import JitPolicy
  return JitPolicy()


def target(*args):
  return main, None


if __name__ == '__main__':
  sys.exit(main(sys.argv))
