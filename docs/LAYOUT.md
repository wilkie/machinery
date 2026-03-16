# Project Layout

This is a TypeScript-heavy monorepo. The projects within are meant to be built separately but more or less be used together to serve the building of larger applications.

We are using turbopack and pnpm together to maintain the packages and build them. We are building them with tsup.

We want to use jest as the testing framework.

## Domains

- core - These are core types and utilities that are useful to all of the other packages.
- file-systems - These are implementations of processors that handle particular archive file formats and disk images.
- processors - These are descriptions of different processor targets and architectures.
- devices - These are descriptions of devices.
- simulators - These are simulations of different architectures.
- compilers - These are implementations of compilers for a variety of targets, some of which are classical.
- assemblers - These are assemblers for different targets.
- disassemblers - These are disassemblers for different targets.
- formats - These are different processors of file formats that are relevant to the preservation of systems.
- virtual-machine - The parser of operation code and generator of backend simulators.
