# FSLinkManager
#####  FSLinkManager is a simple linux CLI tool to save and manage filesystem links.

It allows the user to create. toggle and remove links between files and directories. It manages these links in a local json based database, stored in a hidden folder `.fslink/links`. It behaves somewhat like git in that matter, operating in a project-line manner.

The idea behind the project is to mimic how game mod-managers work, enabling the user to easily manage multiple mods at the same time. Eventually it could be a simple, universal tool to manage any program's plugins, additions or configs easily. 
## Features
- Create soft and hard links between files and directories
- Toggle links on and off
- Remove links
- List all links in the database
#### Extra possible features:
- Grouping links to be toggled together
- Managing links to remote storage - integration with rsync or such
- Emulating linking files directly out of an archive - by creating a staging folder for it's contents, then linking out of that folder. 
- Tracking changes to the link sources

