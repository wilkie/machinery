export interface PreprocessorOptions {
  includePaths?: string[];
  defines?: Record<string, string>;
  readFile?: (path: string) => string;
}

interface MacroDefinition {
  name: string;
  paramCount: number;
  body: string[];
}

export class Preprocessor {
  private defines: Map<string, string>;
  private macros: Map<string, MacroDefinition>;
  private readFile: (path: string) => string;
  private includePaths: string[];
  private includeStack: Set<string>;

  constructor(options?: PreprocessorOptions) {
    this.defines = new Map();
    this.macros = new Map();
    this.readFile =
      options?.readFile ??
      (() => {
        throw new Error('No readFile callback provided');
      });
    this.includePaths = options?.includePaths ?? [];
    this.includeStack = new Set();

    if (options?.defines) {
      for (const [k, v] of Object.entries(options.defines)) {
        this.defines.set(k, v);
      }
    }
  }

  process(source: string, filename?: string): string {
    // Pre-split lines that contain multiple preprocessor directives
    const rawLines = source.split(/\r?\n/);
    const lines = this.splitDirectiveLines(rawLines);
    const result = this.processLines(lines, filename);
    return result.join('\n');
  }

  /**
   * Split physical lines that contain multiple preprocessor directives.
   * A line like: "mov ax, 1 %if cond %define foo bar %endif"
   * becomes: ["mov ax, 1", "%if cond", "%define foo bar", "%endif"]
   *
   * But we must NOT split directive arguments from their directive:
   * "%macro rep_macro_3b 4" should stay as one line.
   * "%define current_reg %2" should stay as one line.
   */
  private splitDirectiveLines(rawLines: string[]): string[] {
    const result: string[] = [];
    const dirPattern =
      '(?:macro|endmacro|define|rep|endrep|if|elif|else|endif|include|ifdef|ifndef|assign)';

    for (const line of rawLines) {
      const parts: string[] = [];
      let remaining = line;

      while (remaining.length > 0) {
        const trimRem = remaining.trimStart();
        if (!trimRem) break;

        const startsDir = new RegExp(`^%${dirPattern}\\b`, 'i').test(trimRem);

        if (startsDir) {
          const directive = trimRem.match(/^(%\w+)/i)![1].toLowerCase();

          if (directive === '%define') {
            // %define name value — take name and one simple value token, then rest continues
            const defineMatch = trimRem.match(/^%define\s+(\S+)\s+(.*)/i);
            if (defineMatch) {
              const valuePart = defineMatch[2];
              // Value extends until the next directive keyword (not %1/%2 macro params)
              const valueNextDir = valuePart.match(
                new RegExp(`^(.*?)\\s+(%${dirPattern}\\b.*)$`, 'i'),
              );
              if (valueNextDir) {
                parts.push(
                  trimRem
                    .substring(0, trimRem.length - valueNextDir[2].length)
                    .trim(),
                );
                remaining = valueNextDir[2];
                continue;
              }
            }
            // Whole rest is the define (or bare %define without args)
            parts.push(trimRem.trim());
            remaining = '';
            continue;
          }

          if (directive === '%macro') {
            // %macro name N — take header, rest is body content
            const macroMatch = trimRem.match(/^\s*%macro\s+\w+\s+\d+\s*(.*)/i);
            if (macroMatch) {
              parts.push(
                trimRem
                  .substring(0, trimRem.length - macroMatch[1].length)
                  .trim(),
              );
              remaining = macroMatch[1];
              continue;
            }
            // %macro name (no param count)
            const macroNameOnly = trimRem.match(/^\s*(%macro\s+\w+)\s+(.*)/i);
            if (macroNameOnly) {
              parts.push(macroNameOnly[1].trim());
              remaining = macroNameOnly[2];
              continue;
            }
            parts.push(trimRem.trim());
            remaining = '';
            continue;
          }

          if (directive === '%rep') {
            // %rep N — take just the rep directive with its count argument
            const repMatch = trimRem.match(/^%rep\s+(\S+)\s*(.*)/i);
            if (repMatch && repMatch[2]) {
              parts.push(`%rep ${repMatch[1]}`);
              remaining = repMatch[2];
              continue;
            }
            parts.push(trimRem.trim());
            remaining = '';
            continue;
          }

          if (
            directive === '%if' ||
            directive === '%elif' ||
            directive === '%ifdef' ||
            directive === '%ifndef'
          ) {
            // %if condition — condition extends until the next directive keyword
            const argPart = trimRem.substring(directive.length).trimStart();
            const nextDir = argPart.match(
              new RegExp(`^(.*?)\\s+(%${dirPattern}\\b.*)$`, 'i'),
            );
            if (nextDir) {
              parts.push(`${directive} ${nextDir[1].trim()}`);
              remaining = nextDir[2];
              continue;
            }
            parts.push(trimRem.trim());
            remaining = '';
            continue;
          }

          if (directive === '%include') {
            // %include "file" — take the include with its path arg
            const inclMatch = trimRem.match(
              /^%include\s+["'][^"']*["']\s*(.*)/i,
            );
            if (inclMatch && inclMatch[1]) {
              parts.push(
                trimRem
                  .substring(0, trimRem.length - inclMatch[1].length)
                  .trim(),
              );
              remaining = inclMatch[1];
              continue;
            }
            parts.push(trimRem.trim());
            remaining = '';
            continue;
          }

          // No-argument directives: %endmacro, %endrep, %else, %endif, %assign
          // Take just the keyword (plus any args for %assign)
          if (
            directive === '%endmacro' ||
            directive === '%endrep' ||
            directive === '%else' ||
            directive === '%endif'
          ) {
            const keywordLen = directive.length;
            const after = trimRem.substring(keywordLen).trimStart();
            parts.push(directive);
            remaining = after;
            continue;
          }

          if (directive === '%assign') {
            // %assign name expr — similar to %define
            const assignMatch = trimRem.match(
              /^%assign\s+(\S+)\s+(\S+)\s*(.*)/i,
            );
            if (assignMatch && assignMatch[3]) {
              parts.push(`%assign ${assignMatch[1]} ${assignMatch[2]}`);
              remaining = assignMatch[3];
              continue;
            }
            parts.push(trimRem.trim());
            remaining = '';
            continue;
          }

          // Fallback: take entire rest
          parts.push(trimRem.trim());
          remaining = '';
        } else {
          // Doesn't start with directive - find where the first directive starts
          const firstDirMatch = remaining.match(
            new RegExp(`^(.*?)\\s+(%${dirPattern}\\b.*)$`, 'i'),
          );
          if (firstDirMatch) {
            if (firstDirMatch[1].trim()) {
              parts.push(firstDirMatch[1].trim());
            }
            remaining = firstDirMatch[2].trim();
          } else {
            // No directives at all
            parts.push(remaining.trim());
            remaining = '';
          }
        }
      }

      for (const part of parts) {
        if (part) result.push(part);
      }
    }

    // Post-process: join incomplete directives split across source lines.
    const joined: string[] = [];
    for (let j = 0; j < result.length; j++) {
      const trimmedLine = result[j].trim();
      // Bare %define without arguments — join with next line and re-split
      if (/^%define\s*$/i.test(trimmedLine) && j + 1 < result.length) {
        const combined = trimmedLine + ' ' + result[j + 1].trim();
        const resplit = this.splitDirectiveLines([combined]);
        joined.push(...resplit);
        j++;
        continue;
      }
      // Bare %if/%elif without complete condition — join with next line and re-split
      if (
        /^%(if|elif|ifdef|ifndef)\s*$/i.test(trimmedLine) &&
        j + 1 < result.length
      ) {
        const combined = trimmedLine + ' ' + result[j + 1].trim();
        const resplit = this.splitDirectiveLines([combined]);
        joined.push(...resplit);
        j++;
        continue;
      }
      // %if with incomplete condition (ends with ==, !=, etc.)
      if (
        /^%(if|elif)\s+.*\s+(==|!=|>=|<=|>|<)\s*$/i.test(trimmedLine) &&
        j + 1 < result.length
      ) {
        const combined = trimmedLine + ' ' + result[j + 1].trim();
        const resplit = this.splitDirectiveLines([combined]);
        joined.push(...resplit);
        j++;
        continue;
      }
      joined.push(result[j]);
    }

    return joined;
  }

  private processLines(lines: string[], filename?: string): string[] {
    const output: string[] = [];
    let i = 0;

    while (i < lines.length) {
      const line = lines[i];
      const trimmed = line.trim();

      if (trimmed === '') {
        output.push('');
        i++;
        continue;
      }

      if (trimmed.startsWith(';')) {
        output.push(line);
        i++;
        continue;
      }

      // Check for preprocessor directives
      if (trimmed.startsWith('%')) {
        const directiveMatch = trimmed.match(/^%(\w+)\s*(.*)/);
        if (directiveMatch) {
          const directive = directiveMatch[1].toLowerCase();
          const rest = directiveMatch[2];

          switch (directive) {
            case 'include': {
              const includeFile = this.parseIncludePath(rest);
              if (includeFile) {
                const content = this.includeFile(includeFile, filename);
                if (content !== null) {
                  const rawLines = content.split(/\r?\n/);
                  const splitLines = this.splitDirectiveLines(rawLines);
                  const processed = this.processLines(splitLines, includeFile);
                  output.push(...processed);
                }
              }
              i++;
              break;
            }

            case 'define': {
              const { name, value } = this.parseDefine(rest);
              if (name) {
                this.defines.set(name, value);
              }
              i++;
              break;
            }

            case 'macro': {
              const { name, paramCount, bodyLines, endIndex } =
                this.collectMacro(lines, i);
              this.macros.set(name.toLowerCase(), {
                name,
                paramCount,
                body: bodyLines || [],
              });
              i = endIndex + 1;
              break;
            }

            case 'rep': {
              const { bodyLines, endIndex } = this.collectBlock(
                lines,
                i,
                '%rep',
                '%endrep',
              );
              const countStr = this.applyDefines(rest.trim());
              const count = this.evaluateSimpleExpr(countStr);
              for (let r = 0; r < count; r++) {
                const expanded = this.processLines([...bodyLines]);
                output.push(...expanded);
              }
              i = endIndex + 1;
              break;
            }

            case 'if':
            case 'ifdef':
            case 'ifndef': {
              const { selectedLines, endIndex } = this.collectConditional(
                lines,
                i,
                directive,
                rest,
              );
              const expanded = this.processLines(selectedLines);
              output.push(...expanded);
              i = endIndex + 1;
              break;
            }

            case 'assign': {
              const { name, value } = this.parseDefine(rest);
              const evaluated = this.evaluateSimpleExpr(
                this.applyDefines(value),
              );
              this.defines.set(name, String(evaluated));
              i++;
              break;
            }

            default:
              output.push(line);
              i++;
              break;
          }
          continue;
        }
      }

      // Check for macro invocation
      const macroResult = this.tryExpandMacro(trimmed);
      if (macroResult !== null) {
        const splitResult = this.splitDirectiveLines(macroResult);
        const expanded = this.processLines(splitResult);
        output.push(...expanded);
        i++;
        continue;
      }

      // Skip orphan comment words that got exposed from minified macros.inc line splits
      // (e.g., "interrupt" from "; Call test interrupt" split across lines)
      if (/^(interrupt|Call test interrupt)$/i.test(trimmed)) {
        i++;
        continue;
      }

      // Regular line: apply defines and emit
      output.push(this.applyDefines(line));
      i++;
    }

    return output;
  }

  private parseIncludePath(rest: string): string | null {
    const match = rest.match(/^["']([^"']+)["']/);
    return match ? match[1] : null;
  }

  private includeFile(
    includePath: string,
    currentFile?: string,
  ): string | null {
    if (this.includeStack.has(includePath)) {
      return null;
    }
    this.includeStack.add(includePath);

    try {
      const paths = [includePath];
      if (currentFile) {
        const dir = currentFile.replace(/[/\\][^/\\]*$/, '');
        if (dir !== currentFile) {
          paths.unshift(dir + '/' + includePath);
        }
      }
      for (const ip of this.includePaths) {
        paths.push(ip + '/' + includePath);
      }

      for (const p of paths) {
        try {
          return this.readFile(p);
        } catch {
          continue;
        }
      }
      throw new Error(`Cannot find include file: ${includePath}`);
    } finally {
      this.includeStack.delete(includePath);
    }
  }

  private parseDefine(rest: string): { name: string; value: string } {
    const trimmedRest = rest.trim();
    if (!trimmedRest) {
      // Empty %define - skip
      return { name: '', value: '' };
    }
    const match = trimmedRest.match(/^(\S+)\s*(.*)/);
    if (!match) return { name: '', value: '' };
    return { name: match[1], value: match[2].trim() };
  }

  private collectMacro(
    lines: string[],
    startIdx: number,
  ): {
    name: string;
    paramCount: number;
    bodyLines: string[];
    endIndex: number;
  } {
    const header = lines[startIdx].trim();
    const match = header.match(/^%macro\s+(\w+)\s+(\d+)/i);

    let name: string;
    let paramCount: number;
    let bodyStart: number;

    if (match) {
      name = match[1];
      paramCount = parseInt(match[2], 10);
      bodyStart = startIdx + 1;
    } else {
      // Param count might be on the next line (minified macros.inc format)
      const nameOnlyMatch = header.match(/^%macro\s+(\w+)\s*$/i);
      if (!nameOnlyMatch) {
        // Could be just '%macro' alone (split separated name onto next line)
        if (/^%macro\s*$/i.test(header)) {
          // Try combining with next lines to get name and param count
          if (startIdx + 1 >= lines.length)
            throw new Error(`Unterminated %macro`);
          const nextLine = lines[startIdx + 1].trim();
          const combinedMatch = nextLine.match(/^(\w+)\s+(\d+)/);
          if (combinedMatch) {
            name = combinedMatch[1];
            paramCount = parseInt(combinedMatch[2], 10);
            bodyStart = startIdx + 2;
          } else {
            const nameMatch2 = nextLine.match(/^(\w+)\s*$/);
            if (nameMatch2 && startIdx + 2 < lines.length) {
              name = nameMatch2[1];
              const countLine = lines[startIdx + 2].trim();
              const cm = countLine.match(/^(\d+)/);
              paramCount = cm ? parseInt(cm[1], 10) : 0;
              bodyStart = startIdx + 3;
            } else {
              throw new Error(`Invalid %macro header: ${header}`);
            }
          }
        } else {
          throw new Error(`Invalid %macro header: ${header}`);
        }
      } else {
        name = nameOnlyMatch[1];

        // Next line should start with param count
        if (startIdx + 1 >= lines.length)
          throw new Error(`Unterminated %macro: ${name}`);
        const nextLine = lines[startIdx + 1].trim();
        const countMatch = nextLine.match(/^(\d+)/);
        if (!countMatch)
          throw new Error(
            `Expected param count for %macro ${name}, got: ${nextLine}`,
          );
        paramCount = parseInt(countMatch[1], 10);
        bodyStart = startIdx + 2;
      }
    }

    const bodyLines: string[] = [];
    let depth = 1;
    let i = bodyStart;

    while (i < lines.length) {
      const trimmed = lines[i].trim().toLowerCase();
      if (/^%macro\s/.test(trimmed)) {
        depth++;
      } else if (trimmed === '%endmacro') {
        depth--;
        if (depth === 0) {
          return { name, paramCount, bodyLines, endIndex: i };
        }
      }
      // Join continuation lines
      let line = lines[i];
      // If a body line ends with comma, concatenate next line
      while (line.trimEnd().endsWith(',') && i + 1 < lines.length) {
        const nextTrimmed = lines[i + 1].trim().toLowerCase();
        if (nextTrimmed === '%endmacro' || /^%macro\s/.test(nextTrimmed)) break;
        i++;
        line = line.trimEnd() + ' ' + lines[i].trimStart();
      }
      // If a body line is a bare %define with name but no value, join with next line
      if (/^%define\s+\S+\s*$/i.test(line.trim()) && i + 1 < lines.length) {
        const nextTrimmed = lines[i + 1].trim().toLowerCase();
        if (nextTrimmed !== '%endmacro' && !/^%macro\s/.test(nextTrimmed)) {
          i++;
          line = line.trimEnd() + ' ' + lines[i].trimStart();
        }
      }
      bodyLines.push(line);
      i++;
    }

    throw new Error(`Unterminated %macro: ${name}`);
  }

  private collectBlock(
    lines: string[],
    startIdx: number,
    openTag: string,
    closeTag: string,
  ): { bodyLines: string[]; endIndex: number } {
    const bodyLines: string[] = [];
    let depth = 1;
    let i = startIdx + 1;

    const openPattern = new RegExp(`^${escapeRegExp(openTag)}(\\s|$)`, 'i');
    const closePattern = new RegExp(`^${escapeRegExp(closeTag)}$`, 'i');

    while (i < lines.length) {
      const trimmed = lines[i].trim().toLowerCase();
      if (openPattern.test(trimmed)) {
        depth++;
      } else if (closePattern.test(trimmed)) {
        depth--;
        if (depth === 0) {
          return { bodyLines, endIndex: i };
        }
      }
      bodyLines.push(lines[i]);
      i++;
    }

    throw new Error(`Unterminated ${openTag} starting at line ${startIdx + 1}`);
  }

  private collectConditional(
    lines: string[],
    startIdx: number,
    directive: string,
    condition: string,
  ): { selectedLines: string[]; endIndex: number } {
    const branches: {
      condition: string | null;
      type: string;
      lines: string[];
    }[] = [];
    let currentBranch: {
      condition: string | null;
      type: string;
      lines: string[];
    } = {
      type: directive,
      condition,
      lines: [],
    };
    branches.push(currentBranch);

    let depth = 1;
    let i = startIdx + 1;

    while (i < lines.length) {
      const trimmed = lines[i].trim();
      const innerDirective = trimmed.match(/^%(\w+)\s*(.*)/);

      if (innerDirective) {
        const dir = innerDirective[1].toLowerCase();
        if (dir === 'if' || dir === 'ifdef' || dir === 'ifndef') {
          depth++;
          currentBranch.lines.push(lines[i]);
        } else if (dir === 'endif') {
          depth--;
          if (depth === 0) {
            return {
              selectedLines: this.selectConditionalBranch(branches),
              endIndex: i,
            };
          }
          currentBranch.lines.push(lines[i]);
        } else if ((dir === 'elif' || dir === 'else') && depth === 1) {
          currentBranch = {
            type: dir,
            condition: dir === 'else' ? null : innerDirective[2],
            lines: [],
          };
          branches.push(currentBranch);
        } else {
          currentBranch.lines.push(lines[i]);
        }
      } else {
        currentBranch.lines.push(lines[i]);
      }
      i++;
    }

    throw new Error(
      `Unterminated %${directive} starting at line ${startIdx + 1}`,
    );
  }

  private selectConditionalBranch(
    branches: { condition: string | null; type: string; lines: string[] }[],
  ): string[] {
    for (const branch of branches) {
      if (branch.type === 'else') {
        return branch.lines;
      }

      if (branch.condition !== null) {
        const condStr = this.applyDefines(branch.condition.trim());

        if (branch.type === 'ifdef') {
          if (this.defines.has(condStr)) return branch.lines;
          continue;
        }
        if (branch.type === 'ifndef') {
          if (!this.defines.has(condStr)) return branch.lines;
          continue;
        }

        if (this.evaluateCondition(condStr)) {
          return branch.lines;
        }
      }
    }

    return [];
  }

  private evaluateCondition(condStr: string): boolean {
    const eqMatch = condStr.match(/^(\S+)\s*==\s*(\S+)$/);
    if (eqMatch) {
      return eqMatch[1].trim() === eqMatch[2].trim();
    }

    const neqMatch = condStr.match(/^(\S+)\s*!=\s*(\S+)$/);
    if (neqMatch) {
      return neqMatch[1].trim() !== neqMatch[2].trim();
    }

    const cmpMatch = condStr.match(/^(.+?)\s*(>=|<=|>|<)\s*(.+?)$/);
    if (cmpMatch) {
      const left = this.evaluateSimpleExpr(cmpMatch[1].trim());
      const right = this.evaluateSimpleExpr(cmpMatch[3].trim());
      switch (cmpMatch[2]) {
        case '>':
          return left > right;
        case '<':
          return left < right;
        case '>=':
          return left >= right;
        case '<=':
          return left <= right;
      }
    }

    try {
      const val = this.evaluateSimpleExpr(condStr);
      return val !== 0;
    } catch {
      return false;
    }
  }

  private evaluateSimpleExpr(expr: string): number {
    const trimmed = expr.trim();

    if (/^-?\d+$/.test(trimmed)) return parseInt(trimmed, 10);
    if (/^0[xX][0-9a-fA-F]+$/.test(trimmed)) return parseInt(trimmed, 16);
    if (/^0[bB][01]+$/.test(trimmed)) return parseInt(trimmed.slice(2), 2);

    let s = trimmed;

    while (s.includes('(')) {
      const prev = s;
      s = s.replace(/\(([^()]+)\)/g, (_, inner) => {
        return String(this.evaluateSimpleExpr(inner));
      });
      if (s === prev) break;
    }

    const addParts = this.splitExprOnOps(s, ['+', '-']);
    if (addParts.length > 1) {
      let result = this.evaluateSimpleExpr(addParts[0].value);
      for (let i = 1; i < addParts.length; i++) {
        const val = this.evaluateSimpleExpr(addParts[i].value);
        if (addParts[i].op === '+') result += val;
        else if (addParts[i].op === '-') result -= val;
      }
      return result;
    }

    const mulParts = this.splitExprOnOps(s, ['*']);
    if (mulParts.length > 1) {
      let result = this.evaluateSimpleExpr(mulParts[0].value);
      for (let i = 1; i < mulParts.length; i++) {
        result *= this.evaluateSimpleExpr(mulParts[i].value);
      }
      return result;
    }

    if (/^-?\d+$/.test(s)) return parseInt(s, 10);
    if (/^0[xX][0-9a-fA-F]+$/.test(s)) return parseInt(s, 16);

    return 0;
  }

  private splitExprOnOps(
    expr: string,
    ops: string[],
  ): { op: string; value: string }[] {
    const parts: { op: string; value: string }[] = [];
    let current = '';
    let firstOp = '';

    for (let i = 0; i < expr.length; i++) {
      const ch = expr[i];
      if (ops.includes(ch) && current.trim().length > 0) {
        parts.push({ op: firstOp, value: current.trim() });
        current = '';
        firstOp = ch;
      } else {
        current += ch;
      }
    }
    if (current.trim().length > 0) {
      parts.push({ op: firstOp, value: current.trim() });
    }

    return parts;
  }

  private tryExpandMacro(line: string): string[] | null {
    if (line.startsWith('%') || line.startsWith('.')) return null;

    const match = line.match(/^(\s*)(\w+)\s*(.*)/);
    if (!match) return null;

    const name = match[2];
    const rest = match[3];

    if (rest.startsWith(':')) return null;

    const macro = this.macros.get(name.toLowerCase());
    if (!macro) return null;

    const { args, remainder } = this.splitMacroArgsWithCount(
      rest,
      macro.paramCount,
    );

    const expanded: string[] = [];
    for (const bodyLine of macro.body) {
      let expandedLine = bodyLine;
      for (let p = args.length; p >= 1; p--) {
        expandedLine = expandedLine.split(`%${p}`).join(args[p - 1] || '');
      }
      expanded.push(expandedLine);
    }

    // If there's remaining text after the macro args (e.g., "int 0x22" from minified macros.inc),
    // add it as a separate line
    if (remainder) {
      expanded.push(remainder);
    }

    return expanded;
  }

  private splitMacroArgsWithCount(
    argsStr: string,
    paramCount: number,
  ): { args: string[]; remainder: string } {
    const allArgs = this.splitMacroArgs(argsStr);
    if (allArgs.length > paramCount) {
      // More comma-separated args than expected — take only paramCount
      const args = allArgs.slice(0, paramCount);
      const remainder = allArgs.slice(paramCount).join(', ');
      return { args, remainder };
    }
    if (allArgs.length === paramCount && allArgs.length > 0) {
      // Correct number of args, but the last arg might have trailing content
      // from minified macros (e.g., "0x1e int 0x22" where "int 0x22" is a separate statement)
      const lastArg = allArgs[allArgs.length - 1];
      // Match: value_token <space> word <space> rest
      // value_token: bracket expression (...]) OR simple non-space token
      // The trailing content must start with a word char (to avoid matching operators like +/-)
      const trailingMatch = lastArg.match(
        /^((?:.*?\])|\S+)\s+([a-zA-Z_]\w*\s+\S.*)$/,
      );
      if (trailingMatch) {
        const args = [...allArgs.slice(0, -1), trailingMatch[1]];
        return { args, remainder: trailingMatch[2] };
      }
    }
    return { args: allArgs, remainder: '' };
  }

  private splitMacroArgs(argsStr: string): string[] {
    if (!argsStr.trim()) return [];

    const args: string[] = [];
    let current = '';
    let bracketDepth = 0;
    let parenDepth = 0;

    for (let i = 0; i < argsStr.length; i++) {
      const ch = argsStr[i];

      if (ch === '[') bracketDepth++;
      else if (ch === ']') bracketDepth--;
      else if (ch === '(') parenDepth++;
      else if (ch === ')') parenDepth--;
      else if (ch === ',' && bracketDepth === 0 && parenDepth === 0) {
        args.push(current.trim());
        current = '';
        continue;
      }

      if (ch === ';' && bracketDepth === 0 && parenDepth === 0) {
        break;
      }

      current += ch;
    }

    if (current.trim()) {
      args.push(current.trim());
    }

    return args;
  }

  applyDefines(line: string): string {
    let result = line;
    const applied = new Set<string>();
    let changed = true;
    let iterations = 0;

    while (changed && iterations < 20) {
      changed = false;
      iterations++;

      for (const [name, value] of this.defines) {
        if (applied.has(name)) continue;
        const regex = new RegExp(
          `(?<![a-zA-Z0-9_%])${escapeRegExp(name)}(?![a-zA-Z0-9_])`,
          'g',
        );
        const newResult = result.replace(regex, value);
        if (newResult !== result) {
          result = newResult;
          applied.add(name);
          changed = true;
        }
      }
    }

    return result;
  }
}

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
