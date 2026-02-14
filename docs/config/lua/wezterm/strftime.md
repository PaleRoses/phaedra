---
title: phaedra.strftime
tags:
 - utility
 - time
 - string
---
# `phaedra.strftime(format)`

{{since('20210314-114017-04b7cedd')}}

Formats the current local date/time into a string using [the Rust chrono
strftime syntax](https://docs.rs/chrono/0.4.19/chrono/format/strftime/index.html).

```lua
local phaedra = require 'phaedra'

local date_and_time = phaedra.strftime '%Y-%m-%d %H:%M:%S'
phaedra.log_info(date_and_time)
```

See also [strftime_utc](strftime_utc.md) and [phaedra.time](../phaedra.time/index.md).

