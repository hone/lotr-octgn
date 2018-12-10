extern crate dirs;
extern crate docopt;
extern crate lotr_octgn;
#[macro_use]
extern crate serde_derive;

use std::io::Write;

use docopt::Docopt;

const APP_DIR: &str = ".lotr-octgn";
const USAGE: &str = "
LotR OCTGN

Usage:
  lotr-octgn pack [--set=<id>]
  lotr-octgn sets

Options:
  --set=<id>  OCTGN Set ID
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_set: Option<String>,
    cmd_pack: bool,
    cmd_sets: bool,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    let home_dir = dirs::home_dir().unwrap_or_else(|| {
        eprintln!("Couldn't find a home directory for caching");
        std::process::exit(10);
    });
    let app_dir = home_dir.join(APP_DIR);

    if args.cmd_pack {
        let git_dir = app_dir.join("git").join("lotr");
        lotr_octgn::fetch_or_update_octgn_git_dir(&git_dir).unwrap();

        let sets = lotr_octgn::sets(&git_dir).unwrap_or_else(|_| {
            eprintln!("Couldn't fetch Sets");
            std::process::exit(1);
        });

        let set = args
            .flag_set
            .map(|set_id| {
                sets.iter().find(|set| set.id == set_id).unwrap_or_else(|| {
                    eprintln!("Couldn't find that Set");
                    std::process::exit(2);
                })
            })
            .unwrap_or_else(|| {
                // if no set id provided, allow users to pick one from list of available
                for (index, set) in sets.iter().enumerate() {
                    println!("{}: {}", index, set.name);
                }
                print!("Input Set #: ");
                std::io::stdout().flush().unwrap();
                let mut buffer = String::new();
                std::io::stdin().read_line(&mut buffer).unwrap();

                let index = buffer.trim_right().parse::<usize>().unwrap_or_else(|_| {
                    eprintln!("Please specify a number: '{}'", buffer);
                    std::process::exit(6);
                });

                sets.get(index).unwrap_or_else(|| {
                    eprintln!("Couldn't find that Set");
                    std::process::exit(2);
                })
            });
        lotr_octgn::pack(&set).unwrap_or_else(|_| {
            std::process::exit(3);
        });
    } else if args.cmd_sets {
        let git_dir = app_dir.join("git").join("lotr");
        lotr_octgn::fetch_or_update_octgn_git_dir(&git_dir).unwrap();
        match lotr_octgn::sets(&git_dir) {
            Ok(sets) => {
                for set in sets {
                    println!("{}: {}", set.name, set.id);
                }
            }
            Err(_) => {
                eprintln!("Couldn't fetch Sets");
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("Invalid Command");
        println!("{}", USAGE);
        std::process::exit(4);
    }
}
