# CopyMode `MoveLeft`

{{since('20220624-141144-bd1b7c5d')}}

Moves the CopyMode cursor position one cell to the left.

```lua
local phaedra = require 'phaedra'
local act = phaedra.action

return {
  key_tables = {
    copy_mode = {
      { key = 'h', mods = 'NONE', action = act.CopyMode 'MoveLeft' },
      { key = 'LeftArrow', mods = 'NONE', action = act.CopyMode 'MoveLeft' },
    },
  },
}
```
