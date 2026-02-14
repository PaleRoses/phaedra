---
title: phaedra.truncate_left
tags:
 - utility
 - string
---
# phaedra.truncate_left(string, max_width)

{{since('20210502-130208-bff6815d')}}

Returns a copy of `string` that is no longer than `max_width` columns
(as measured by [phaedra.column_width](column_width.md)).

Truncation occurs by removing excess characters from the left
end of the string.

For example, `phaedra.truncate_left("hello", 3)` returns `"llo"`.

See also: [phaedra.truncate_right](truncate_right.md), [phaedra.pad_right](pad_right.md).

