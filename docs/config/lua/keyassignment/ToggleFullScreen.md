# `ToggleFullScreen`

Toggles full screen mode for the current window.

```lua
local phaedra = require 'phaedra'

config.keys = {
  {
    key = 'n',
    mods = 'SHIFT|CTRL',
    action = phaedra.action.ToggleFullScreen,
  },
}
```

See also: [native_macos_fullscreen_mode](../config/native_macos_fullscreen_mode.md).

