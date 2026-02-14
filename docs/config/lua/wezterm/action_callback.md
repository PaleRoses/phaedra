---
title: phaedra.action_callback
tags:
 - keys
 - event
---

# `phaedra.action_callback(callback)`

{{since('20211204-082213-a66c61ee9')}}

This function is a helper to register a custom event and return an action triggering it.

It is helpful to write custom key bindings directly, without having to declare
the event and use it in a different place.

The implementation is essentially the same as:
```lua
function phaedra.action_callback(callback)
  local event_id = '...' -- the function generates a unique event id
  phaedra.on(event_id, callback)
  return phaedra.action.EmitEvent(event_id)
end
```

See [phaedra.on](./on.md) and [phaedra.action](./action.md) for more info on what you can do with these.


## Usage

```lua
local phaedra = require 'phaedra'

return {
  keys = {
    {
      mods = 'CTRL|SHIFT',
      key = 'i',
      action = phaedra.action_callback(function(win, pane)
        phaedra.log_info 'Hello from callback!'
        phaedra.log_info(
          'WindowID:',
          win:window_id(),
          'PaneID:',
          pane:pane_id()
        )
      end),
    },
  },
}
```
