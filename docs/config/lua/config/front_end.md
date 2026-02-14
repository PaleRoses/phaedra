---
tags:
  - gpu
---
# `front_end = "OpenGL"`

Specifies which render front-end to use.  This option used to have
more scope in earlier versions of phaedra, but today it allows two
possible values:

* `OpenGL` - use GPU accelerated rasterization
* `WebGpu` - use GPU accelerated rasterization {{since('20221119-145034-49b9839f', inline=True)}}

{{since('20240127-113634-bbcac864', outline=true)}}
    The default is `"WebGpu"`. In earlier versions it was `"OpenGL"`

{{since('20240128-202157-1e552d76', outline=true)}}
    The default has been reverted to `"OpenGL"`.

When running in a Remote Desktop environment on Windows, phaedra forces
a fallback adapter to avoid driver issues.

## WebGpu

{{since('20221119-145034-49b9839f')}}

The WebGpu front end allows phaedra to use GPU acceleration provided by
a number of platform-specific backends:

* Metal (on macOS)
* Vulkan
* DirectX 12 (on Windows)

See also:

* [webgpu_preferred_adapter](webgpu_preferred_adapter.md)
* [webgpu_power_preference](webgpu_power_preference.md)
* [webgpu_force_fallback_adapter](webgpu_force_fallback_adapter.md)
