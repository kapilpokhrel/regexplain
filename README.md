# regexplain

A terminal UI for explaining and visualizing regular expressions, kindof like regex101.

![demo](regexplain.gif)

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

## Design
I structured it in such a way that it first parses raw and verbose AST (currently only from [regex_syntax](https://docs.rs/regex-syntax/latest/regex_syntax/))
into a simplified form and all the other modules, (description generator, colorizer) work on top of that simplified intermediate form.
I did this to so that we may easily add a regex flavours later (only rust-flavoured regex right now).

## Usage

You can download the binary from release or if you have cargo

```
cargo install --git https://github.com/kapilpokhrel/regexplain
```


### Controls

| Key | Action |
|---|---|
| `Esc` / `Ctrl+C` | Save state and quit |
| `Shift+Tab` | Cycle focus (Pattern → Text → Desc Tree) |
| Click | Focus the clicked panel |
| `Ctrl+Y` | Copy pattern to clipboard (when focused on pattern input) |
| `j/k/h/l` or Arrows | Navigate desctree (desctree focused) |
| `Ctrl+N` | Scroll viewport down (desctree focused) |
| `Ctrl+J` | Scroll viewport up (desctree focused) |


Also, both input and match editor window use `tui-textarea2`, so we get Emacs-style bindings (`C-f`/`C-b`, `C-n`/`C-p`, `M-f`/`M-b`, `C-a`/`C-e`, `C-d`/`C-h`, `C-k`, `M-<`/`M->`, etc.). 
The pattern input is single-line (Enter disabled).

I'm still getting the hang of Rust, so the code might not always be the most idiomatic. This project also went through a lot of iterations (and pauses) — you might smell some of that in places.
