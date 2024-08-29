# vbsp-to-gtlf

Convert Valve BSP files to GLTF files

## Usage

```bash
vbsp-to-gltf input.bsp output.glb
```

Note that this requires TF2 to be installed to get the texture and props referenced in the map.

It should be able to automatically detect the tf2 path or you can overwrite it by setting the `TF_DIR` environment
variable.

## Model optimization

The output for the converter isn't particularly optimized, it's strongly recommended to run the output
through [gltfpack](https://github.com/zeux/meshoptimizer) before usage.

![screenshot of koth_bagel model](readme/bagel.webp)

`koth_bagel` as viewed with the [PlayCanvas model viewer](https://playcanvas.com/viewer).