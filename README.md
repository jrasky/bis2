# bis2
Better search for bash, again

Uses [flx](https://github.com/jrasky/flx)

Use a keybind in bash to replace the readline search:
```bash
# add a keybind for bis
bind '"\C-r":"bis2\n"'
```

For a better integration, compile with the `no_ioctl` feature, and install a keybind like the following:
```bash
# function to interact with readline variables
function bis2_integration {
  { READLINE_LINE=$(</dev/tty bis2 2>&1 1>&$bis2out); } {bis2out}>&1
  READLINE_POINT=${#READLINE_LINE}
}

bind -x '"\C-r": "bis2_integration"'
```

You should also include the following, in order to be able to use CTRL-S:
```bash
# Disable flow control so we can use CTRL-S
stty -ixon
```