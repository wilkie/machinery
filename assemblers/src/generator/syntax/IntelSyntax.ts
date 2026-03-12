import type { Target, OpcodeMatcher } from '@machinery/core';
import { InstructionOperandTypes, RegisterTypes } from '@machinery/core';
import type { Syntax } from './Syntax';

interface RegisterEntry {
  name: string;
  size: number;
}

export class IntelSyntax implements Syntax {
  name = 'intel';
  operandOrder = 'dest-first' as const;
  memoryFormat = { brackets: ['[', ']'] as [string, string], sizePrefix: true };

  registerFormat(name: string): string {
    return name.toUpperCase();
  }

  generateTokenizer(target: Target): string {
    const registerMap = this.buildRegisterMap(target);
    const mnemonics = this.collectMnemonics(target);
    const prefixes = this.collectPrefixes(target);

    // Add NASM-specific mnemonic aliases not in the target definition
    mnemonics.add('RETF'); // NASM alias for far return
    const sortedMnemonics = [...mnemonics].sort();
    const sortedRegisters = [...registerMap.keys()].sort();

    const lines: string[] = [];
    lines.push(`@preprocessor typescript`);
    lines.push(``);
    lines.push(`@{%`);
    lines.push(`import moo from 'moo';`);
    lines.push(``);
    lines.push(
      `const caseInsensitiveKeywords = (map: Record<string, string[]>) => {`,
    );
    lines.push(`  const transform = moo.keywords(map);`);
    lines.push(`  return (value: string) => transform(value.toUpperCase());`);
    lines.push(`};`);
    lines.push(``);

    // Emit register info map so grammar post-processing can look up size/type
    lines.push(
      `const registerMap: Record<string, { type: string; name: string; size: number }> = {`,
    );
    for (const [name, entry] of [...registerMap.entries()].sort(([a], [b]) =>
      a.localeCompare(b),
    )) {
      const escaped = name.replace(/'/g, "\\'");
      lines.push(
        `  '${escaped}': { type: 'register', name: '${escaped}', size: ${entry.size} },`,
      );
    }
    lines.push(`};`);
    lines.push(``);

    // Segment register set for detecting segment overrides (derived from target metadata)
    const segmentRegs = target.registers
      .filter((r) => r.type === RegisterTypes.Segment)
      .map((r) => r.identifier.toUpperCase());
    lines.push(
      `const segmentRegisters = new Set<string>(${JSON.stringify(segmentRegs)});`,
    );
    lines.push(``);

    lines.push(`const lexer = moo.compile({`);
    lines.push(`  ws: { match: /[ \\t]+/, lineBreaks: false },`);
    lines.push(`  newline: { match: /\\r?\\n/, lineBreaks: true },`);
    lines.push(`  comment: /;[^\\n]*/,`);
    lines.push(`  hexnum: /0[xX][0-9a-fA-F]+|[0-9][0-9a-fA-F]*[hH]/,`);
    lines.push(`  binnum: /0[bB][01]+|[01]+[bB]/,`);
    lines.push(`  number: /[0-9]+/,`);
    lines.push(
      `  string: { match: /'[^']*'|"[^"]*"/, value: s => s.slice(1, -1) },`,
    );
    lines.push(`  comma: ',',`);
    lines.push(`  colon: ':',`);
    lines.push(`  plus: '+',`);
    lines.push(`  minus: '-',`);
    lines.push(`  star: '*',`);
    lines.push(`  slash: '/',`);
    lines.push(`  ampersand: '&',`);
    lines.push(`  pipe: '|',`);
    lines.push(`  tilde: '~',`);
    lines.push(`  lparen: '(',`);
    lines.push(`  rparen: ')',`);
    lines.push(`  lbracket: '[',`);
    lines.push(`  rbracket: ']',`);
    lines.push(`  dot: '.',`);
    lines.push(`  dollar: '$',`);
    lines.push(`  ident: {`);
    lines.push(`    match: /[a-zA-Z_][a-zA-Z0-9_]*'?/,`);
    lines.push(`    type: caseInsensitiveKeywords({`);
    lines.push(
      `      mnemonic: [${sortedMnemonics.map((m) => `'${m}'`).join(', ')}],`,
    );
    lines.push(
      `      register: [${sortedRegisters.map((r) => `'${r.replace(/'/g, "\\'")}'`).join(', ')}],`,
    );
    lines.push(`      kw_byte: ['BYTE'],`);
    lines.push(`      kw_word: ['WORD'],`);
    lines.push(`      kw_ptr: ['PTR'],`);
    lines.push(`      kw_short: ['SHORT'],`);
    lines.push(`      kw_far: ['FAR'],`);
    lines.push(`      kw_near: ['NEAR'],`);
    lines.push(`      kw_org: ['ORG'],`);
    lines.push(`      kw_db: ['DB'],`);
    lines.push(`      kw_dw: ['DW'],`);
    lines.push(`      kw_dd: ['DD'],`);
    lines.push(`      kw_equ: ['EQU'],`);
    lines.push(`      kw_times: ['TIMES'],`);
    lines.push(`      kw_bits: ['BITS'],`);
    lines.push(`      kw_section: ['SECTION'],`);
    lines.push(`      kw_resb: ['RESB'],`);
    lines.push(`      kw_resw: ['RESW'],`);
    lines.push(`      kw_resd: ['RESD'],`);
    lines.push(`      kw_align: ['ALIGN'],`);
    // Add prefix instruction keywords from target
    for (const prefix of prefixes) {
      const allNames = [prefix.name, ...prefix.aliases];
      lines.push(
        `      kw_prefix_${prefix.name.toLowerCase()}: [${allNames.map((n) => `'${n}'`).join(', ')}],`,
      );
    }
    lines.push(`    }),`);
    lines.push(`  },`);
    lines.push(`});`);
    lines.push(`%}`);

    return lines.join('\n');
  }

  generateGrammarRules(_target: Target): string {
    const prefixes = this.collectPrefixes(_target);
    const lines: string[] = [];

    lines.push(`@lexer lexer`);
    lines.push(``);
    lines.push(`# Program structure`);
    lines.push(
      `program -> _ lines _ {% ([, lines]) => lines.filter(Boolean) %}`,
    );
    lines.push(``);
    lines.push(`lines -> line {% ([l]) => [l] %}`);
    lines.push(`       | lines newline line {% ([ls,, l]) => [...ls, l] %}`);
    lines.push(``);
    lines.push(`line -> _ {% () => null %}`);
    lines.push(`      | _ comment {% () => null %}`);
    lines.push(`      | _ statement _ comment:? {% ([, s]) => s %}`);
    lines.push(``);

    // statement: label, directive, instruction, or multi-node forms that produce arrays
    lines.push(`statement -> label {% id %}`);
    lines.push(`           | directive {% id %}`);
    lines.push(`           | instruction {% id %}`);
    // label-before-directive: "bb db 0x0, 0x0" or "mydata dw 0x1234"
    lines.push(`           | label_directive {% id %}`);
    // label-before-instruction: "start: mov ax, 0" (label with colon then instruction on same line)
    lines.push(
      `           | label __ instruction {% ([lbl,, inst]) => [lbl, inst] %}`,
    );
    lines.push(
      `           | label _ directive {% ([lbl,, dir]) => [lbl, dir] %}`,
    );
    lines.push(``);
    lines.push(`newline -> %newline {% () => null %}`);
    lines.push(`         | newline %newline {% () => null %}`);
    lines.push(``);
    lines.push(`comment -> %comment {% () => null %}`);
    lines.push(``);
    lines.push(`_ -> null {% () => null %}`);
    lines.push(`   | %ws {% () => null %}`);
    lines.push(``);
    lines.push(`__ -> %ws {% () => null %}`);
    lines.push(``);

    // Labels
    lines.push(`# Labels`);
    lines.push(
      `label -> %ident _ %colon {% ([name]) => ({ type: 'label', name: name.value }) %}`,
    );
    // Local labels: .name:
    lines.push(
      `       | %dot %ident _ %colon {% ([, name]) => ({ type: 'label', name: '.' + name.value }) %}`,
    );
    lines.push(``);

    // Label-before-directive: "name db ..." / "name dw ..." / "name dd ..." (no colon)
    lines.push(`# Label-before-directive (no colon)`);
    lines.push(
      `label_directive -> %ident __ %kw_db __ db_args {% ([name,,,, args]) => [{ type: 'label', name: name.value }, { type: 'directive', name: 'DB', args }] %}`,
    );
    lines.push(
      `                 | %ident __ %kw_dw __ dw_args {% ([name,,,, args]) => [{ type: 'label', name: name.value }, { type: 'directive', name: 'DW', args }] %}`,
    );
    lines.push(
      `                 | %ident __ %kw_dd __ dd_args {% ([name,,,, args]) => [{ type: 'label', name: name.value }, { type: 'directive', name: 'DD', args }] %}`,
    );
    lines.push(``);

    // Directives
    lines.push(`# Directives`);
    lines.push(
      `directive -> %kw_org __ expr {% ([,, val]) => ({ type: 'directive', name: 'ORG', args: [val] }) %}`,
    );
    lines.push(
      `           | %kw_db __ db_args {% ([,, args]) => ({ type: 'directive', name: 'DB', args }) %}`,
    );
    lines.push(
      `           | %kw_dw __ dw_args {% ([,, args]) => ({ type: 'directive', name: 'DW', args }) %}`,
    );
    lines.push(
      `           | %kw_dd __ dd_args {% ([,, args]) => ({ type: 'directive', name: 'DD', args }) %}`,
    );
    lines.push(
      `           | %ident __ %kw_equ __ expr {% ([name,,,,val]) => ({ type: 'directive', name: 'EQU', args: [name.value, val] }) %}`,
    );
    // TIMES N DB/DW VALUE
    lines.push(
      `           | %kw_times __ expr __ %kw_db __ db_args {% ([,, count,,,, args]) => ({ type: 'directive', name: 'TIMES', args: [count, 'DB', ...args] }) %}`,
    );
    lines.push(
      `           | %kw_times __ expr __ %kw_dw __ dw_args {% ([,, count,,,, args]) => ({ type: 'directive', name: 'TIMES', args: [count, 'DW', ...args] }) %}`,
    );
    lines.push(
      `           | %kw_times __ expr __ %kw_dd __ dd_args {% ([,, count,,,, args]) => ({ type: 'directive', name: 'TIMES', args: [count, 'DD', ...args] }) %}`,
    );
    // BITS 16 - no-op, parse and ignore
    lines.push(
      `           | %kw_bits __ expr {% ([,, val]) => ({ type: 'directive', name: 'BITS', args: [val] }) %}`,
    );
    // SECTION .text / .data - no-op
    lines.push(
      `           | %kw_section __ %dot %ident {% ([,,,name]) => ({ type: 'directive', name: 'SECTION', args: ['.' + name.value] }) %}`,
    );
    // RESB N / RESW N / RESD N - reserve N bytes/words/dwords of zeros
    lines.push(
      `           | %kw_resb __ expr {% ([,, count]) => ({ type: 'directive', name: 'TIMES', args: [count, 'DB', 0] }) %}`,
    );
    lines.push(
      `           | %kw_resw __ expr {% ([,, count]) => ({ type: 'directive', name: 'TIMES', args: [count, 'DW', 0] }) %}`,
    );
    lines.push(
      `           | %kw_resd __ expr {% ([,, count]) => ({ type: 'directive', name: 'TIMES', args: [count, 'DD', 0] }) %}`,
    );
    // ALIGN N: emit padding to align to N byte boundary
    lines.push(
      `           | %kw_align __ expr {% ([,, alignment]) => ({ type: 'directive', name: 'ALIGN', args: [alignment] }) %}`,
    );
    lines.push(``);

    // DB args: expressions, strings
    lines.push(`db_args -> db_arg {% ([a]) => [a] %}`);
    lines.push(
      `         | db_args _ %comma _ db_arg {% ([as,,,,a]) => [...as, a] %}`,
    );
    lines.push(``);
    lines.push(`db_arg -> expr {% id %}`);
    lines.push(`        | %string {% ([s]) => s.value %}`);
    lines.push(``);

    // DW args: expressions, labels
    lines.push(`dw_args -> dw_arg {% ([a]) => [a] %}`);
    lines.push(
      `         | dw_args _ %comma _ dw_arg {% ([as,,,,a]) => [...as, a] %}`,
    );
    lines.push(``);
    lines.push(`dw_arg -> expr {% id %}`);
    lines.push(``);

    // DD args
    lines.push(`dd_args -> dd_arg {% ([a]) => [a] %}`);
    lines.push(
      `         | dd_args _ %comma _ dd_arg {% ([as,,,,a]) => [...as, a] %}`,
    );
    lines.push(``);
    lines.push(`dd_arg -> expr {% id %}`);
    lines.push(``);

    // Instructions
    lines.push(`# Instructions`);
    lines.push(
      `instruction -> mnemonic {% ([m]) => ({ type: 'instruction', mnemonic: m, operands: [] }) %}`,
    );
    lines.push(
      `             | mnemonic __ operand_list {% ([m,, ops]) => ({ type: 'instruction', mnemonic: m, operands: ops }) %}`,
    );
    // Segment register as instruction prefix: "es lea ax, [bx+si+3]"
    lines.push(
      `             | %register __ mnemonic __ operand_list {% ([seg,, m,, ops]) => {`,
    );
    lines.push(`                 const segName = seg.value.toUpperCase();`);
    lines.push(
      `                 if (!segmentRegisters.has(segName)) return null;`,
    );
    lines.push(
      `                 const newOps = ops.map((op: { type: string; segment?: string }) => op.type === 'memory' ? { ...op, segment: segName } : op);`,
    );
    lines.push(
      `                 return { type: 'instruction', mnemonic: m, operands: newOps };`,
    );
    lines.push(`               } %}`);
    lines.push(`             | %register __ mnemonic {% ([seg,, m]) => {`);
    lines.push(`                 const segName = seg.value.toUpperCase();`);
    lines.push(
      `                 if (!segmentRegisters.has(segName)) return null;`,
    );
    lines.push(
      `                 return { type: 'instruction', mnemonic: segName + ' ' + m, operands: [] };`,
    );
    lines.push(`               } %}`);
    // Prefix instruction + string instruction (e.g., REP STOSB, REPE CMPSB)
    for (const prefix of prefixes) {
      const kwName = `kw_prefix_${prefix.name.toLowerCase()}`;
      lines.push(
        `             | %${kwName} __ mnemonic {% ([p,, m]) => ({ type: 'instruction', mnemonic: p.value.toUpperCase() + ' ' + m, operands: [] }) %}`,
      );
    }
    lines.push(``);
    lines.push(`mnemonic -> %mnemonic {% ([m]) => m.value.toUpperCase() %}`);
    lines.push(``);
    lines.push(`operand_list -> operand {% ([op]) => [op] %}`);
    lines.push(
      `              | operand_list _ %comma _ operand {% ([ops,,,,op]) => [...ops, op] %}`,
    );
    lines.push(``);

    // Operands
    lines.push(`# Operands`);
    lines.push(
      `operand -> %register {% ([r]) => registerMap[r.value.toUpperCase()] %}`,
    );
    lines.push(`         | memory {% id %}`);
    lines.push(`         | immediate {% id %}`);
    lines.push(
      `         | %kw_short __ immediate {% ([,, imm]) => ({ ...imm, short: true }) %}`,
    );
    // FAR seg:off (with explicit FAR keyword)
    lines.push(
      `         | %kw_far __ expr %colon expr {% ([,, seg,, off]) => ({ type: 'immediate', value: { seg, off }, far: true }) %}`,
    );
    // seg:off without FAR keyword: "jmp 0x1234:0x4321"
    lines.push(
      `         | expr _ %colon _ expr {% ([seg,,,, off]) => ({ type: 'immediate', value: { seg, off }, far: true }) %}`,
    );
    // NEAR qualifier on memory: "jmp near [bp + si - 9]"
    lines.push(
      `         | %kw_near __ memory {% ([,, mem]) => ({ ...mem, near: true }) %}`,
    );
    // FAR qualifier on memory: "jmp far [ptr_ip_cs]"
    lines.push(
      `         | %kw_far __ memory {% ([,, mem]) => ({ ...mem, far: true }) %}`,
    );
    // NEAR qualifier on immediate (label): "jmp near label"
    lines.push(
      `         | %kw_near __ immediate {% ([,, imm]) => ({ ...imm, near: true }) %}`,
    );
    // WORD [mem] - size hint as word for indirect jumps/calls
    lines.push(
      `         | %kw_word __ memory {% ([,, mem]) => ({ ...mem, size: 16 }) %}`,
    );
    // BYTE [mem] without PTR - size hint for indirect operations
    lines.push(
      `         | %kw_byte __ memory {% ([,, mem]) => ({ ...mem, size: 8 }) %}`,
    );
    // BYTE/WORD size hint on immediate (for sign-extension syntax: "cmp word [mem], byte -1")
    lines.push(
      `         | %kw_byte __ immediate {% ([,, imm]) => ({ ...imm, size: 8 }) %}`,
    );
    lines.push(
      `         | %kw_word __ immediate {% ([,, imm]) => ({ ...imm, size: 16 }) %}`,
    );
    lines.push(``);

    // Memory operands
    lines.push(`# Memory operands`);
    lines.push(
      `memory -> %lbracket _ mem_expr _ %rbracket {% ([,, expr]) => ({ type: 'memory', ...expr }) %}`,
    );
    lines.push(
      `        | %kw_byte __ %kw_ptr __ %lbracket _ mem_expr _ %rbracket {% ([,,,,, , expr]) => ({ type: 'memory', ...expr, size: 8 }) %}`,
    );
    lines.push(
      `        | %kw_word __ %kw_ptr __ %lbracket _ mem_expr _ %rbracket {% ([,,,,, , expr]) => ({ type: 'memory', ...expr, size: 16 }) %}`,
    );
    // Segment override outside brackets: reg:[mem]
    lines.push(
      `        | %register %colon %lbracket _ mem_expr _ %rbracket {% ([seg,,,,expr]) => ({ type: 'memory', ...expr, segment: seg.value.toUpperCase() }) %}`,
    );
    lines.push(
      `        | %kw_byte __ %kw_ptr __ %register %colon %lbracket _ mem_expr _ %rbracket {% ([,,,, seg,,, , expr]) => ({ type: 'memory', ...expr, size: 8, segment: seg.value.toUpperCase() }) %}`,
    );
    lines.push(
      `        | %kw_word __ %kw_ptr __ %register %colon %lbracket _ mem_expr _ %rbracket {% ([,,,, seg,,, , expr]) => ({ type: 'memory', ...expr, size: 16, segment: seg.value.toUpperCase() }) %}`,
    );
    lines.push(``);

    // Memory expression internals
    lines.push(`mem_expr -> mem_base {% id %}`);
    lines.push(
      `          | mem_base _ %plus _ mem_index {% ([base,,,, idx]) => ({ ...base, index: idx.index || idx.base }) %}`,
    );
    lines.push(
      `          | mem_base _ %plus _ expr {% ([base,,,, disp]) => ({ ...base, displacement: disp }) %}`,
    );
    lines.push(
      `          | mem_base _ %minus _ expr {% ([base,,,, disp]) => ({ ...base, displacement: (typeof disp === 'number') ? -disp : { type: 'expression', op: '-', left: 0, right: disp } }) %}`,
    );
    lines.push(
      `          | mem_base _ %plus _ mem_index _ %plus _ expr {% ([base,,,, idx,,,, disp]) => ({ ...base, index: idx.index || idx.base, displacement: disp }) %}`,
    );
    lines.push(
      `          | mem_base _ %plus _ mem_index _ %minus _ expr {% ([base,,,, idx,,,, disp]) => ({ ...base, index: idx.index || idx.base, displacement: (typeof disp === 'number') ? -disp : { type: 'expression', op: '-', left: 0, right: disp } }) %}`,
    );
    lines.push(`          | expr {% ([disp]) => ({ displacement: disp }) %}`);
    // Segment override inside brackets: [ds:bp], [es:bx+si+3], [es:label]
    lines.push(
      `          | %register %colon mem_base {% ([seg,, base]) => ({ ...base, segment: seg.value.toUpperCase() }) %}`,
    );
    lines.push(
      `          | %register %colon mem_base _ %plus _ mem_index {% ([seg,, base,,,, idx]) => ({ ...base, index: idx.index || idx.base, segment: seg.value.toUpperCase() }) %}`,
    );
    lines.push(
      `          | %register %colon mem_base _ %plus _ expr {% ([seg,, base,,,, disp]) => ({ ...base, displacement: disp, segment: seg.value.toUpperCase() }) %}`,
    );
    lines.push(
      `          | %register %colon mem_base _ %minus _ expr {% ([seg,, base,,,, disp]) => ({ ...base, displacement: (typeof disp === 'number') ? -disp : { type: 'expression', op: '-', left: 0, right: disp }, segment: seg.value.toUpperCase() }) %}`,
    );
    lines.push(
      `          | %register %colon mem_base _ %plus _ mem_index _ %plus _ expr {% ([seg,, base,,,, idx,,,, disp]) => ({ ...base, index: idx.index || idx.base, displacement: disp, segment: seg.value.toUpperCase() }) %}`,
    );
    lines.push(
      `          | %register %colon mem_base _ %plus _ mem_index _ %minus _ expr {% ([seg,, base,,,, idx,,,, disp]) => ({ ...base, index: idx.index || idx.base, displacement: (typeof disp === 'number') ? -disp : { type: 'expression', op: '-', left: 0, right: disp }, segment: seg.value.toUpperCase() }) %}`,
    );
    // Segment override with displacement-only (no register base): [es:label_name]
    lines.push(
      `          | %register %colon expr {% ([seg,, disp]) => ({ displacement: disp, segment: seg.value.toUpperCase() }) %}`,
    );
    lines.push(``);

    lines.push(
      `mem_base -> %register {% ([r]) => ({ base: r.value.toUpperCase() }) %}`,
    );
    lines.push(
      `mem_index -> %register {% ([r]) => ({ index: r.value.toUpperCase() }) %}`,
    );
    lines.push(``);

    // Expression grammar with precedence (lowest to highest):
    //   expr: bitor_expr (|)
    //   bitor_expr: bitand_expr (&)
    //   bitand_expr: add_expr (+/-)
    //   add_expr: term (*)
    //   term: factor (atom, parens, unary -, ~)
    lines.push(`# Expression grammar with operator precedence`);
    lines.push(`expr -> bitor_expr {% id %}`);
    lines.push(``);
    lines.push(`bitor_expr -> bitand_expr {% id %}`);
    lines.push(
      `           | bitor_expr _ %pipe _ bitand_expr {% ([left,,,, right]) => {`,
    );
    lines.push(
      `               if (typeof left === 'number' && typeof right === 'number') return left | right;`,
    );
    lines.push(
      `               return { type: 'expression', op: '|', left, right };`,
    );
    lines.push(`             } %}`);
    lines.push(``);
    lines.push(`bitand_expr -> add_expr {% id %}`);
    lines.push(
      `            | bitand_expr _ %ampersand _ add_expr {% ([left,,,, right]) => {`,
    );
    lines.push(
      `                if (typeof left === 'number' && typeof right === 'number') return left & right;`,
    );
    lines.push(
      `                return { type: 'expression', op: '&', left, right };`,
    );
    lines.push(`              } %}`);
    lines.push(``);
    lines.push(`add_expr -> term {% id %}`);
    lines.push(`         | add_expr _ %plus _ term {% ([left,,,, right]) => {`);
    lines.push(
      `             if (typeof left === 'number' && typeof right === 'number') return left + right;`,
    );
    lines.push(
      `             return { type: 'expression', op: '+', left, right };`,
    );
    lines.push(`           } %}`);
    lines.push(
      `         | add_expr _ %minus _ term {% ([left,,,, right]) => {`,
    );
    lines.push(
      `             if (typeof left === 'number' && typeof right === 'number') return left - right;`,
    );
    lines.push(
      `             return { type: 'expression', op: '-', left, right };`,
    );
    lines.push(`           } %}`);
    lines.push(``);
    lines.push(`term -> factor {% id %}`);
    lines.push(`      | term _ %star _ factor {% ([left,,,, right]) => {`);
    lines.push(
      `          if (typeof left === 'number' && typeof right === 'number') return left * right;`,
    );
    lines.push(
      `          return { type: 'expression', op: '*', left, right };`,
    );
    lines.push(`        } %}`);
    lines.push(``);
    lines.push(`factor -> atom {% id %}`);
    lines.push(`        | %lparen _ expr _ %rparen {% ([,, e]) => e %}`);
    lines.push(`        | %minus factor {% ([, f]) => {`);
    lines.push(`            if (typeof f === 'number') return -f;`);
    lines.push(
      `            return { type: 'expression', op: '-', left: 0, right: f };`,
    );
    lines.push(`          } %}`);
    lines.push(`        | %tilde factor {% ([, f]) => {`);
    lines.push(`            if (typeof f === 'number') return ~f;`);
    lines.push(
      `            return { type: 'expression', op: '~', left: 0, right: f };`,
    );
    lines.push(`          } %}`);
    lines.push(``);
    lines.push(`atom -> %hexnum {% ([n]) => {`);
    lines.push(`          const v = n.value;`);
    lines.push(
      `          if (v.match(/[hH]$/)) return parseInt(v.slice(0, -1), 16);`,
    );
    lines.push(`          return parseInt(v, 16);`);
    lines.push(`        } %}`);
    lines.push(`      | %binnum {% ([n]) => {`);
    lines.push(`          const v = n.value;`);
    lines.push(
      `          if (v.match(/[bB]$/)) return parseInt(v.slice(0, -1), 2);`,
    );
    lines.push(`          return parseInt(v.slice(2), 2);`);
    lines.push(`        } %}`);
    lines.push(`      | %number {% ([n]) => parseInt(n.value, 10) %}`);
    lines.push(
      `      | %string {% ([s]) => s.value.length === 1 ? s.value.charCodeAt(0) : s.value %}`,
    );
    lines.push(`      | %ident {% ([id]) => id.value %}`);
    lines.push(`      | %dollar {% () => '$' %}`);
    // Local label reference in expression: .name (without colon)
    lines.push(`      | %dot %ident {% ([, id]) => '.' + id.value %}`);
    lines.push(``);

    // Immediate values (wraps expr for instruction operands)
    lines.push(`# Immediate values`);
    lines.push(`immediate -> expr {% ([v]) => {`);
    lines.push(
      `    if (typeof v === 'number') return { type: 'immediate', value: v };`,
    );
    lines.push(
      `    if (typeof v === 'string') return { type: 'immediate', value: v };`,
    );
    lines.push(`    return { type: 'immediate', value: v };`);
    lines.push(`  } %}`);

    return lines.join('\n');
  }

  /**
   * Builds a map of register name -> { size } by:
   * 1. Collecting register names from operand encoding fields
   * 2. Collecting literal register names from instruction form operands
   * 3. Looking up size from target.registers (including global sub-fields)
   */
  private buildRegisterMap(target: Target): Map<string, RegisterEntry> {
    const encodedNames = new Set<string>();

    // Collect from target operand definitions
    for (const op of target.operands) {
      this.collectFromMatcher(op, encodedNames);
    }

    // Collect from inline opcode matchers in instruction forms
    for (const inst of target.instructions) {
      for (const form of inst.forms) {
        for (const entry of form.opcode) {
          if (
            typeof entry === 'object' &&
            entry !== null &&
            'fields' in entry
          ) {
            this.collectFromMatcher(entry as OpcodeMatcher, encodedNames);
          }
        }
        // Collect literal register names from operands arrays.
        // An operand is "literal" if its name doesn't match any field identifier
        // in the form's opcode matchers (i.e., it's a fixed register name like 'AL').
        if (form.operands) {
          for (const op of form.operands) {
            const isField = form.opcode.some((entry) => {
              if (typeof entry === 'string') {
                // Referenced operand definition — check its fields
                const def = target.operands.find((d) => d.identifier === entry);
                return def?.fields?.some((f) => f.identifier === op);
              } else if (
                typeof entry === 'object' &&
                entry !== null &&
                'fields' in entry
              ) {
                // Inline matcher — check its fields
                return (entry as OpcodeMatcher).fields?.some(
                  (f) => f.identifier === op,
                );
              }
              return false;
            });
            if (!isField) {
              encodedNames.add(op);
            }
          }
        }
      }
    }

    // Build size lookup from target.registers
    const sizeMap = new Map<string, number>();
    for (const reg of target.registers) {
      sizeMap.set(reg.identifier, reg.size);
      if (reg.fields) {
        for (const field of reg.fields) {
          if (field.global) {
            sizeMap.set(field.identifier, field.size);
          }
        }
      }
    }

    // Build final register map
    const result = new Map<string, RegisterEntry>();
    for (const name of encodedNames) {
      const size = sizeMap.get(name);
      if (size === undefined) continue; // Not a known register
      result.set(name, { name, size });
    }

    return result;
  }

  private collectFromMatcher(
    matcher: OpcodeMatcher,
    encodedNames: Set<string>,
  ): void {
    if (!matcher.fields) return;
    for (const field of matcher.fields) {
      if (
        field.type !== InstructionOperandTypes.Register &&
        field.type !== InstructionOperandTypes.Memory
      )
        continue;
      if (!field.encoding) continue;

      for (const name of field.encoding) {
        if (!name) continue;
        encodedNames.add(name);
      }
    }
  }

  private collectPrefixes(
    target: Target,
  ): { name: string; aliases: string[]; opcode: number }[] {
    const prefixes: { name: string; aliases: string[]; opcode: number }[] = [];
    for (const inst of target.instructions) {
      if (!inst.prefix) continue;
      // Skip segment override prefixes (handled separately)
      if (inst.forms.some((f) => f.segmentOverride)) continue;
      const name = inst.identifier.toUpperCase();
      const aliases = (inst.aliases || []).map((a) => a.toUpperCase());
      const opcode =
        typeof inst.forms[0]?.opcode[0] === 'number'
          ? inst.forms[0].opcode[0]
          : 0;
      prefixes.push({ name, aliases, opcode });
    }
    return prefixes;
  }

  private collectMnemonics(target: Target): Set<string> {
    const mnemonics = new Set<string>();
    for (const inst of target.instructions) {
      if (inst.prefix) continue;
      mnemonics.add(inst.identifier.toUpperCase());
      if (inst.aliases) {
        for (const alias of inst.aliases) {
          mnemonics.add(alias.toUpperCase());
        }
      }
      for (const form of inst.forms) {
        if (form.aliases) {
          for (const alias of form.aliases) {
            mnemonics.add(alias.toUpperCase());
          }
        }
      }
    }
    return mnemonics;
  }
}
