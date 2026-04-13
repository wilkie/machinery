import type { ClockInfo } from './Clock';

export interface BusRowInfo {
  /** The identifier for the row which is indicated in pin assignments */
  identifier: string;
  /* A more descriptive name for the row */
  name: string;
  /* The number of pins on this row */
  count: number;
}

export interface BusPinInfo {
  /** The identifier for the pin */
  identifier: string;
  /** A more descriptive name for the pin, if needed */
  name?: string;
  /** A longer description for the signal, if desired */
  description?: string;
  /**
   * The pin's location (row name and index).
   *
   * If multiple are specified, this is because the pin signal is redundant and
   * each pin carries the same signal.
   */
  pin: [string, number] | [string, number][];
}

export interface BusLineInfo {
  /** The identifier to use to reference this value */
  identifier: string;
  /** A more descriptive name for this particular line */
  name: string;
  /** The number of bits this value has at most */
  size: number;
  /** Separate fields that can be referenced within the value */
  fields?: BusLineFieldInfo[];
  /** The pin locations for the line */
  pins: BusPinInfo[];
}

export interface BusLayoutInfo {
  /** A description of each pin row */
  rows: BusRowInfo[];
  /** A description of the address line */
  address: BusLineInfo;
  /** A description of the data line */
  data: BusLineInfo;
  /** A description of the different control signals */
  control: BusPinInfo[];
  /** A description of power signals */
  power: BusPinInfo[];
}

/**
 * Describes a device that handles communication across a wire to connected devices.
 */
export interface BusInfoBase {
  /** A short identifier that uniquely identifies this bus device */
  identifier: string;
  /** A more descriptive name for the bus */
  name: string;
  /** A longer description of this bus */
  description: string;
  /** A description of the clock behavior */
  clock: ClockInfo;
  /** A description of the layout of the bus wires and pins */
  layout: BusLayoutInfo;
  /** Other buses (by identifier) this bus can also handle */
  compatible?: string[];
}

/**
 * The different states a pin can be in during a cycle.
 *
 * The 'pulse' state means that it toggles its state during the cycle.
 * Therefore, it contains both edges during the cycle. If it was 'low' to
 * begin the cycle, it pulses 'high' and returns to low within the cycle.
 * Vice versa if the pin was 'high' to start.
 *
 * The 'low' and 'high' just assert the line in either direction. This does
 * not regard the active status of the pin. If you mark a pin 'low' and it
 * is active at 'low', that pin becomes a value of 1. The protocol orients
 * around the level, not the semantics of the pins.
 */
export type BusSignalState = 'pulse' | 'low' | 'high';

/**
 * Describes a cycle in the read/write protocol for the bus.
 *
 * The 'signal' section suggests what pins are active during that
 * cycle. If 'pulse' is used, the pin is active within that cycle
 * for edge-triggering.
 *
 * The 'sample' section suggests that the bus is listening to a
 * particular signal downstream. It latches that value in the
 * given internal state.
 *
 * The 'wait' section captures which signals the bus is waiting on.
 * It will create wait cycles until all signals it is waiting for are
 * properly reflected. Then, it will continue.
 *
 * When the 'wait' section exists, the 'respond' section can also be
 * described. This contains signals that happen in response to the
 * signals achieved in the 'wait'. These happen during the cycle.
 */
export interface BusProtocolCycle {
  /**
   * Affect the given signals to put them in the indicated states.
   */
  signal?: {
    [key: string]: BusSignalState;
  };
  /**
   * Latch the particular signals and store values into internal state.
   */
  sample?: {
    [key: string]: string;
  };
  /**
   * Insert cycles until the given signals reflect the given states.
   */
  wait?: {
    [key: string]: BusSignalState;
  };
  /**
   * When we would release our wait, quickly set the state of the given signals
   * so that they update during the cycle.
   */
  respond?: Omit<BusProtocolCycle, 'wait' | 'respond'>;
}

/**
 * Describes the different signals used to achieve the high-level read/write
 * operation on the bus (or on a particular channel).
 *
 * If the given size is not provided, then it is not possible to perform that
 * operation on the bus or device.
 */
export interface BusProtocol {
  /** The protocol to perform to read/write an 8-bit byte from the bus or channel */
  byte?: BusProtocolCycle[];
  /** The protocol to perform to read/write a 16-bit word from the bus or channel */
  word?: BusProtocolCycle[];
  /** The protocol to perform to read/write a 32-bit word from the bus or channel */
  dword?: BusProtocolCycle[];
  /** The protocol to perform to read/write a 64-bit word from the bus or channel */
  qword?: BusProtocolCycle[];
}

/** For when a Bus only has one channel */
export interface BusInfoBasic extends BusInfoBase {
  channels: never;
  /** The operation to perform when we are reading from the bus directly */
  read: BusProtocol;
  /** The operation to perform when we are writing to the bus directly */
  write: BusProtocol;
}

/** For when a bus offers multiple channels */
export interface BusInfoChannels extends BusInfoBase {
  /** The bus allows different channels which offer different address spaces. */
  channels: {
    [key: string]: {
      /** The operation to perform when we are reading from this particular channel on the bus */
      read: BusProtocol;
      /** The operation to perform when we are writing to this particular channel on the bus */
      write: BusProtocol;
    }
  }
  read: never;
  write: never;
}

/** Describes a bus, which manages communication across a set of devices */
export type BusInfo = BusInfoChannels | BusInfoBasic;

/**
 * Expresses a required connection to a bus.
 *
 * When a device or processor requires a bus, it can presume it exists in operations,
 * but it must be specified when creating a full simulation.
 */
export interface BusRequestInfo {
  /** The bus requested by matching its identifier. */
  identifier: string;
}

/**
 * Expresses a connection within a System.
 */
export interface BusConnectionInfo {
  /** The device identifier */
  device: string;
}
