<div align="center">

<img src="https://txtr.benji.mom/oxyl.png" alt="logo" width="900"/>

**A LaTeX compiler written in Rust.**
</div>


**Status:** very early stage - workspace skeleton only, nothing to compile documents yet or produce an AST etc. Run with `--dump-tokens` or `--dump-ast` to see what oxyl is making of your file.

## What is this?

oxyl is a from-scratch LaTeX compiler. The goal is to be fast, produce helpful error messages, and eventually have proper package management built in rather than just being bolted on afterwards.

## Goals
- Parse and compile `.tex` files to PDF (i really want to get this done)
- Incremental compilation - only reprocess what changed 
- Clear error messages + source locations 
- An `oxyl.toml` package manifest instead of the TEXMF mess

## Building 
```sh 
git clone https://github.com/benjibrown/oxyl.git
cargo build 
./target/release/oxyl
```

## Installing from crates.io

```
cargo install oxyl 
oxyl <file.tex>
```

## Usage 

```sh
oxyl <file.tex>                 parse the file and report any errors 
oxyl --dump-tokens <file.tex>   print every lexer token with its byte span 
oxyl --dump-ast <file.tex>      print the parsed AST nodes 
oxyl --help                     full flag list 
```

Diagnostics include a 1-based line/column and an awesome caret pointing at the offending span: 

```
error [E022]: unclosed optional argument
  --> line 1:6
  |
1 | \sqrt[
  |      ^
```

## What's currently parsed
- Plain text and paragraphs (blank lines)
- Commands with optional and mandatory arguments: `\sqrt[3]{27}`
- Brace groups `{ ... }`
- Inline math `$ ... $`
- Display math `\[ ... \]`
- Line comments (preserved in the AST so source-fidelity tools can access them)

## Diagnostic Codes 

| Code | Where  | Meaning |
|------|--------|--------------------------------------|
| E010 | lexer  | lone backslash / non-ASCII character |
| E020 | parser | unclosed `{`                         |
| E021 | parser | unclosed mandatory argument          |
| E022 | parser | unclosed optional argument           |
| E030 | parser | unclosed `$` (inline math)           |
| E031 | parser | unclosed `\[` (display math)         |
| E032 | parser | stray `\]` (no matching `\[`)        |


