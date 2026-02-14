---
title: phaedra.truncate_right
tags:
 - utility
 - string
---
# phaedra.truncate_right(string, max_width)

{{since('20210502-130208-bff6815d')}}

Returns a copy of `string` that is no longer than `max_width` columns
(as measured by [phaedra.column_width](column_width.md)).

Truncation occurs by reemoving excess characters from the right end
of the string.

For example, `phaedra.truncate_right("hello", 3)` returns `"hel"`,

See also: [phaedra.truncate_left](truncate_left.md), [phaedra.pad_left](pad_left.md).
