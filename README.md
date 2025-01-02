# THIS IS VERY INCOMPLETE!

#### What is this?

This program is supposed to let a unmodified [Minetest](https://github.com/minetest/minetest) client connect to  
another (mostly) unmodified Minecraft (Java Edition) server.  
The Java Server version needed is 1.21.4, use [ViaProxy](https://github.com/ViaVersion/ViaProxy) if you need another version.  
It compiles to a standalone executable, which will listen on 127.0.0.1:30000  
for minetest and then proxy to a minecraft server specified in CONF_DIR/config.txt  

You need nightly rust to build some dependencies (`rustup default nightly`).  

#### Things that are still missing from a usable version:

* Crafting (Containers work (mostly, the UI is broken))  
* Attacking/usable combat in general  

#### Other, smaller, broken things:

* Rotated Blocks (ex. ladders that have a "side")  
* Climbable Blocks (ladders/vines)  
* Sneaking (waiting on upstream)  
* Swimming  
* Various block interactions like opening doors, using levers etc.  
* Particles (will suck to implement, delayed until i cant do other stuff instead)  
* Imprecisions in the movement (the client speed/gravity etc is not exact, so  
  server/client will drift out of sync for up to half a block, at which point the  
  proxy re-positions the client)

#### Even more limitations (ones that don't affect gameplay):

* The Minecraft server needs to be in offline-mode. I could fix that  
  with `azalea-auth` but most people who might use this probably do not  
  have a minecraft account. TODO later  

* Any Anticheats are ~~likely~~ near-certain to ban you.  
  (if they don't, you probably found a bug in the anticheat? the traffic sent  
  by this proxy is looking basically the same as that from any bot.)  
  That is a slight danger even with GeyserMC in proxy mode, a similar  
  (but mature) program basically doing the same thing for Bedrock.  

* The program *might* work on Windows, but I am not testing this.  
  If you find a windows bug, feel free to open a issue, but I will only work  
  on that if it won't take too long. PRs fixing windows will be accepted.  
  for now, i'd prefer getting this mess to work at all :3  

* The upstream library for the minecraft protocol  
  needs to be the bleeding-edge git version, but you can simply ignore  
  this warning here if you only want to *use* this program.  

* The proxy can only handle one client at a time, but could probably be  
  rewritten to handle more clients without changing that much.  

#### Attributions

This repository contains entity models (the .b3d files).  
These were not made by me and are licensed under the [CC-BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/legalcode.en).  
The MPL2 License does __NOT__ apply to anything in the `models` directory!  
The Models are taken from [Mineclonia](https://content.minetest.net/packages/ryvnf/mineclonia/), a minetest mod.  
This Mod is owned on ContentDB by [ryvnf](https://content.minetest.net/users/ryvnf/), a full list of contributors is [here](https://codeberg.org/mineclonia/mineclonia/src/branch/main/CREDITS.md).  

#### Isn't this violating Microsofts Intellectual Property?

The minecraft protocol is implemented by another library, not by me.  

The textures this server is sending are NOT the official minecraft resources.  
This repository contains NO textures, but the program will offer to download  
the [Faithful x32](https://faithfulpack.net/) texture pack ([license](https://faithfulpack.net/license)) if no pack is found.  
You can change what pack is used by changing the URL the config file  
(at `~/.config/bridgetest.toml`) points to or by changing the texture pack  
itself, at `~/.local/share/bridgetest/textures`).  
(these paths are dirs::local_data_dir and dirs::config_dir, not hardcoded)  
The 3D-models are also not provided by this program.  
