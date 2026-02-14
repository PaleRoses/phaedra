---
title: phaedra.log_error
tags:
 - utility
 - log
 - debug
---

# `phaedra.log_error(arg, ..)`

This function logs the provided message string through phaedra's logging layer
at 'ERROR' level, which can be displayed via [ShowDebugOverlay](../keyassignment/ShowDebugOverlay.md) action.  If you started phaedra from a terminal that text will print
to the stdout of that terminal.  If running as a daemon for the multiplexer
server then it will be logged to the daemon output path.

```lua
local phaedra = require 'phaedra'
phaedra.log_error 'Hello!'
```

{{since('20210814-124438-54e29167')}}

Now accepts multiple arguments, and those arguments can be of any type.

See also [log_info](log_info.md) and [log_warn](log_warn.md).
