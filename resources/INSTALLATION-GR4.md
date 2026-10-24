# GRKit

For RICOH GR IV and GR IV HDF firmware 1.11 only.
One package serves each supported firmware version. At each power-on, GRKit finds the firmware of the camera.
The update to firmware 1.20 (1.10 on GR IV Monochrome) cannot be undone. After the update, the same package still works.
GR IV Monochrome runs the GR IV firmware and shows its own version: 1.00 for firmware 1.11, and 1.10 for firmware 1.20. Its package has no Image Control. GR IV Monochrome uses the GR IV runtimes, but has not been tested on a camera itself.

## Install

1. Turn the camera off.
2. Copy the contents of `SD_ROOT` to the root of the SD card.
3. Insert the card. Hold MENU while you turn the camera on. Wait until the camera turns itself off.
4. Turn the camera on normally.

Step 3 turns on the camera's SD scripts. The camera shows Mode Set Command, then Complete, and then turns itself off.

![The Mode Set Command screen shows Script - Enable and Complete.](images/modeset.png)

If the SD scripts are already on, skip step 3.
After the installation, you can remove `00078560.636` and `DEVELOP.MOD` from the card root. This step is optional.

## Update

1. Turn the camera off.
2. Copy the contents of `SD_ROOT` of the new package to the root of the SD card. Replace or merge the existing files.
3. Turn the camera on normally.

GRKit keeps your settings and effects in the `GRKIT` folder, so the update keeps them. You do not need to delete `SCRIPT` first. If you update from GRKit 1.2 or earlier and replace the `SCRIPT` folder, for example with the default Replace of macOS Finder, the earlier settings and the effects that the new package does not have are lost, because GRKit 1.2 kept them in `SCRIPT`.

## Use

Open ADJ and select the sixth entry, GRK. GRKit loads a few seconds after power-on, so the entry can appear late. When GRKit has loaded, the shooting screen shows **GRKit** at the top for a few seconds. This label needs the Framing or Guides module.

- **Image Control:** highlight an effect to preview it. Press OK to keep it. Press Fn to adjust the Standard parameters of the effect. On the GRK menu, press Fn on the Image Control row to adjust the selected effect. Each effect keeps its own adjustments, and they do not change the camera's Image Control settings. The effects were made for GR III, so their colors can differ on GR IV.
- **Framing:** select XPan, 2.35:1, 16:9, 4:3, 5:4, or 1:1. 16:9, 4:3, and 1:1 set the camera's own aspect ratio. XPan, 2.35:1, and 5:4 set the native aspect ratio to 3:2. The viewfinder then stays 3:2 and dims the area outside the photo. XPan and 2.35:1 also work when the native aspect ratio is 16:9. DNG files keep the full sensor image and record the crop. A raw converter that applies the recorded crop shows only the photo area.
- **Guides:** select a guide style. The lines show only in the viewfinder.
- **Date Imprint:** press OK to open its settings. In **Date Style**, select a date style. In **Date Look**, select the digits: Classic, GR1s, or GR II. Photos get the date at the bottom right.
- **Grain:** select Weak, Medium, or Strong. Only photos get the grain.

Highlight the last row, GRK, to see the GRKit version.

Press MENU to go back one level. GRKit keeps your settings after power-off.

GRKit keeps the effects in the `GRKIT/EFFECTS` folder of the card. A new package adds its effects and replaces their earlier versions. It does not remove other effects. To remove an effect, turn the camera off and delete its file from `GRKIT/EFFECTS`. A later package adds it again only when its file in the package changes. `GRKIT/EFFECTS.TXT` lists the file of each effect. GRKit installs at most 20 effects: first those of the last package, then the others.

## Troubleshooting

### The camera starts a firmware update when you hold MENU

The card root has a firmware update file. Turn the camera off. Remove each `.bin` file from the card root. Then do step 3 again.

### The camera does not show Mode Set Command

- Hold MENU, then turn the camera on. If you press MENU after the camera is on, the SD scripts stay off.
- Copy the contents of `SD_ROOT` to the card root, not into `DCIM` or another folder.

### GRK does not show in ADJ

1. Wait a few seconds after power-on. GRKit loads after the camera starts. When it has loaded, the shooting screen shows **GRKit** at the top for a few seconds.
2. Look for GRK in ADJ. It is not in MENU.
3. If the camera did not show Mode Set Command in step 3, do step 3 again.
4. Make sure that the card root has the `SCRIPT` folder. Copy all the contents of `SD_ROOT`, including `SCRIPT`.
5. If the card has `SCRIPT/PLUGINS/INSTALL.ERR`, GRKit did not install. Make sure that the camera shows a supported firmware version. To see the firmware version, open the camera menu. If the version is supported, send a [device report](https://github.com/ahonn/grkit/issues/new?template=device-report.yml) with all the lines of `INSTALL.ERR`.
6. GRKit runs only on the official Ricoh firmware. It does not run on modified firmware.

## Remove or recover

To remove GRKit, turn the camera off. Then delete the `SCRIPT` and `GRKIT` folders from the card. `GRKIT` holds your GRKit settings.

The SD script setting stays on. To turn it off:

1. On https://grkit.app, download the files that turn SD scripts off for your camera. The button is below the installation steps.
2. Copy the contents of `SD_ROOT` to the root of the SD card.
3. Insert the card. Hold MENU while you turn the camera on. Wait until the camera turns itself off.
4. Remove `00078560.636` and `DEVELOP.MOD` from the card root.

If the camera stops during startup, remove the battery and the card. Then start the camera without the card. If it still does not start, see the [recovery guide](https://github.com/ahonn/grkit/blob/main/resources/RECOVERY.md).

GRKit is not affiliated with Ricoh. Source code and licenses: https://github.com/ahonn/grkit
