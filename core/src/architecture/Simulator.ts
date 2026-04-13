import Video from './Video';
import Serial from './Serial';

import type { RegisterInfo } from './Register';

export const SimulatorStates = {
  Uninitialized: 0,
  Initialized: 1,
  Running: 2,
  Paused: 3,
  Stopped: 4,
  Crashed: 5,
} as const;

export type SimulatorState =
  (typeof SimulatorStates)[keyof typeof SimulatorStates];

export interface BaseSimulatorOptions {
  onBreakpoint?: (index: number) => void;
  onStateChange?: (state: SimulatorState) => void;
  onWrite?: (data: string) => void;
  onError?: (error: string) => void;
  /* eslint-disable-next-line @typescript-eslint/no-explicit-any -- We want to allow extending this type */
  [key: string]: any;
}

/**
 * A high-level interface for a machine simulation.
 */
export abstract class Simulator {
  _options: BaseSimulatorOptions;
  _state: SimulatorState;

  constructor(options: BaseSimulatorOptions) {
    this._options = options;
    this._state = SimulatorStates.Uninitialized;
  }

  /**
   * Retrieve the current state of the simulator.
   */
  get state(): SimulatorState {
    return this._state;
  }

  set state(value: SimulatorState) {
    if (this._state !== value) {
      this._state = value;
      if (this._options.onStateChange) {
        this._options.onStateChange(this._state);
      }
    }
  }

  /**
   * Mounts a file system directory to the simulator, if supported.
   */
  abstract mount(): Promise<boolean>;

  /**
   * Boots the simulator.
   */
  abstract run(): Promise<boolean>;

  /**
   * Pauses the simulator, if it is currently running.
   */
  abstract pause(): Promise<boolean>;

  /**
   * Resumes a paused simulation.
   */
  abstract resume(): Promise<boolean>;

  /**
   * Provides a description of the registers the simulator will output.
   */
  abstract get registerMap(): RegisterInfo;

  /**
   * Retrieves the state of all registers.
   */
  abstract registers(): Promise<object>;

  /**
   * Reads a particular register value.
   */
  abstract readRegister(register: string): Promise<boolean>;

  /**
   * Writes the given value to the given register.
   */
  abstract writeRegister(register: string, value: number): Promise<boolean>;

  /**
   * Provides a description of memory.
   */
  abstract get memoryMap(): object;

  /**
   * Reads from the simulation memory.
   */
  abstract readMemory(
    device: number,
    context: number,
    address: number,
  ): Promise<number>;

  /**
   * Writes to the simulation memory.
   */
  abstract writeMemory(
    device: number,
    context: number,
    address: number,
    value: number,
  ): Promise<boolean>;

  /**
   * Creates an instruction breakpoint and returns an index for it.
   */
  abstract addBreakpoint(address: number): Promise<number>;

  /**
   * Removes an existing breakpoint via its index.
   */
  abstract removeBreakpoint(index: number): Promise<boolean>;

  /**
   * Clears all breakpoints
   */
  abstract clearBreakpoints(): Promise<boolean>;

  /**
   * Sets an output video device.
   */
  abstract attachVideo(device: Video, index: number): Promise<boolean>;

  /**
   * Sets an output serial device.
   */
  abstract attachSerial(device: Serial, index: number): Promise<boolean>;
}

export default Simulator;
