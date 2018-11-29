extern crate docopt;
extern crate lotr_octgn;
#[macro_use]
extern crate serde_derive;

use docopt::Docopt;

const USAGE: &'static str = "
LotR OCTGN

Usage:
  lotr-octgn pack --set <id>
  lotr-octgn sets
";

#[derive(Debug, Deserialize)]
struct Args {
    arg_id: String,
    cmd_pack: bool,
    cmd_sets: bool,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    if args.cmd_pack {
        let sets = lotr_octgn::sets().unwrap_or_else(|_| {
            eprintln!("Couldn't fetch Sets");
            std::process::exit(1);
        });
        let set = sets
            .iter()
            .find(|set| set.id == args.arg_id)
            .unwrap_or_else(|| {
                eprintln!("Couldn't find that Set");
                std::process::exit(2);
            });
        lotr_octgn::pack(&set).unwrap_or_else(|_| {
            std::process::exit(3);
        });
    } else if args.cmd_sets {
        match lotr_octgn::sets() {
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
