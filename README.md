# regexplain

Interactively explore and explain regular expressions.

```
regexplain [OPTIONS]

  -p, --pattern <PATTERN>              regex pattern
  -t, --text-to-match <TEXT_TO_MATCH>  text to match against
  -f, --file-to-match <FILE_TO_MATCH>  read text to match from file
  -n, --no-tui                         run in CLI mode (no TUI)
  -r, --restore                        restore previous TUI session
  -h, --help                           print help
  -V, --version                        print version
```

## Building

```
cargo build --release
```

## Usage

```
regexplain -p '(\w+)@(\w+)\.(\w+)' -t 'hello@example.com'
regexplain -p '^TODO' -f todo.md
regexplain -p '\d{3}-\d{4}' -t '555-1234' --no-tui
regexplain -r
```

## TUI Controls

| Key | Action |
|---|---|
| `Esc` / `Ctrl+C` | Save state and quit |
| `Shift+Tab` | Cycle focus (Pattern → Text → Desc Tree) |
| Click | Focus the clicked panel |
| `Ctrl+Y` | Copy pattern to clipboard (when focused on pattern input) |

### Pattern Input & Text to Match

Both use `tui-textarea2` with Emacs-style bindings (`C-f`/`C-b`, `C-n`/`C-p`, `M-f`/`M-b`, `C-a`/`C-e`, `C-d`/`C-h`, `C-k`, `M-<`/`M->`, etc.). The pattern input is single-line (Enter disabled).

### Description Tree

| Key | Action |
|---|---|
| `j` / `Down` | Move selection down |
| `k` / `Up` | Move selection up |
| `h` / `Left` | Collapse node / move to parent |
| `l` / `Right` | Expand node |
| `Ctrl+N` | Scroll viewport down |
| `Ctrl+J` | Scroll viewport up |

Selecting a description node highlights the matching span in the pattern input and shows a breadcrumb in the text panel.

I'm still getting the hang of Rust, so the code might not always be the most idiomatic. This project also went through a lot of iterations (and pauses) — you might smell some of that in places.
