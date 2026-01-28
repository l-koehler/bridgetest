# Brigetest

**This program is unstable and a mess. Please read this section before using it.**  
This program is supposed to let a unmodified [Luanti (Minetest)](https://www.luanti.org/) client connect to  
a unmodified Minecraft (Java Edition) server.  
The Minecraft server version needed is 1.21.11, use [ViaProxy](https://github.com/ViaVersion/ViaProxy) if you need another version.  
Compile it using `cargo build --release`, then run `--help` for usage info.  
Due to [luanti-rs](https://github.com/kawogi/luanti-rs), the supported/required Luanti version is 5.11.0.  
Other versions should work if the protocol didn't change, try it if you need these.  
Debug mode causes weird performance issues, don't use it.  

You need nightly Rust to build this and some dependencies (`rustup default nightly`).  
You should follow the instructions below to install and configure the proxy.  

## Installation Instructions

This program needs the minecraft textures.  
I won't bundle these due to copyright reasons, but you can get them:  

* From the Minecraft client:  
  * Get a minecraft jar file (should be something like `minecraft-1.21.11-client.jar`)  
  * Unpack it (jar files are glorified zip archives)  
  * Grab the folders in `assets/minecraft/textures/`  
* or from the internet:  
  * Visit mcasset.cloud [(you will need these files)](https://mcasset.cloud/1.21.11/assets/minecraft/textures)  
  * Click "Download Folder"  
  * You'll need to unpack that zip file, it contains your textures.  
* or from a unusually complete texture pack:  
  You likely won't find a texture pack that has all the needed files,  
  but if you do you could use that instead.  
  You might need to change `texture_pack_res` in the config file if you do this.  

After unpacking, you should have several directories full of .png files.  
Move them to create the following structure:  

```text
bridgetext-data-directory
└── textures
    ├── block
    ├── colormap
    └── ...
```

The `bridgetest-data-directory` is:

* `~/.local/share/bridgetest` on Linux  
* `C:\Users\YourName\AppData\Roaming\bridgetest` on Windows  

That folder is created automatically on first launch, but you can also create it manually.  

The rest of the required files will be downloaded automatically  
the first time you run the program.  

If you want to use a microsoft account:

* Enable online mode in the config file:
  Set `online_mode = true` in `~/.config/bridgetest.toml` (Linux) or  
  `C:\Users\YourName\AppData\Roaming\bridgetest.toml` (Windows).  
* Either add your E-Mail address to the config file (`microsoft_email = your.email@example.com`)  
  or set it on the command line with `--account your.email@example.com`.  
* On the first start, you'll be asked to visit `https://microsoft.com/link` and enter a Code  
  to allow "Minecraft for Nintendo Switch" (or similar) to authenticate using your account.  
  You should only need to do this once per account.  

The proxy will not be able to see your password or do anything more nefarious  
than logging into Minecraft servers on your behalf.  

If you don't use a microsoft account, the server you connect to has to be in offline mode.  

## Troubleshooting Steps

* Ensure you are running the latest commit for bridgetest.  
* Try using luanti compiled from somewhat recent source.  
* Open a issue here, most likely any problems are caused by bridgetest.  

## Things that are still missing from a usable version

* Crafting (Containers work sometimes, the UI is broken)  
* Rotated Blocks  
* Swimming  

## Other, smaller, broken things

* Climbable Blocks (ladders, vines etc) don't do anything  
* Particles aren't implemented  
* Jittery Movement: The client physics are slightly different, so  
  server/client will drift out of sync for up to 0.75 blocks,  
  at which point the proxy teleports the client (as smooth as it sounds).  
* Textures: The texture system is on its third rewrite and still doesn't do what  
  it should half the time (the half you rarely see, luckily).  

## Even more limitations (ones that don't affect gameplay)

* Any Anticheats are near-certain to ban you.  
  If they don't, you probably found a bug in the anticheat.  
  The traffic sent by this proxy is looking basically the  
  same as that from any (badly-made) bot.  

* The program *might* work on Windows, but I am not testing this. I doubt it.  

* The proxy can only handle one client at a time, but could probably be rewritten to handle more clients.  
  Just start several proxies with different listening ports (`--port`) for now.  

## Attributions

This program automatically downloads entity models.  
These were not made by me and are licensed under the [CC-BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/legalcode.en).  
The Models are taken from [Mineclonia](https://content.minetest.net/packages/ryvnf/mineclonia/), a minetest mod.  
Mineclonia is owned on ContentDB by [ryvnf](https://content.minetest.net/users/ryvnf/), a full list of contributors is [in their repository](https://codeberg.org/mineclonia/mineclonia/src/branch/main/CREDITS.md).  
