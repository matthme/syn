export interface SynConfig {
  hearbeatInterval: number;
  requestTimeout: number;
  debounceInterval: number;
  outOfSessionTimeout: number;
  syncStrategy: SyncStrategy;
  commitStrategy: CommitStrategy;
}

export enum SyncStrategy {
  CRDT,
  BlockOnConflict,
  DropOnConflict,
}

// Or both
export interface CommitStrategy {
  CommitEveryNMs: number | undefined;
  CommitEveryNDeltas: number | undefined;
}

export function defaultConfig(): SynConfig {
  return {
    hearbeatInterval: 2 * 1000,
    outOfSessionTimeout: 8 * 1000,
    debounceInterval: 500,
    requestTimeout: 4000,
    commitStrategy: { CommitEveryNDeltas: 200, CommitEveryNMs: 1000 * 30 }, // TODO: reduce ms
    syncStrategy: SyncStrategy.BlockOnConflict,
  };
}

export type RecursivePartial<T> = {
  [P in keyof T]?: T[P] extends (infer U)[]
    ? RecursivePartial<U>[]
    : T[P] extends object
    ? RecursivePartial<T[P]>
    : T[P];
};
