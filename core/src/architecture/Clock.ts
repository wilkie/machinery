/**
 * Describes the clock attached to some device.
 */
export interface ClockInfo {
  /**
   * The frequency in which the clock 'ticks'.
   *
   * The first value is the factor followed by a string representing
   * the units. Either 'Hz' or 'MHz'.
   */
  frequency: [number, 'Hz' | 'MHz'];
}
