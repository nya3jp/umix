import { Terminal } from '@xterm/xterm';
import { ClipboardAddon } from '@xterm/addon-clipboard';
import { FitAddon } from '@xterm/addon-fit';
import { openpty } from 'xterm-pty';
import { Memory } from './common';
import { run } from './interpreter';
import './style.css';

async function loadUmixCodex(): Promise<DataView> {
  const response = await fetch('/umix/codex/umix.um');
  const buffer = await response.arrayBuffer();
  return new DataView(buffer);
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

async function main() {
  const terminal = initUi();

  const {master, slave} = openpty();
  terminal.loadAddon(master);

  const codex = await loadUmixCodex();

  const memory = new Memory(codex);
  const io = new PtyIO(slave);

  await run(memory, io);

  slave.write('\n--- Session terminated ---\n');
}

main().catch((e) => {
  console.error(e);
});
