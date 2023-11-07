# Float Crush

## Concept
Using a modified binary-search algorithm, this plugin emulates floating point bitcrushing at any arbitrary floating point bit-depth (including bit depths inbetween whole numbers!).

## Parameters

- Input Gain
...Increase or decrease the gain going into the plugin. All audio is clipped at 0dbfs
- Round
...Control whether the quantization rounds up, down, or to the nearest point. Default is nearest.
- Exponent
...The bit depth for the exponent part of the floating point. In pratice, higher values allows you to preserve audio quality as the signal gets quieter.
- Exponent Base
...In real floating point numbers, the exponent base is 2. However, this is an emulation so we can emulate floating points with exponents bases other than 2!
- Mantissa
...The bit depth for the linear part of the floating point. This is like every other bitcrusher, except the range it crushes is nested under each exponent domain.
- Mantissa Bias
...I put this one in for fun. Higher values group the quantization points louder, lower values group them quieter, the middle value spreads the points equally.
- Dry Gain and Wet Gain
...Control how much dry and wet signal is output from the plugin

## Building

After installing [Rust](https://rustup.rs/), you can compile Float Crush as follows:

```shell
cargo xtask bundle float_crush --release
```
