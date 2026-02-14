---
title: phaedra.config_dir
tags:
 - filesystem
---

# `phaedra.config_dir`

This constant is set to the path to the directory in which your `phaedra.lua`
configuration file was found.

```lua
local phaedra = require 'phaedra'
phaedra.log_error('Config Dir ' .. phaedra.config_dir)
```


