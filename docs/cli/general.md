# Command Line

This section documents the phaedra command line.

*Note that `phaedra --help` or `phaedra SUBCOMMAND --help` will show the precise
set of options that are applicable to your installed version of phaedra.*

phaedra is deployed with two major executables:

* `phaedra` (or `phaedra.exe` on Windows) - for interacting with phaedra from the terminal
* `phaedra-gui` (or `phaedra-gui.exe` on Windows) - for spawning phaedra from a desktop environment

You will typically use `phaedra` when scripting phaedra; it knows when to
delegate to `phaedra-gui` under the covers.

If you are setting up a launcher for phaedra to run in the Windows GUI
environment then you will want to explicitly target `phaedra-gui` so that
Windows itself doesn't pop up a console host for its logging output.

!!! note
    `phaedra-gui.exe --help` will not output anything to a console when
    run on Windows systems, because it runs in the Windows GUI subsystem and has no
    connection to the console.  You can use `phaedra.exe --help` to see information
    about the various commands; it will delegate to `phaedra-gui.exe` when
    appropriate.

## Synopsis

```console
{% include "../examples/cmd-synopsis-phaedra--help.txt" %}
```
