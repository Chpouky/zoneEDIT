# zoneEDIT

**zoneEDIT** is a **desktop** app created with the sole purpose of making
**modding** more **accessible**.

![What is zoneEDIT](imgs/01-what-is-zoneedit.png)

## Features

- **Edit** `.cfg` values and export, while **following** the documented **ZoneKit** `{bpatch}` rules
- Create **presets** that you can go back to
- **Review** your changes with the **Diff Preview**

Every change is visible in the **Diff Preview**, grouped by item, with the
original value next to the new one before anything is written.

## Planned features

- **Adding** more **editable** values, such as **weapon** / **armor** stats
- **Removing** / **adding** / **replacing** items from the spawn logic
- **Automating** the ZoneKit **`.bat`** files execution

## Workflow

1. **Download** the **Lesser ZoneKit** on the **Epic Games Store**
2. Run the ZoneKit `CreatePlainMod.bat` followed by your **mod name**
3. **Launch** the **zoneEDIT** release you downloaded
4. **Select** your **STALKER2ZoneKit** folder
5. **Make** your value **changes**
6. **Export** to your newly created mod folder located in
   `STALKER2ZoneKit/Stalker2/Mods/`
7. Run the ZoneKit **`PakPlainMod.bat`** followed by your **mod name**
8. **Grab** your mod from the `STALKER2ZoneKit/Stalker2/SavedMods/Staged/` folder
9. **Install** your mod like usual in `~mods`

## Feature requests

I'd **love** to hear about your **ideas**! Don't hesitate to open an
[issue](https://github.com/Chpouky/zoneEDIT/issues) if you wish to contribute.

## Bug reports

If you find any **bug**, please **report** it on the
[GitHub issues](https://github.com/Chpouky/zoneEDIT/issues).

## Building from source

```sh
cargo run --release
```

## Disclaimer

- This tool was **entirely** vibe coded **locally** using Qwen3.8 27B
- This tool was **NOT** generated in an hour or on a whim without supervision.
  **A lot of design decisions and actual human intent was put into this tool.**
  AI was used to generate the **Rust** code I know nothing about
