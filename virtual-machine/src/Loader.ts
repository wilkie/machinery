import type { Executable, FileFormat } from '@machinery/core';
import type { ExpressionNode } from './ast';
import Evaluator, { type Evaluatable } from './Evaluator';
import Parser from './Parser';

class Loader {
  readonly format: FileFormat;
  readonly loader: Executable;
  readonly data: Uint8Array;
  readonly parser: Parser;
  readonly evaluator: Evaluator;
  readonly properties: Evaluatable;

  constructor(format: FileFormat, loader: Executable, data: Uint8Array) {
    this.format = format;
    this.loader = loader;
    this.data = data;
    this.parser = new Parser({}, {});
    this.properties = {
      file: {
        size: data.byteLength,
      },
    };
    this.evaluator = new Evaluator(this.properties);

    // Define properties
    for (const propertyName of Object.keys(format.data || {})) {
      Object.defineProperty(this, propertyName, {
        get: () => this.lookup(propertyName),
      });
    }
  }

  identify(): boolean {
    let matched = true;
    for (const token of this.format.identify || []) {
      let position = token.at;
      for (const byte of token.match) {
        if (this.data[position] !== byte) {
          matched = false;
          break;
        }
        position++;
      }

      if (!matched) {
        return false;
      }
    }
    return matched;
  }

  sizeof(type: string): number {
    if (['u8', 'u16', 'u32', 'u64', 'i8', 'i16', 'i32', 'i64'].includes(type)) {
      return parseInt(type.slice(1)) / 8;
    }

    const typeInfo = this.format.types?.[type];

    if (!typeInfo) {
      return 0;
    }

    let ret: number = 0;
    for (const field of typeInfo.fields) {
      if (field.count) {
        ret += field.count * this.sizeof(field.type);
      } else {
        ret += this.sizeof(field.type);
      }
    }

    return ret;
  }

  parse(type: string, offset: number, endian: string): Evaluatable | undefined {
    const typeInfo = this.format.types?.[type];

    if (!typeInfo) {
      return;
    }

    const ret: Evaluatable = {};
    const view = new DataView(this.data.buffer);

    for (const field of typeInfo.fields) {
      if (field.count) {
        ret[field.name] = Array(field.count)
          .fill(0)
          .map((_) => {
            if (field.type === 'u8') {
              return view.getUint8(offset);
              offset++;
            } else if (field.type === 'u16') {
              return view.getUint16(offset, endian === 'little');
              offset += 2;
            } else if (field.type === 'u32') {
              return view.getUint32(offset, endian === 'little');
              offset += 2;
            } else if (field.type === 'i8') {
              return view.getInt8(offset);
              offset++;
            } else if (field.type === 'i16') {
              return view.getInt16(offset, endian === 'little');
              offset += 2;
            } else if (field.type === 'i32') {
              return view.getInt32(offset, endian === 'little');
              offset += 2;
            }
            return 0;
          });
      } else if (field.type === 'u8') {
        ret[field.name] = view.getUint8(offset);
        offset++;
      } else if (field.type === 'u16') {
        ret[field.name] = view.getUint16(offset, endian === 'little');
        offset += 2;
      } else if (field.type === 'u32') {
        ret[field.name] = view.getUint32(offset, endian === 'little');
        offset += 2;
      } else if (field.type === 'i8') {
        ret[field.name] = view.getInt8(offset);
        offset++;
      } else if (field.type === 'i16') {
        ret[field.name] = view.getInt16(offset, endian === 'little');
        offset += 2;
      } else if (field.type === 'i32') {
        ret[field.name] = view.getInt32(offset, endian === 'little');
        offset += 2;
      }
    }

    return ret;
  }

  lookup(tag: string): Evaluatable | undefined {
    const description = this.format.data?.[tag];

    if (!description) {
      return;
    }

    if (tag in this.properties) {
      return this.properties[tag] as Evaluatable;
    }

    const result = this.parse(
      description.type,
      description.offset || 0,
      this.format.endianness || 'little',
    );
    if (!result) {
      return;
    }

    this.properties[tag] = result;

    // Add properties lazily
    for (const [propertyName, propertyInfo] of Object.entries(
      description.properties || {},
    )) {
      if (typeof propertyInfo === 'string') {
        // This is just code... so parse it
        console.log(this.parser.parse(propertyInfo + ' ;', {}, {}));
      } else {
        // This is defined as a field or list
        // Parse 'countExpression' if it is there (this means it is an array)
        if (propertyInfo.countExpression) {
          // This is a list
          this.parser.parse(propertyInfo.countExpression + ' ;', {}, {});
        }
      }

      Object.defineProperty(this.properties[tag], propertyName, {
        get: () =>
          typeof propertyInfo === 'string'
            ? this.evaluator.evaluate(
                this.parser.parse(propertyInfo + ' ;', {}, {})
                  .node as ExpressionNode,
              )
            : propertyInfo.countExpression
              ? Array(
                  this.evaluator.evaluate(
                    this.parser.parse(
                      propertyInfo.countExpression + ' ;',
                      {},
                      {},
                    ).node as ExpressionNode,
                  ),
                )
                  .fill(0)
                  .map((_, i) =>
                    this.parse(
                      propertyInfo.type,
                      this.evaluator.evaluate(
                        this.parser.parse(propertyInfo.at + ' ;', {}, {})
                          .node as ExpressionNode,
                      ) +
                        i * this.sizeof(propertyInfo.type),
                      this.format.endianness || 'little',
                    ),
                  )
              : this.parse(
                  propertyInfo.type,
                  this.evaluator.evaluate(
                    this.parser.parse(propertyInfo.at + ' ;', {}, {})
                      .node as ExpressionNode,
                  ),
                  this.format.endianness || 'little',
                ),
      });
    }

    return this.properties[tag] as Evaluatable;
  }
}

export default Loader;
