# THIS IS VERY INCOMPLETE!

#### What is this?

This program is supposed to let a unmodified [Minetest](https://github.com/minetest/minetest) client connect to  
another (mostly) unmodified Minecraft (Java Edition) server.  
It compiles to a standalone executable, which will listen on 127.0.0.1:30000  
for minetest and then proxy to a minecraft server specified in CONF_DIR/config.txt  

#### Things that should be added:

still nearly everything, but for now:  

* Config file parsing  

* Sending a texture pack (MT S->C)  

* Sending default/empty values for all required packets  

#### Limitations:

Note:
*Currently*, nearly everything is a limitation, but  
most things are planned to be added. This only lists problems that will  
likely remain even if this ever becomes somewhat complete.  
Technical Limitations:  

* The Minecraft server needs to be in offline-mode. I could fix that  
  with `azalea-auth` but most people who might use this probably do not  
  have a minecraft account.  

* Any Anticheats are ~~likely~~ near-certain to ban you.  
  (if they don't, you probably found a bug in the anticheat? the traffic sent  
  by this proxy is looking basically the same as that from any bot.)  
  That is a slight danger even with GeyserMC in proxy mode, a similar  
  (but mature) program basically doing the same thing for Bedrock.  

* The program cannot run as a server- or clientside mod.  
  Server-side would probably be possible *somehow*, but  
  there are only few protocol librarys for minetest, none of them for Java.  

  A Client-side mod is simply impossible, as the Minetest Modding API (Lua)  
  does not allow me to rip the entire engine networking out and replace it.  

* The program *might* work on Windows, but I am not testing this.  
  If you find a windows-bug, feel free to open a issue, but I will only work  
  on that if it won't take too long. PRs fixing windows will be accepted.  
  for now, i'd prefer getting this mess to work at all :3  

* The upstream library for the minecraft protocol needs to be the latest git version,
  for that purpose just clone [[TODO explain this!]]
  
#### Isn't this violating Microsofts IP?

The protocol is implemented by another library, not by me.
The textures sent to the client are not made or endorsed by microsoft,  
It sends the [Faithful x32](https://faithfulpack.net) texture pack by default,  
but you can replace this pack if you like, it is pulled from CONF_DIR/textures.  
CONF_DIR can differ from system to system, the path will be shown on each start.  
