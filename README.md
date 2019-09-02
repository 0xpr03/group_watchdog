 ### Group Watchdog
 
 Ensure that all Files & Folders in a location contain a certain group and permissions. This can be helpful to allow a local shared folder between a group of users.
 
 Some file managers do not respect the file creation flag, preserving the original permissions when copying files into such a folder. Group Watchdog will override any file permissions and groups that are added or changed in their permissions at the given location.
 
 This tool is tested on Linux but should run on any unix system.
 
 #### Compiling
 
 Install rust, then run
 `cargo build --release`
 
 #### Running
 
 Add the watchdog as autostart entry.
 ```text
 USAGE:                                                                                                               
    group_watchdog [FLAGS] [OPTIONS] <file>                                                                          
                                                                                                                     
FLAGS:                                                                                                               
    -d, --debug      Activate debug mode                                                                             
    -h, --help       Prints help information                                                                         
    -V, --version    Prints version information                                                                      

OPTIONS:
    -g, --group-id <gid>        
    -n, --group-name <gname>    

ARGS:
    <file>
 ```

Note that you have to specify either `-g` or `-n`.
