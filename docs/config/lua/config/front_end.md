---
tags:
  - gpu
---
# `front_end = "WebGpu"`

Specifies which render front-end to use.

Possible values:

* `WebGpu` - use GPU accelerated rasterization {{since('20221119-145034-49b9839f', inline=True)}}

`front_end` defaults to `"WebGpu"`.

When running in a Remote Desktop environment on Windows, phaedra forces
a fallback adapter to avoid driver issues.

## WebGpu

{{since('20221119-145034-49b9839f')}}

The WebGpu front end allows phaedra to use GPU acceleration provided by
supported platform GPU APIs.

See also:

* [webgpu_preferred_adapter](webgpu_preferred_adapter.md)
* [webgpu_power_preference](webgpu_power_preference.md)
* [webgpu_force_fallback_adapter](webgpu_force_fallback_adapter.md)
