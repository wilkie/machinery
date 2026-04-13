import type { Operation } from './Operation';

export interface MacrosInfo {
  [key: string]: string | number | Operation;
}
