# THIS IS VERY INCOMPLETE!

#### What is this?

This program is supposed to let a unmodified [Minetest](https://github.com/minetest/minetest) client connect to  
another (mostly) unmodified Minecraft (Java Edition) server.  
The Java Server version is whatever the latest git commit to [azalea](https://github.com/azalea-rs/azalea/commits/main/) needs.  
It compiles to a standalone executable, which will listen on 127.0.0.1:30000  
for minetest and then proxy to a minecraft server specified in CONF_DIR/config.txt  

You need nightly rust to build some dependencies (`rustup default nightly`).  

As a minetest client assumes nearly everything is provided by the server  
while the server assumes textures, blocks and items are known by the client,  
this program does NOT ONLY proxy all traffic, but also sends a texture  
pack and block/item/entity definitions, which are obtained from [ArcticData](https://github.com/Articdive/ArticData).  
You can generate these definitions yourself if you don't want to use ArcticData,  
possibly by using [Minecrafts inbuilt Data Generators](https://wiki.vg/Data_Generators) instead of it.

This also needs the entity models as .b3d files. You can get these by  
downloading Mineclonia, taking all .b3d files that are in that folder  
and pasting them into `~/.local/share/bridgetest/models/` (on linux).  

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
