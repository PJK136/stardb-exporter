# stardb-exporter with artifact exporter

This is a fork of `stardb-exporter` with a feature to export artifacts as easily as achievements.

<img width="802" height="628" alt="image" src="https://github.com/user-attachments/assets/38862900-a775-4729-a486-e3d41ebb4305" />


The feature is still in alpha testing, any comments are welcome!

You can download it here:
- [Windows](https://github.com/PJK136/stardb-exporter/releases/latest/download/stardb-exporter.exe)

It works the same way as the original `stardb-exporter`: run the game but don't go through the door, run the exporter, click on Artifact Exporter, go through the door, and wait for a button to copy the artifacts to your clipboard. It will be in the [GOOD](https://frzyc.github.io/genshin-optimizer/#/doc) format so it is ready to be imported anywhere else.

I haven't changed any other functionality including the sponsored section so I'm not related to anything else except the Artifact Exporter.

The protocol parser is also a fork I made to add the support for artifacts: [auto-artifactarium](https://github.com/PJK136/auto-artifactarium).

This is my first project in Rust, so please forgive me for any bad practices in the code. 😅

If you're interested, I can maintain it, add more features (e.g., exporting weapons, setting a minimum level for exporting, etc.) and/or make a pull request upstream.

The feature doesn’t specifically require `stardb-exporter` (but thanks to them, I was able to release this first version so easily!). If there’s demand for it, I can also try to make it standalone or integrate it into other tools.

You’re also free to reuse the code (at least the part I wrote) and improve it however you want, I'm glad to share the knowledge with other players. Just please credit me somewhere in your work.

Feel free to reach out to me here in this repository or on Discord with any questions.

Thanks to [@IceDynamix](https://github.com/IceDynamix), [@hashblen](https://github.com/hashblen), and [@juliuskreutz](https://github.com/juliuskreutz/stardb-exporter) for their publicly available work, which made this possible.

# Original Readme of stardb-exporter

> [!CAUTION]
> HSR Achievement Exporter is not working as of the `2025-08-13`!

## Instructions

This method will not work on any kind of VPN

- Download and install pcap

  - Windows: [Npcap Installer](https://npcap.com/#download) (Ensure `Install Npcap in WinPcap API-compatible mode` is ticked) \
    **Also ensure, that you're using the latest version of Npcap**
  - Linux: Figure it out, lol. The package should be called libpcap
  - Macos: Use brew https://formulae.brew.sh/formula/libpcap

- Note for WiFi users:

  - Windows: During Npcap installation, ensure `Support raw 802.11 traffic (and monitor mode) for wireless adapters` is ticked.
  - Linux and Macos: Make sure you enable monitor mode for your wireless adapter.

- Download the latest release:
  - [Windows](https://github.com/juliuskreutz/stardb-exporter/releases/latest/download/stardb-exporter.exe)
  - [Linux](https://github.com/juliuskreutz/stardb-exporter/releases/latest/download/stardb-exporter-linux)
  - [MacOs](https://github.com/juliuskreutz/stardb-exporter/releases/latest/download/stardb-exporter-macos)
- Launch the game to the point where.
  - HSR: The train is right before going into hyper speed
  - Genshin: Right before entering the door
- Execute the exporter (You might need to do this as admin/root) and wait for it to say `Device <i> ready~!`.
- Go into hyperspeed/Enter the door and it should copy the export to your clipboard.
- Paste it [here](https://stardb.gg/import).

## Building from source

For linux users, you need to set the `CAP_NET_RAW` capability

```sh
sudo setcap CAP_NET_RAW=+ep target/release/stardb-exporter
```

## Special thanks

Thank you [@IceDynamix](https://github.com/IceDynamix) for providing the building blocks for this with their [reliquary](https://github.com/IceDynamix/reliquary) project!

Thank you [@hashblen](https://github.com/hashblen) for creating protocol parsers that don't need any further updates ([auto-reliquary](https://github.com/hashblen/auto-reliquary) and [auto-artifactarium](https://github.com/hashblen/auto-artifactarium))!

Thank you [@emmachase](https://github.com/emmachase) for providing support wherever she can!
