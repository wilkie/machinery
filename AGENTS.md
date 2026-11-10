# Machinery - A monorepo that contains a bunch of different libraries and projects related to emulation and reverse engineering.

## Layout

For a full description, read `docs/LAYOUT.md`

This is a TypeScript-heavy monorepo. The projects within are meant to be built separately but more or less be used together to serve the building of larger applications.

We are using turbopack and pnpm together to maintain the packages and build them. We are building them with tsup.

We want to use jest as the testing framework.

These are the main subdirectories for sub-projects:

- core - These are core types and utilities that are useful to all of the other packages.
- file-systems - These are implementations of processors that handle particular archive file formats and disk images.
- processors - These are descriptions of different processor targets and architectures.
- devices - These are descriptions of devices.
- simulators - These are simulations of different architectures.
- compilers - These are implementations of compilers for a variety of targets, some of which are classical.
- assemblers - These are assemblers for different targets.
- formats - These are different processors of file formats that are relevant to the preservation of systems.

## Performance work

Record every performance approach, kept or not, in `hdl/bench/CHRONICLE.md`: an index row and an entry (the idea, the change, before and after in host instructions with how they were measured, exactness, the outcome). Kept approaches name their commit. Approaches not kept leave their code as a patch in `hdl/bench/experiments/<date>-<name>.patch`, made with `git diff` from the repository root, headed with the commit it applies to.
