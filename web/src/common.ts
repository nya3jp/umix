export class Memory {
  readonly arrays: (DataView | undefined)[];
  readonly frees: number[];
  readonly regs: Uint32Array;

  constructor(program: DataView) {
    this.arrays = [program];
    this.frees = [];
    this.regs = new Uint32Array(8);
  }

  alloc(size: number): number {
    let id: number;
    if (this.frees.length > 0) {
      id = this.frees.pop()!;
    } else {
      id = this.arrays.length;
      this.arrays.push(undefined);
    }
    this.arrays[id] = new DataView(new ArrayBuffer(size * 4));
    return id;
  }

  free(id: number): void {
    this.arrays[id] = undefined;
    this.frees.push(id);
  }
}

export interface IO {
  read(): Promise<number>,
  write(value: number): Promise<void>,
}
