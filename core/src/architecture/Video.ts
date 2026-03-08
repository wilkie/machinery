/**
 * Represents a high-level video device which a Simulator can decide to make
 * use of for visual display.
 *
 * If the current setting is a headless mode where a graphical context is
 * not possible, the device will return an undefined canvas.
 */
export class Video {
  _canvas: HTMLCanvasElement | undefined;
  _width: number;
  _height: number;

  constructor(width: number, height: number) {
    this._canvas =
      typeof document !== 'undefined'
        ? document.createElement('canvas')
        : undefined;
    this._width = width;
    this._height = height;
  }

  get canvas(): HTMLCanvasElement | undefined {
    return this._canvas;
  }

  get width(): number {
    return this._width;
  }

  get height(): number {
    return this._height;
  }

  resize(width: number, height: number) {
    this._width = width;
    this._height = height;
  }
}

export default Video;
