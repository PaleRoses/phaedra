---
title: phaedra.version
tags:
 - utility
 - version
---
# `phaedra.version`

This constant is set to the `phaedra` version string that is also reported
by running `phaedra -V`.  This can potentially be used to adjust configuration
according to the installed version.

The version string looks like `20200406-151651-5b700e4`.  You can compare the
strings lexicographically if you wish to test whether a given version is newer
than another; the first component is the date on which the release was made,
the second component is the time and the final component is a git hash.

```lua
local phaedra = require 'phaedra'
phaedra.log_error('Version ' .. phaedra.version)
```


