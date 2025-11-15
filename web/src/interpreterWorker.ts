import { Memory, type IO } from './common';
import { runInterpreter } from './interpreter';
import type { MainToWorkerMessage, WorkerToMainMessage } from './interpreterMessages';

const ctx: DedicatedWorkerGlobalScope = self as DedicatedWorkerGlobalScope;

function postMessageToMain(message: WorkerToMainMessage): void {
  ctx.postMessage(message);
}

class MessageIO implements IO {
  private readonly buffer: number[] = [];
  private readonly pendingReaders: ((value: number) => void)[] = [];

  async read(): Promise<number> {
    if (this.buffer.length > 0) {
      return this.buffer.shift()!;
    }
    return await new Promise<number>((resolve) => {
      this.pendingReaders.push(resolve);
      postMessageToMain({ type: 'read' });
    });
  }

  async write(value: number): Promise<void> {
    postMessageToMain({ type: 'write', value });
  }

  enqueueInput(value: number): void {
    if (this.pendingReaders.length > 0) {
      const resolve = this.pendingReaders.shift()!;
      resolve(value);
    } else {
      this.buffer.push(value);
    }
  }
};

const io = new MessageIO();

function start(program: ArrayBuffer): void {
  const memory = new Memory(new DataView(program));

  void runInterpreter(memory, io).then(() => {
    postMessageToMain({ type: 'exit' });
  }, (error) => {
    const err = error instanceof Error ? error : new Error(String(error));
    postMessageToMain({ type: 'error', message: err.message, stack: err.stack });
  });
}

ctx.onmessage = (event: MessageEvent<MainToWorkerMessage>) => {
  const data = event.data;
  switch (data.type) {
    case 'start':
      start(data.program);
      break;
    case 'input':
      io.enqueueInput(data.value);
      break;
  }
};
