# howto

Convert natural language to shell commands.

```bash
howto list all docker containers
# Command: docker ps -a
# Explain: List all Docker containers (running and stopped)
# Risk:    low
```

You stay in control — commands are inserted into your terminal prompt, never auto-executed.

## Install

### From source

```bash
git clone https://github.com/youruser/howto.git
cd howto
cargo install --path .
```

### From crates.io (once published)

```bash
cargo install howto
```

The binary is placed in `~/.cargo/bin/`. Make sure it's in your `$PATH`.

## Requirements

One of:

- **OpenAI API key** — set `OPENAI_API_KEY`
- **Ollama** running locally — default at `http://127.0.0.1:11434`

Auto-detection tries OpenAI first, then Ollama.

## Usage

```bash
howto <natural language query>
```

### Examples

```bash
howto find files larger than 100MB
howto all processes listening on port 8080
howto compress this directory as tar.gz
howto show git log as oneline graph
```

### Options

```bash
howto --engine <ENGINE> <query>  # LLM engine: auto (default), openai, ollama
howto --print-cmd <query>        # print only the command (no UI)
howto --print-json <query>       # print raw JSON from the LLM
howto --force <query>            # allow high-risk and blocked commands
```

### Interactive mode (default)

Shows command, explanation, and risk level. You choose what to do:

```
  Command: docker ps -a
  Explain: List all Docker containers (running and stopped)
  Risk:    low

  [Enter] insert   [e] edit   [Esc] cancel
```

- **Enter** — accept the command
- **e** — edit the command before accepting
- **Esc** / **q** — cancel

Medium-risk commands require typing `EXECUTE` to confirm. High-risk commands are blocked unless `--force` is passed.

### Replace mode

Set `HOWTO_MODE=replace` to skip the interactive UI and print the command directly. Designed for shell widget integration where the command replaces your current terminal line.

## Shell integration

Add a keybinding (`Ctrl+G`) that triggers howto from your terminal:

```bash
howto install
```

This adds a small block to your `~/.zshrc` or `~/.bashrc`. Restart your shell or run:

```bash
source ~/.zshrc   # or ~/.bashrc
```

Now press `Ctrl+G` to:

- **Interactive mode** — opens howto, inserts the result into your prompt
- **Replace mode** (`HOWTO_MODE=replace`) — sends whatever you've typed as the query, replaces it with the generated command

To remove:

```bash
howto uninstall
```

To print the shell init script without modifying rc files (useful for `eval` in custom setups):

```bash
howto init zsh   # or bash
```

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `HOWTO_MODE` | `interactive` | UX mode: `interactive` or `replace` |
| `HOWTO_ENGINE` | `auto` | LLM engine: `auto`, `openai`, `ollama` |
| `HOWTO_MODEL` | engine default | Override model name |
| `OPENAI_API_KEY` | — | OpenAI API key |
| `OLLAMA_HOST` | `http://127.0.0.1:11434` | Ollama server URL |
| `HOWTO_ALLOW_HIGH` | `0` | Set to `1` to allow high-risk and blocked commands (same as `--force`) |

## Safety

howto classifies every generated command independently of the LLM:

- **low** — read-only commands (ls, ps, grep, docker ps)
- **medium** — state changes (service restart, package install) — requires `EXECUTE` confirmation
- **high** — destructive commands (rm, prune, sudo) — blocked unless `--force`
- **BLOCKED** — hard-blocked patterns (rm -rf /, mkfs, curl|sh, fork bombs) — blocked unless `--force`

## License

MIT
