---
title: phaedra.config_file
tags:
 - filesystem
---

# `phaedra.config_file`

{{since('20210502-130208-bff6815d')}}

This constant is set to the path to the `phaedra.lua` that is in use.

```lua
local phaedra = require 'phaedra'
phaedra.log_info('Config file ' .. phaedra.config_file)
```



