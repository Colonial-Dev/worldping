<h1 align="center">Worldping</h1>
<h3 align="center">A command-line tool for mass IPv4 pinging.</h3>

<p align="center">
<img src="https://img.shields.io/github/license/Colonial-Dev/worldping">
<img src="https://img.shields.io/github/stars/Colonial-Dev/worldping">
</p>

Satpaper generates live wallpapers for your desktop, using near-real-time imagery from [RAMMB SLIDER](https://rammb-slider.cira.colostate.edu).

There are several satellites to choose from, each covering a different region of the world.
- GOES East (used in the sample image - covers most of North and South America)
- GOES West (Pacific Ocean and parts of the western US)
- Himawari (Oceania and East Asia)
- Meteosat 9 (Africa, Middle East, India, Central Asia)
- Meteosat 10 (Atlantic Ocean, Africa, Europe)

It's also possible to specify a custom background image, if desired.

## Installation

Dependencies:
- The [Rust programming language](https://rustup.rs/).
- A C/C++ toolchain (such as `gcc`.)

Just use `cargo install`, and Worldping will be compiled and added to your `PATH`.
```sh
cargo install --locked --git https://github.com/Colonial-Dev/worldping --branch master
```

## Command Line Options
- `-s`/`--satellite`/`SATPAPER_SATELLITE` - the satellite to source imagery from. 
    - Possible values: `goes-east`, `goes-west`, `himawari`, `meteosat9`, and `meteosat10`.
- `-x`/`--resolution-x`/`SATPAPER_RESOLUTION_X` (and equivalents for the `y` dimension) - the width/height of the generated wallpaper.
    - Any arbitary resolution should work, including vertical aspect ratios.
- `-d`/`--disk-size`/`SATPAPER_DISK_SIZE` - the size of the "disk" (Earth) relative to the generated wallpaper's smaller dimension.
    - Required to be an integer value in the range `[1, 100]` inclusive, mapping to a percentage value.
    - For most desktop environments, a value in the 90-95 range will give the most detail while preventing parts from being cut off by UI elements like taskbars.
- `-t`/`--target-path`/`SATPAPER_TARGET_PATH` - where the generated wallpaper should be saved.
    - Satpaper will output to a file called "satpaper_latest.png" at this path.
    - Example: if the argument is `/home/user/Pictures`, the output will be at `/home/user/Pictures/satpaper_latest.png`.

## FAQ

### *Why is Satpaper using hundreds of megs of RAM?*

There are two possible causes:
- You're seeing RAM usage spike to 500+ megabytes whenever Satpaper is compositing a new wallpaper. This is expected and unavoidable - the raw satellite imagery alone is ~450 megabytes after being decompressed and stitched together. However, this spike should only last several seconds - once composition is complete, the image buffers are all freed, and `libmimalloc_sys::mi_collect` is called to ensure as much memory as possible is returned to the OS.
- You're using an early version of Satpaper. Early versions had issues with `libc`'s `free` deciding it was fine to just... not return multi-hundred-megabyte allocations to the OS, as well as the `tokio` runtime being fairly memory heavy. I resolved these issues by switching to `mimalloc` and transitioning away from async, so behavior *should* improve if you update.

### *Why are continents purple in night imagery?* / *Why does night imagery look kinda weird?*
This is a byproduct of the CIRA GeoColor processing algorithm used to generate full-color images from the raw satellite data. GeoColor uses infrared for night-time imaging, which is then overlaid with false city lights and whitened clouds. The resulting image usually looks pretty good at a glance, but might begin to seem unnatural upon closer inspection.

Unfortunately, this is a necessary evil, as geostationary weather satellites don't capture enough visible spectrum light to generate a true-color night-time image.

### *I live at `$EXTREME_LATITUDE` - is there a way to get better imagery of my location?*
Not really. Geostationary orbits (required for the type of imaging we want) can only be achieved at a very specific altitude directly above the equator.

### *Why am I seeing glitchy imagery from GOES East at night?*
You're most likely seeing something like this:
<p align="center">
<img src=".github/goes_east_glitch.png">
<p>

This is not a software error, but is instead lens flare from the Sun peeking over from the other side of the Earth. This is caused by the Earth's tilt, and is most visible in late February and August.

You can find a more detailed explanation [here](https://www.reddit.com/r/WeatherGifs/comments/pj25ht/comment/hbvs1wo).
