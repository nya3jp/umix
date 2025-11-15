import { Terminal } from '@xterm/xterm';
import { ClipboardAddon } from '@xterm/addon-clipboard';
import { FitAddon } from '@xterm/addon-fit';
import { openpty } from 'xterm-pty';
import type { WorkerToMainMessage } from './interpreterMessages';
import './style.css';

async function loadUmixCodex(): Promise<ArrayBuffer> {
  const response = await fetch('/umix/codex/umix.um');
  return await response.arrayBuffer();
}

interface Slave {
  onReadable(listener: () => void): void,
  read(length?: number): number[],
  write(arg: string | number[]): void,
}

class PtyIO {
  private readonly slave: Slave;
  private readonly inputBuffer: number[] = [];
  private inputPromise: Promise<void>;
  private inputResolve: () => void;

  constructor(slave: Slave) {
    this.slave = slave;

    this.inputResolve = () => {};  // just to make the compiler happy
    this.inputPromise = new Promise((resolve) => {
      this.inputResolve = resolve;
    });

    slave.onReadable(() => {
      for (const value of slave.read()) {
        this.inputBuffer.push(value);
      }
      this.inputResolve();
      this.inputPromise = new Promise((resolve) => {
        this.inputResolve = resolve;
      });
    });
  }

  async read(): Promise<number> {
    while (this.inputBuffer.length === 0) {
      await this.inputPromise;
    }
    return this.inputBuffer.shift()!;
  }

  async write(value: number): Promise<void> {
    this.slave.write(String.fromCharCode(value));
  }
}

function initUi(): Terminal {
  const terminal = new Terminal();

  terminal.loadAddon(new ClipboardAddon());

  const fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);

  const element = document.getElementById('terminal')!;
  terminal.open(element);

  const observer = new ResizeObserver(() => {
    fitAddon.fit();
  });
  observer.observe(element);
  fitAddon.fit();

  terminal.focus();
  return terminal;
}

async function runWorker(codex: ArrayBuffer, io: PtyIO, slave: Slave): Promise<void> {
  const worker = new Worker(new URL('./interpreterWorker.ts', import.meta.url), {type: 'module'});

  const workerCompletion = new Promise<void>((resolve, reject) => {
    worker.addEventListener('message', (event: MessageEvent<WorkerToMainMessage>) => {
      const data = event.data;
      switch (data.type) {
        case 'write':
          void io.write(data.value).catch((error) => {
            console.error('Failed to write to terminal', error);
          });
          break;
        case 'read':
          void io.read().then((value) => {
            worker.postMessage({type: 'input', value});
          }).catch((error) => {
            reject(error);
            worker.terminate();
          });
          break;
        case 'exit':
          slave.write('\n--- Session terminated ---\n');
          resolve();
          worker.terminate();
          break;
        case 'error': {
          const err = new Error(data.message);
          if (data.stack) {
            err.stack = data.stack;
          }
          slave.write(`\n--- Interpreter error: ${data.message} ---\n`);
          reject(err);
          worker.terminate();
          break;
        }
      }
    });

    worker.addEventListener('error', (event) => {
      const error = event.error ?? new Error(`Worker error: ${event.message}`);
      reject(error);
      worker.terminate();
    });
  });

  worker.postMessage({type: 'start', program: codex}, [codex]);

  await workerCompletion;
}

async function main() {
  const terminal = initUi();

  const {master, slave} = openpty();
  terminal.loadAddon(master);

  const codex = await loadUmixCodex();
  const io = new PtyIO(slave);

  await runWorker(codex, io, slave);
}

main().catch((e) => {
  console.error(e);
});
