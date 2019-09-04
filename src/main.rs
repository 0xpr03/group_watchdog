use crossbeam_channel::unbounded;
use nix::sys::stat::{fchmodat, FchmodatFlags, Mode};
use nix::unistd::{fchownat, FchownatFlags, Gid};
use notify::event::EventKind::*;
use notify::event::ModifyKind::*;
use notify::{Error, ErrorKind, RecommendedWatcher, RecursiveMode, Result, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;
use structopt::StructOpt;
use users::mock::gid_t;
use users::{get_group_by_gid, get_group_by_name};
use walkdir::WalkDir;

#[derive(StructOpt, Debug)]
struct GidArgs {
    #[structopt(long = "group-id", short = "g")]
    gid: Option<gid_t>,
    #[structopt(long = "group-name", short = "n")]
    gname: Option<String>,
}

#[derive(StructOpt, Debug)]
struct Opt {
    #[structopt(name = "FILE", parse(from_os_str))]
    files: PathBuf,
    #[structopt(flatten)]
    gid: GidArgs,
    #[structopt(short = "d", long = "debug", help = "Activate debug mode")]
    debug: bool,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let debug = opt.debug;

    let gid = if let Some(v) = opt.gid.gid {
        if get_group_by_gid(v).is_none() {
            let msg = format!("Invalid group ID provided! {}", v);
            eprintln!("{}", &msg);
            return Err(Error {
                kind: ErrorKind::Generic(msg),
                paths: Vec::new(),
            });
        }
        v
    } else if let Some(v) = opt.gid.gname {
        match get_group_by_name(&v) {
            Some(group) => group.gid(),
            None => {
                let msg = format!("Invalid group Name. {}", v);
                eprintln!("{}", msg);
                return Err(Error {
                    kind: ErrorKind::Generic(msg),
                    paths: Vec::new(),
                });
            }
        }
    } else {
        return Err(Error {
            kind: ErrorKind::Generic(format!("Please specify the group to set!")),
            paths: Vec::new(),
        });
    };

    let mut mode_file = Mode::empty();
    mode_file.insert(Mode::S_IRUSR);
    mode_file.insert(Mode::S_IWUSR);
    mode_file.insert(Mode::S_IRGRP);
    mode_file.insert(Mode::S_IWGRP);

    let mut mode_folder = mode_file.clone();
    mode_folder.insert(Mode::S_IXUSR);
    mode_folder.insert(Mode::S_IXGRP);

    if debug {
        println!("Parsed input!");
    }

    let (tx, rx) = unbounded();

    // Automatically select the best implementation for your platform.
    let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2))?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(opt.files.as_path(), RecursiveMode::Recursive)?;

    println!("Watching {:?} for activity...", opt.files);

    let mut set = HashSet::new();

    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                if debug {
                    println!("{:?}", event);
                }
                match event.kind {
                    Create(_) | Modify(Metadata(_)) => event.paths.into_iter().for_each(|p| {
                        if !set.remove(&p) {
                            if let Err(e) = update_path(
                                p.as_path(),
                                gid,
                                mode_file,
                                mode_folder,
                                &mut set,
                                debug,
                            ) {
                                eprintln!(
                                    "Error settings permissions for {:?}: {:?}",
                                    p.as_path(),
                                    e
                                );
                            }
                            set.insert(p);
                        } else {
                            if debug {
                                println!("Found in set, ignoring {:?}", &p);
                            }
                        }
                    }),
                    e => {
                        if debug {
                            println!("Ignoring {:?}", e)
                        }
                    }
                }
            }
            Ok(Err(e)) => eprintln!("Error receiving inotify event: {}", e),
            Err(err) => eprintln!("watch error: {:?}", err),
        };
    }
}

fn update_path(
    path: &Path,
    group: gid_t,
    mode_file: Mode,
    mode_folder: Mode,
    set: &mut HashSet<PathBuf>,
    debug: bool,
) -> nix::Result<()> {
    if debug {
        println!("Handling {:?}", path);
    }
    if path.is_dir() {
        for entry in WalkDir::new(path) {
            match entry {
                Ok(entry) => {
                    if debug {
                        println!("Updating {:?}", entry.path());
                    }
                    // insert first, otherwise double trigger for partial perm update failure
                    set.insert(entry.clone().into_path());
                    if let Err(e) =
                        set_permissions(entry.path(), group, mode_file, mode_folder, debug)
                    {
                        eprintln!("Error, can't update {:?}: {}", entry.path(), e);
                    }
                }
                Err(e) => eprintln!("Can't access {}", e),
            }
        }
    } else {
        set_permissions(path, group, mode_file, mode_folder, debug)?;
    }

    Ok(())
}

fn set_permissions(
    path: &Path,
    group: gid_t,
    mode_file: Mode,
    mode_folder: Mode,
    debug: bool,
) -> nix::Result<()> {
    fchownat(
        None,
        path,
        None,
        Some(Gid::from_raw(group)),
        FchownatFlags::NoFollowSymlink,
    )?;
    if debug {
        println!("Group set.");
    }
    if path.is_dir() {
        if debug {
            println!("Folder");
        }
        fchmodat(None, path, mode_folder, FchmodatFlags::FollowSymlink)?;
    } else {
        if let Ok(metadata) = path.metadata() {
            if metadata.file_type().is_symlink() {
                if debug {
                    println!("Ignoring symlink.");
                }
                return Ok(());
            }
        } else {
            eprintln!("No metadata, ignoring..");
            return Ok(());
        }
        if debug {
            println!("No folder");
        }
        fchmodat(None, path, mode_file, FchmodatFlags::FollowSymlink)?;
    }
    Ok(())
}
