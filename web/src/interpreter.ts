import type { IO, Memory } from "./common";

export async function runInterpreter(memory: Memory, io: IO): Promise<void> {
  for (let pc = 0;;) {
    const inst = memory.arrays[0]!.getUint32(pc * 4);
    const op = inst >>> 28;
    switch (op) {
      case 0:
        if (memory.regs[inst & 7] !== 0) {
          memory.regs[(inst >>> 6) & 7] = memory.regs[(inst >>> 3) & 7];
        }
        pc += 1;
        break;
      case 1:
        memory.regs[(inst >>> 6) & 7] = memory.arrays[memory.regs[(inst >>> 3) & 7]]!.getUint32(memory.regs[inst & 7] << 2);
        pc += 1;
        break;
      case 2:
        memory.arrays[memory.regs[(inst >>> 6) & 7]]!.setUint32(memory.regs[(inst >>> 3) & 7] << 2, memory.regs[inst & 7]);
        pc += 1;
        break;
      case 3:
        memory.regs[(inst >>> 6) & 7] = (memory.regs[(inst >>> 3) & 7] + memory.regs[inst & 7]) >>> 0;
        pc += 1;
        break;
      case 4:
        memory.regs[(inst >>> 6) & 7] = Math.imul(memory.regs[(inst >>> 3) & 7], memory.regs[inst & 7]) >>> 0;
        pc += 1;
        break;
      case 5:
        memory.regs[(inst >>> 6) & 7] = (memory.regs[(inst >>> 3) & 7] / memory.regs[inst & 7]) >>> 0;
        pc += 1;
        break;
      case 6:
        memory.regs[(inst >>> 6) & 7] = (~(memory.regs[(inst >>> 3) & 7] & memory.regs[inst & 7])) >>> 0;
        pc += 1;
        break;
      case 7:
        return;
      case 8: {
        const size = memory.regs[inst & 7];
        const id = memory.alloc(size);
        memory.regs[(inst >>> 3) & 7] = id;
        pc += 1;
        break;
      }
      case 9: {
        const id = memory.regs[inst & 7];
        memory.free(id);
        pc += 1;
        break;
      }
      case 10: {
        const value = memory.regs[inst & 7];
        await io.write(value);
        pc += 1;
        break;
      }
      case 11:
        memory.regs[inst & 7] = await io.read();
        pc += 1;
        break;
      case 12: {
        const id = memory.regs[(inst >>> 3) & 7];
        const newPc = memory.regs[inst & 7];
        if (id !== 0) {
          const array = memory.arrays[id]!;
          memory.arrays[0] = new DataView(array.buffer.slice(0));
        }
        pc = newPc;
        break;
      }
      case 13:
        memory.regs[(inst >>> 25) & 7] = inst & 0x1ffffff;
        pc += 1;
        break;
      default:
        throw new Error(`Invalid opcode: ${op}`);
    }
  }
}
