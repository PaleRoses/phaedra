---
title: phaedra.emit
tags:
 - event
---

# `phaedra.emit(event_name, args...)`

{{since('20201031-154415-9614e117')}}

`phaedra.emit` resolves the registered callback(s) for the specified
event name and calls each of them in turn, passing the additional
arguments through to the callback.

If a callback returns `false` then it prevents later callbacks from
being called for this particular call to `phaedra.emit`, and `phaedra.emit`
will return `false` to indicate that no additional/default processing
should take place.

If none of the callbacks returned `false` then `phaedra.emit` will
itself return `true` to indicate that default processing should take
place.

This function has no special knowledge of which events are defined by
phaedra, or what their required arguments might be.

See [phaedra.on](on.md) for more information about event handling.

