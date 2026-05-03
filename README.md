<div align="center">

<img src="https://github.com/benjibrown/oxyl/blob/main/assets/logo.png?raw=true" alt="logo" width="900"/>

**A LaTeX compiler written in Rust.**
</div>


**Status:** very early stage - workspace skeleton only, nothing to compile documents yet or produce an AST etc.

## What is this?

oxyl is a from-scratch LaTeX compiler. The goal is to be, produce helpful error messages, and eventually have proper package management built in rather than just being bolted on afterwards.

## Goals
- Parse and compile `.tex` files to PDF (i really want to get this done)
- Incremental compilation - only reprocess what changed 
- Clear error messages + source locations 

## Building 
```sh 
cargo build 
cargo run 
```


