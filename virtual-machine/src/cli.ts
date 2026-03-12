#!/usr/bin/env node

import { parseArgs } from 'node:util';

import VirtualMachine from './VirtualMachine';
import Generator from './Generator';
import type { BackendClass } from './Backend';
import {
  TypeScriptBackend,
  WebAssemblyBackend,
  WasmMachineBackend,
} from './backends';

const backends: Record<string, BackendClass> = {
  typescript: TypeScriptBackend,
  webassembly: WebAssemblyBackend as unknown as BackendClass,
  'wasm-machine': WasmMachineBackend,
};

function usage(): never {
  console.error(
    `Usage: machinery-vm --target <name> [--backend <typescript|webassembly|wasm-machine>]`,
  );
  console.error();
  console.error(`Options:`);
  console.error(`  -t, --target <name>    Target processor (e.g. i286)`);
  console.error(`  -b, --backend <name>   Code backend (default: typescript)`);
  process.exit(1);
}

async function main() {
  const { values } = parseArgs({
    options: {
      target: { type: 'string', short: 't' },
      backend: { type: 'string', short: 'b', default: 'typescript' },
    },
    strict: true,
  });

  const targetName = values.target;
  const backendName = values.backend!;

  if (!targetName) {
    usage();
  }

  const backendClass = backends[backendName];
  if (!backendClass) {
    console.error(
      `Unknown backend: ${backendName}. Valid options: ${Object.keys(backends).join(', ')}`,
    );
    process.exit(1);
  }

  let target;
  try {
    const mod = await import(`@machinery/processors/${targetName}`);
    target = mod.default;
  } catch {
    console.error(
      `Failed to load target "${targetName}". Make sure @machinery/processors/${targetName} exists.`,
    );
    process.exit(1);
  }

  const vm = new VirtualMachine(target);
  const generator = new Generator(vm, backendClass);
  generator.generate();
}

main();
