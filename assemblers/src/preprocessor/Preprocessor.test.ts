import { Preprocessor } from './Preprocessor.js';

function process(source: string, defines?: Record<string, string>): string {
  const pp = new Preprocessor({ defines });
  return pp.process(source);
}

/** Strip blank lines and leading/trailing whitespace per line for easier comparison */
function lines(s: string): string[] {
  return s
    .split('\n')
    .map((l) => l.trim())
    .filter((l) => l.length > 0);
}

describe('Preprocessor', () => {
  describe('%define', () => {
    it('substitutes a simple define', () => {
      const result = process('%define SIZE 16\nmov ax, SIZE');
      expect(lines(result)).toContain('mov ax, 16');
    });

    it('substitutes multiple defines', () => {
      const result = process('%define A 1\n%define B 2\nadd A, B');
      expect(lines(result)).toContain('add 1, 2');
    });

    it('does not substitute partial matches', () => {
      const result = process('%define AX 99\nmov AX_VAL, AX');
      // AX_VAL should NOT be substituted, but AX should
      expect(lines(result)).toContain('mov AX_VAL, 99');
    });

    it('handles chained defines', () => {
      const result = process('%define A B\n%define B 42\nmov ax, A');
      expect(lines(result)).toContain('mov ax, 42');
    });
  });

  describe('%assign', () => {
    it('evaluates numeric expressions', () => {
      const result = process('%assign COUNT 3+2\nmov ax, COUNT');
      expect(lines(result)).toContain('mov ax, 5');
    });

    it('evaluates hex values', () => {
      const result = process('%assign ADDR 0x100\nmov ax, ADDR');
      expect(lines(result)).toContain('mov ax, 256');
    });
  });

  describe('%macro / %endmacro', () => {
    it('expands a simple macro', () => {
      const src = [
        '%macro mymacro 2',
        '  mov %1, %2',
        '%endmacro',
        'mymacro ax, 0x55',
      ].join('\n');
      const result = lines(process(src));
      expect(result).toContain('mov ax, 0x55');
    });

    it('expands a macro with multiple body lines', () => {
      const src = [
        '%macro setup 1',
        '  push %1',
        '  pop ax',
        '%endmacro',
        'setup bx',
      ].join('\n');
      const result = lines(process(src));
      expect(result).toContain('push bx');
      expect(result).toContain('pop ax');
    });

    it('expands a macro invoked multiple times', () => {
      const src = [
        '%macro nop2 0',
        '  nop',
        '  nop',
        '%endmacro',
        'nop2',
        'nop2',
      ].join('\n');
      const result = lines(process(src));
      const nopCount = result.filter((l) => l === 'nop').length;
      expect(nopCount).toBe(4);
    });
  });

  describe('%rep / %endrep', () => {
    it('repeats body N times', () => {
      const src = ['%rep 3', '  nop', '%endrep'].join('\n');
      const result = lines(process(src));
      const nopCount = result.filter((l) => l === 'nop').length;
      expect(nopCount).toBe(3);
    });

    it('repeats zero times produces no output', () => {
      const src = ['%rep 0', '  db 0xFF', '%endrep'].join('\n');
      const result = lines(process(src));
      expect(result.filter((l) => l.includes('db'))).toHaveLength(0);
    });
  });

  describe('%if / %elif / %else / %endif', () => {
    it('selects the true branch', () => {
      const src = [
        '%define MODE 1',
        '%if MODE == 1',
        '  mov ax, 1',
        '%else',
        '  mov ax, 2',
        '%endif',
      ].join('\n');
      const result = lines(process(src));
      expect(result).toContain('mov ax, 1');
      expect(result).not.toContain('mov ax, 2');
    });

    it('selects the else branch', () => {
      const src = [
        '%define MODE 2',
        '%if MODE == 1',
        '  mov ax, 1',
        '%else',
        '  mov ax, 2',
        '%endif',
      ].join('\n');
      const result = lines(process(src));
      expect(result).not.toContain('mov ax, 1');
      expect(result).toContain('mov ax, 2');
    });

    it('selects elif branch', () => {
      const src = [
        '%define MODE 2',
        '%if MODE == 1',
        '  mov ax, 1',
        '%elif MODE == 2',
        '  mov ax, 2',
        '%else',
        '  mov ax, 3',
        '%endif',
      ].join('\n');
      const result = lines(process(src));
      expect(result).toContain('mov ax, 2');
      expect(result).not.toContain('mov ax, 1');
      expect(result).not.toContain('mov ax, 3');
    });
  });

  describe('%ifdef / %ifndef', () => {
    it('ifdef true when defined', () => {
      const src = [
        '%define FEATURE 1',
        '%ifdef FEATURE',
        '  mov ax, 1',
        '%endif',
      ].join('\n');
      const result = lines(process(src));
      expect(result).toContain('mov ax, 1');
    });

    it('ifdef false when not defined', () => {
      const src = ['%ifdef FEATURE', '  mov ax, 1', '%endif'].join('\n');
      const result = lines(process(src));
      expect(result).not.toContain('mov ax, 1');
    });

    it('ifndef true when not defined', () => {
      const src = ['%ifndef FEATURE', '  mov ax, 1', '%endif'].join('\n');
      const result = lines(process(src));
      expect(result).toContain('mov ax, 1');
    });
  });

  describe('%include', () => {
    it('includes file content', () => {
      const files: Record<string, string> = {
        'header.inc': 'mov ax, 0x100',
      };
      const pp = new Preprocessor({
        readFile: (path: string) => {
          if (files[path] !== undefined) return files[path];
          throw new Error(`File not found: ${path}`);
        },
      });
      const result = lines(pp.process('%include "header.inc"'));
      expect(result).toContain('mov ax, 0x100');
    });

    it('does not infinite-loop on re-included files', () => {
      // The preprocessor guards against the same file being included
      // while it is already on the include stack (direct recursion).
      // Indirect mutual recursion (a -> b -> a) still recurses because
      // the stack is popped in the finally block. This test verifies
      // simple direct re-inclusion is safe.
      const files: Record<string, string> = {
        'header.inc': 'mov ax, 1',
      };
      const pp = new Preprocessor({
        readFile: (path: string) => {
          if (files[path] !== undefined) return files[path];
          throw new Error(`File not found: ${path}`);
        },
      });
      const result = pp.process('%include "header.inc"\n%include "header.inc"');
      const resultLines = lines(result);
      // Both includes resolve (stack is cleared between calls)
      expect(resultLines.filter((l) => l === 'mov ax, 1')).toHaveLength(2);
    });
  });

  describe('comments', () => {
    it('preserves comment lines', () => {
      const result = process('; this is a comment\nmov ax, 1');
      const resultLines = result.split('\n');
      expect(resultLines.some((l) => l.trim() === '; this is a comment')).toBe(
        true,
      );
    });
  });

  describe('constructor defines', () => {
    it('uses defines from constructor options', () => {
      const result = process('mov ax, VAL', { VAL: '42' });
      expect(lines(result)).toContain('mov ax, 42');
    });
  });
});
