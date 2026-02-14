---
title: phaedra.has_action
tags:
 - utility
 - version
---

# phaedra.has_action(NAME)

{{since('20230408-112425-69ae8472')}}

Returns true if the string *NAME* is a valid key assignment action variant
that can be used with [phaedra.action](action.md).

This is useful when you want to use a phaedra configuration across multiple
different versions of phaedra.

```lua
if phaedra.has_action 'PromptInputLine' then
  table.insert(config.keys, {
    key = 'p',
    mods = 'LEADER',
    action = phaedra.action.PromptInputLine {
      -- other parameters here
    },
  })
end
```
