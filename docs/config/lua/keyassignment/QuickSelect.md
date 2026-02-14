# `QuickSelect`

{{since('20210502-130208-bff6815d')}}

Activates [Quick Select Mode](../../../quickselect.md).

```lua
local phaedra = require 'phaedra'

config.keys = {
  { key = ' ', mods = 'SHIFT|CTRL', action = phaedra.action.QuickSelect },
}
```

See also [QuickSelectArgs](QuickSelectArgs.md)
