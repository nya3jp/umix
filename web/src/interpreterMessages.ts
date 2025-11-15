export interface StartMessage {
  type: 'start',
  program: ArrayBuffer,
}

export interface InputMessage {
  type: 'input',
  value: number,
}

export type MainToWorkerMessage = StartMessage | InputMessage;

export interface ReadMessage {
  type: 'read',
}

export interface WriteMessage {
  type: 'write',
  value: number,
}

export interface ExitMessage {
  type: 'exit',
}

export interface ErrorMessage {
  type: 'error',
  message: string,
  stack?: string,
}

export type WorkerToMainMessage = ReadMessage | WriteMessage | ExitMessage | ErrorMessage;
