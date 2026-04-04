# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**im-select-ssh.nvim** automatically switches the Input Method Editor (IME) for Neovim running on a remote SSH host. When leaving insert mode, the current IME is saved and switched to English (US). When entering insert mode, the saved IME is restored.

### Three Components

1. **Server-side Rust CLI tool** — runs on the remote host, communicates with the client over an SSH tunnel using MsgPack serialization
2. **Server-side Neovim plugin (Lua)** — sets up the SSH tunnel at launch, hooks Neovim's `InsertLeave`/`InsertEnter` autocommands to invoke the Rust tool
3. **Client-side C# tool** — runs on the local Windows machine, receives commands from the Rust tool, saves/restores IME state using `im-select.exe` (e.g., language code `1033` = en-US)

### Communication Flow

```
Neovim (Lua shim) → autocommand fires → Rust CLI → SSH tunnel → C# client → im-select.exe
```

- **InsertLeave**: C# client saves current IME, then sets IME to en-US (1033)
- **InsertEnter**: C# client restores the previously saved IME

## Build Commands

- **Rust**: `cargo build` / `cargo build --release` / `cargo test`
- **C#**: `dotnet build` / `dotnet build -c Release` / `dotnet test`
- **Lua**: No build step; installed as a Neovim plugin (e.g., via lazy.nvim or similar)

## Commit Message Conventions

Follow `.vscode/commit.instructions.md`:
- Headline: capitalized verb, <10 words, no trailing period
- Body: bullet list, important items first
- Use backtick-wrapped `code_item` and `path/filename` references
- Distinguish moves/renames from add/delete pairs
- Avoid vague words like "refactor", "enhance", "improve" unless nothing else fits

## Conventions

- Prefix experimental/scratch files with `x_` or put them in `tmp/` — both are gitignored
- The `.claude/skills/` directory is a git submodule (`claude-skills`)
